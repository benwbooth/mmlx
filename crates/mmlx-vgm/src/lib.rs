//! `mmlx-vgm`: VGM analysis and decompilation to mmlx source.
//!
//! [`parse_header`] reads header + loop points. [`parse_commands`] expands
//! the command stream with sample positions. [`track_song`] runs the YM2612,
//! PSG, and DAC trackers into timestamped [`TrackNote`]s. [`emit_song`]
//! renders compressed, readable mmlx source: pitch/duration literals,
//! `repeat!` runs, per-section voice setups, and intro/loop structure.
//!
//! Exactness contract: sample timing is exact (128th-note tick grid chosen
//! so the song's tick divides evenly); timbre is exact via raw `ym_*` /
//! `op*_tl`-style params; approximated only for SR slides and SSG-EG shapes
//! (flagged per note in `TrackNote::approx`).

use std::collections::HashMap;

// --- header ---

pub struct VgmHeader {
    pub version: u32,
    pub total_samples: u32,
    pub loop_offset: Option<usize>,
    pub loop_samples: u32,
    pub ym2612_clock: u32,
    pub data_start: usize,
}

pub fn parse_header(data: &[u8]) -> Result<VgmHeader, String> {
    if data.len() < 0x40 || &data[0..4] != b"Vgm " {
        return Err("not a VGM file".to_string());
    }
    let u32le = |at: usize| {
        u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize
    };
    let u32v = |at: usize| u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
    let data_start = if u32v(0x34) == 0 { 0x40 } else { 0x34 + u32le(0x34) };
    let loop_offset = if u32v(0x1C) == 0 { None } else { Some(0x1C + u32le(0x1C)) };
    Ok(VgmHeader {
        version: u32v(0x08),
        total_samples: u32v(0x18),
        loop_offset,
        loop_samples: u32v(0x20),
        ym2612_clock: u32v(0x2C),
        data_start,
    })
}

// --- command stream ---

#[derive(Clone, Debug)]
pub enum Command {
    YmWrite { port: u8, addr: u8, val: u8 },
    PsgWrite(u8),
    DacTick,
    End,
}

/// `(command_offset, sample_pos, Command)` in file order.
pub fn parse_commands(data: &[u8], header: &VgmHeader) -> Result<Vec<(usize, u64, Command)>, String> {
    let mut out = Vec::new();
    let mut sample = 0u64;
    let mut i = header.data_start;
    let n = data.len();
    while i < n {
        let offset = i;
        let cmd = data[i];
        i += 1;
        match cmd {
            0x52 | 0x53 => {
                if i + 2 > n {
                    return Err("truncated YM write".to_string());
                }
                out.push((
                    offset,
                    sample,
                    Command::YmWrite {
                        port: cmd - 0x52,
                        addr: data[i],
                        val: data[i + 1],
                    },
                ));
                i += 2;
            }
            0x50 => {
                if i >= n {
                    return Err("truncated PSG write".to_string());
                }
                out.push((offset, sample, Command::PsgWrite(data[i])));
                i += 1;
            }
            0x4F => {
                i += 1; // GG stereo: musically irrelevant
            }
            0x61 => {
                if i + 2 > n {
                    return Err("truncated wait".to_string());
                }
                sample += u16::from_le_bytes([data[i], data[i + 1]]) as u64;
                i += 2;
            }
            0x62 => sample += 735,
            0x63 => sample += 882,
            0x70..=0x7F => sample += (cmd - 0x70 + 1) as u64,
            0x80..=0x8F => {
                out.push((offset, sample, Command::DacTick));
                sample += (cmd - 0x80) as u64;
            }
            0x66 => {
                out.push((offset, sample, Command::End));
                break;
            }
            0x67 => {
                if i + 6 > n {
                    return Err("truncated data block".to_string());
                }
                let size =
                    u32::from_le_bytes([data[i + 2], data[i + 3], data[i + 4], data[i + 5]]) as usize;
                i += 6 + size;
            }
            0xE0 => {
                i += 4; // PCM bank seek
            }
            _ => return Err(format!("unsupported command {cmd:#04x} at {offset:#x}")),
        }
    }
    Ok(out)
}

// --- tracked notes ---

/// One note with full voice-program snapshot, in sample ticks (44100 Hz).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackNote {
    pub start: u64,
    pub duration: u64,
    /// Voice index: 0-5 FM, 10-12 PSG tone, 13 PSG noise, 20 DAC drums.
    pub voice: u8,
    pub midi: u8,
    /// 0-1 loudness (FM always 1.0 — TL params carry it; PSG from nibble).
    pub velocity: f32,
    /// Raw voice program (ym_*/op*_tl for FM, sn_* for PSG).
    pub params: Vec<(String, f32)>,
    /// True when an approximation was used (SR slide, SSG-EG shape, ...).
    pub approx: bool,
}

/// One operator row in FILE order (S1/S3/S2/S4 per Plutiedev).
#[derive(Clone, Copy, Default)]
struct FmRow {
    mult: u8,
    tl: u8,
    ar: u8,
    dr: u8,
    sr: u8,
    sl: u8,
    rr: u8,
    ssg: u8,
}

#[derive(Clone, Default)]
struct FmChannel {
    rows: [FmRow; 4],
    algo: u8,
    feedback: u8,
    pan: u8,
    fnum: u16,
    block: u8,
    active: Option<(u64, u8, Vec<(String, f32)>, bool)>,
}

/// FNUM/block to frequency (core-calibrated relation).
fn fnum_freq(fnum: u16, block: u8, clock: u32) -> f32 {
    fnum as f32 * (clock as f32 / 144.0) / 2.0f32.powi(21 - block as i32)
}

fn freq_midi(freq: f32) -> u8 {
    (69.0 + 12.0 * (freq / 440.0).log2()).round().clamp(0.0, 127.0) as u8
}

/// Snapshot a channel into raw voice params. Slots S1..S4; file rows hold
/// S1/S3/S2/S4, so our op(i+1) reads file row [0,2,1,3][i].
fn fm_snapshot(channel: &FmChannel) -> (Vec<(String, f32)>, bool) {
    let mut params = vec![
        ("ym_algo".to_string(), channel.algo as f32),
        ("ym_feedback".to_string(), channel.feedback as f32),
    ];
    let mut approx = false;
    for slot in 0..4 {
        let row = &channel.rows[[0, 2, 1, 3][slot]];
        let prefix = format!("op{}", slot + 1);
        params.push((format!("{prefix}_mult"), (row.mult & 0x0F) as f32));
        params.push((format!("{prefix}_tl"), (row.tl & 0x7F) as f32));
        params.push((format!("{prefix}_ar"), (row.ar & 0x1F) as f32));
        params.push((format!("{prefix}_dr"), (row.dr & 0x1F) as f32));
        params.push((format!("{prefix}_sr"), (row.sr & 0x1F) as f32));
        params.push((format!("{prefix}_sl"), ((row.sl >> 4) & 0x0F) as f32));
        params.push((format!("{prefix}_rr"), (row.rr & 0x0F) as f32));
        if row.ssg & 0x08 != 0 {
            approx = true; // SSG-EG shape approximated (flagged)
        }
    }
    (params, approx)
}

/// Merge point: when reopening a voice immediately after a sub-44-sample
/// note with an identical program, drop the transient (register halves land
/// a few samples apart) and extend the new note backwards.
fn merged_start(
    notes: &mut Vec<TrackNote>,
    voice: u8,
    start: u64,
    velocity: f32,
    params: &[(String, f32)],
    approx: bool,
) -> (u64, bool) {
    if let Some(last) = notes.last() {
        if last.voice == voice
            && last.start + last.duration == start
            && last.duration < 44
            && last.params == params
            && (last.velocity - velocity).abs() < 1e-6
        {
            let merged = (last.start, last.approx || approx);
            notes.pop();
            return merged;
        }
    }
    (start, approx)
}

/// Close the sounding note on `channel` (if any) at sample `at`.
fn close_note(channels: &mut [FmChannel; 6], notes: &mut Vec<TrackNote>, channel: usize, at: u64) {
    if let Some((start, midi, params, approx)) = channels[channel].active.take() {
        if at > start {
            notes.push(TrackNote {
                start,
                duration: at - start,
                voice: channel as u8,
                midi,
                velocity: 1.0,
                params,
                approx,
            });
        }
    }
}

/// Close the sounding note on `channel` (if any) and reopen it, but only
/// when the rounded pitch actually changed: vibrato wobble within a semitone
/// never splits (our synths play 12-TET anyway).
fn split_on_pitch(
    channels: &mut [FmChannel; 6],
    notes: &mut Vec<TrackNote>,
    channel: usize,
    at: u64,
    clock: u32,
) {
    let sounding = channels[channel].active.as_ref().map(|(_, midi, _, _)| *midi);
    let freq = fnum_freq(channels[channel].fnum, channels[channel].block, clock);
    let midi = freq_midi(freq);
    if sounding == Some(midi) {
        // Sub-semitone slide/vibrato: pitch center is exact in 12-TET, and
        // our synths hold the center (no LFO model) — not flagged.
        return;
    }
    split_note(channels, notes, channel, at, clock);
}

/// Close the sounding note on `channel` (if any) and reopen it with the
/// current frequency snapshot.
fn split_note(
    channels: &mut [FmChannel; 6],
    notes: &mut Vec<TrackNote>,
    channel: usize,
    at: u64,
    clock: u32,
) {
    if channels[channel].active.is_none() {
        return;
    }
    if let Some((start, midi, params, approx)) = channels[channel].active.take() {
        if at > start {
            notes.push(TrackNote {
                start,
                duration: at - start,
                voice: channel as u8,
                midi,
                velocity: 1.0,
                params,
                approx,
            });
        }
    }
    let freq = fnum_freq(channels[channel].fnum, channels[channel].block, clock);
    let (params, approx) = fm_snapshot(&channels[channel]);
    let (start, approx) = merged_start(notes, channel as u8, at, 1.0, &params, approx);
    channels[channel].active = Some((start, freq_midi(freq), params, approx));
}

/// Track the six FM channels. Returns notes in time order.
pub fn track_fm(commands: &[(usize, u64, Command)], clock: u32, end_sample: u64) -> Vec<TrackNote> {
    let mut channels: [FmChannel; 6] = Default::default();
    let mut notes = Vec::new();
    for (_, sample, command) in commands {
        if let Command::YmWrite { port, addr, val } = command {
            let channel = ((addr & 0x03) + port * 3) as usize;
            if channel >= 6 {
                continue;
            }
            match addr {
                0x28 => {
                    let slots = (val >> 4) & 0x0F;
                    let target = (((val & 0x04) >> 2) * 3 + (val & 0x03)) as usize;
                    if target >= 6 {
                        continue;
                    }
                    if slots == 0 {
                        close_note(&mut channels, &mut notes, target, *sample);
                    } else {
                        close_note(&mut channels, &mut notes, target, *sample); // retrigger cuts
                        let freq = fnum_freq(
                            channels[target].fnum,
                            channels[target].block,
                            clock,
                        );
                        let midi = freq_midi(freq);
                        let (params, approx) = fm_snapshot(&channels[target]);
                        let (start, approx) =
                            merged_start(&mut notes, target as u8, *sample, 1.0, &params, approx);
                        channels[target].active = Some((start, midi, params, approx));
                    }
                }
                0xA0..=0xA2 => {
                    let ch = (addr - 0xA0 + port * 3) as usize;
                    if ch < 6 {
                        let new = (channels[ch].fnum & 0x700) | *val as u16;
                        channels[ch].fnum = new;
                        split_on_pitch(&mut channels, &mut notes, ch, *sample, clock);
                    }
                }
                0xA4..=0xA6 => {
                    let ch = (addr - 0xA4 + port * 3) as usize;
                    if ch < 6 {
                        let block = (val >> 3) & 0x07;
                        let new = (channels[ch].fnum & 0xFF) | (((val & 0x07) as u16) << 8);
                        channels[ch].block = block;
                        channels[ch].fnum = new;
                        split_on_pitch(&mut channels, &mut notes, ch, *sample, clock);
                    }
                }
                0xB0..=0xB2 => {
                    let ch = (addr - 0xB0 + port * 3) as usize;
                    if ch < 6 {
                        channels[ch].algo = val & 0x07;
                        channels[ch].feedback = (val >> 3) & 0x07;
                    }
                }
                0xB4..=0xB6 => {
                    let ch = (addr - 0xB4 + port * 3) as usize;
                    if ch < 6 {
                        channels[ch].pan = (val >> 6) & 0x03;
                    }
                }
                0x30..=0x3E => {
                    let row = ((addr - 0x30) >> 2) as usize;
                    let ch = (((addr - 0x30) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].mult = *val;
                    }
                }
                0x40..=0x4E => {
                    let row = ((addr - 0x40) >> 2) as usize;
                    let ch = (((addr - 0x40) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].tl = *val;
                    }
                }
                0x50..=0x5E => {
                    let row = ((addr - 0x50) >> 2) as usize;
                    let ch = (((addr - 0x50) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].ar = *val;
                    }
                }
                0x60..=0x6E => {
                    let row = ((addr - 0x60) >> 2) as usize;
                    let ch = (((addr - 0x60) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].dr = *val;
                    }
                }
                0x70..=0x7E => {
                    let row = ((addr - 0x70) >> 2) as usize;
                    let ch = (((addr - 0x70) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].sr = *val;
                    }
                }
                0x80..=0x8E => {
                    let row = ((addr - 0x80) >> 2) as usize;
                    let ch = (((addr - 0x80) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].sl = *val;
                        channels[ch].rows[row].rr = *val;
                    }
                }
                0x90..=0x9E => {
                    let row = ((addr - 0x90) >> 2) as usize;
                    let ch = (((addr - 0x90) & 0x03) + port * 3) as usize;
                    if ch < 6 && row < 4 {
                        channels[ch].rows[row].ssg = *val;
                    }
                }
                _ => {}
            }
        }
    }
    for channel in 0..6 {
        close_note(&mut channels, &mut notes, channel, end_sample);
    }
    notes.sort_by_key(|note| (note.start, note.voice));
    notes
}

// --- PSG tracker ---

const PSG_CLOCK: f32 = 3_579_545.0;

/// Track SN76489 writes: tone channels 0-2 (voices 10-12), noise (voice 13).
pub fn track_psg(commands: &[(usize, u64, Command)], end_sample: u64) -> Vec<TrackNote> {
    let mut freq = [0u16; 4];
    let mut volume: [u8; 4] = [0x0F; 4];
    let mut latched = 0usize;
    let mut sounding: [Option<(u64, u8, f32)>; 4] = [None, None, None, None];
    let mut notes = Vec::new();
    let loudness = |attenuation: u8| (15 - attenuation.min(15)) as f32 / 15.0;
    let close = |sounding: &mut [Option<(u64, u8, f32)>; 4], channel: usize, at: u64, notes: &mut Vec<TrackNote>| {
        if let Some((start, midi, velocity)) = sounding[channel].take() {
            if at > start {
                notes.push(TrackNote {
                    start,
                    duration: at - start,
                    voice: 10 + channel as u8,
                    midi,
                    velocity,
                    params: vec![],
                    approx: false,
                });
            }
        }
    };
    for (_, sample, command) in commands {
        if let Command::PsgWrite(byte) = command {
            if byte & 0x80 != 0 {
                latched = ((byte >> 5) & 0x03) as usize;
                if byte & 0x10 != 0 {
                    // Volume latch.
                    let attenuation = byte & 0x0F;
                    volume[latched] = attenuation;
                    if latched < 3 {
                        if attenuation == 0x0F {
                            close(&mut sounding, latched, *sample, &mut notes);
                        } else if sounding[latched].is_none() && freq[latched] > 0 {
                            let tone = PSG_CLOCK / 32.0 / freq[latched] as f32;
                            let velocity = loudness(attenuation);
                            let (start, _) = merged_start(
                                &mut notes,
                                10 + latched as u8,
                                *sample,
                                velocity,
                                &[],
                                false,
                            );
                            sounding[latched] =
                                Some((start, freq_midi(tone.max(1.0)), velocity));
                        }
                    } else if attenuation == 0x0F {
                        close(&mut sounding, 3, *sample, &mut notes);
                    } else if sounding[3].is_none() {
                        let velocity = loudness(attenuation);
                        let (start, _) =
                            merged_start(&mut notes, 13, *sample, velocity, &[], false);
                        sounding[3] = Some((start, 60, velocity));
                    }
                } else {
                    // Frequency low latch.
                    freq[latched] = (freq[latched] & 0x3F0) | (byte & 0x0F) as u16;
                    if latched < 3 && volume[latched] != 0x0F && freq[latched] > 0 {
                        // Retune: split only across semitones (vibrato stays glued).
                        let tone = PSG_CLOCK / 32.0 / freq[latched] as f32;
                        let midi = freq_midi(tone.max(1.0));
                        if sounding[latched].map(|(_, held, _)| held) != Some(midi) {
                            close(&mut sounding, latched, *sample, &mut notes);
                            let velocity = loudness(volume[latched]);
                            let (start, _) = merged_start(
                                &mut notes,
                                10 + latched as u8,
                                *sample,
                                velocity,
                                &[],
                                false,
                            );
                            sounding[latched] = Some((start, midi, velocity));
                        }
                    }
                }
            } else {
                // Frequency high data.
                freq[latched] = (freq[latched] & 0x0F) | (((byte & 0x3F) as u16) << 4);
            }
        }
    }
    for channel in 0..4 {
        close(&mut sounding, channel, end_sample, &mut notes);
    }
    notes.sort_by_key(|note| (note.start, note.voice));
    notes
}

// --- emission ---

/// Detect the tick grid: most common note duration above 100 samples.
/// Drivers sequence on a grid; command waits include sub-tick updates.
pub fn detect_tick(notes: &[TrackNote]) -> u64 {
    let mut hist: HashMap<u64, usize> = HashMap::new();
    for note in notes {
        if note.duration > 100 {
            *hist.entry(note.duration).or_default() += 1;
        }
    }
    hist.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(dur, _)| dur)
        .unwrap_or(735)
}

/// Tempo making one tick exactly a 128th note.
pub fn tempo_for_tick(tick_samples: u64) -> f32 {
    240.0 * 44100.0 / (128.0 * tick_samples.max(1) as f32)
}

/// Split notes into (intro, looping) at a loop sample point.
/// Looping notes are rebased to start at 0; intro notes crossing the
/// boundary are clipped. Without a loop point everything is intro.
pub fn split_loop(
    notes: Vec<TrackNote>,
    loop_sample: Option<u64>,
) -> (Vec<TrackNote>, Vec<TrackNote>) {
    match loop_sample {
        Some(at) => {
            let (mut intro, mut looping) = (Vec::new(), Vec::new());
            for mut note in notes {
                if note.start < at {
                    // Intro notes ring out naturally past the loop point.
                    intro.push(note);
                } else {
                    note.start -= at;
                    looping.push(note);
                }
            }
            (intro, looping)
        }
        None => (notes, Vec::new()),
    }
}

/// Pitch const name for a MIDI number (sharp spelling): ("cs", 4) etc.
pub fn midi_name(midi: u8) -> (&'static str, i8) {
    const NAMES: [&str; 12] = ["c", "cs", "d", "ds", "e", "f", "fs", "g", "gs", "a", "as", "b"];
    (NAMES[(midi % 12) as usize], midi as i8 / 12 - 1)
}

/// Duration in 128th-note ticks to literal + tie suffixes.
/// Greedy over a strictly descending table, so any tick count is exact.
pub fn ticks_to_durations(mut ticks: u64) -> Vec<&'static str> {
    const TABLE: [(&str, u64); 24] = [
        ("wdd", 224),
        ("wd", 192),
        ("w", 128),
        ("hddd", 120),
        ("hdd", 112),
        ("hd", 96),
        ("h", 64),
        ("qddd", 60),
        ("qdd", 56),
        ("qd", 48),
        ("q", 32),
        ("eddd", 30),
        ("edd", 28),
        ("ed", 24),
        ("e", 16),
        ("idd", 14),
        ("id", 12),
        ("i", 8),
        ("tdd", 7),
        ("td", 6),
        ("t", 4),
        ("xd", 3),
        ("x", 2),
        ("o", 1),
    ];
    let mut out = Vec::new();
    while ticks > 0 {
        let mut picked = ("o", 1);
        for candidate in TABLE {
            if candidate.1 <= ticks {
                picked = candidate;
                break;
            }
        }
        out.push(picked.0);
        ticks -= picked.1;
    }
    out
}

/// Snap a voice lane to the tick grid: durations round to whole ticks and
/// legato chains are re-laid cumulatively, so chains stay internally exact
/// while chain heads keep their absolute (unquantized) positions.
/// Public so the comparison harness snaps the reference model identically.
pub fn snap_voice(notes: &[TrackNote], tick: u64) -> Vec<TrackNote> {
    let tick = tick.max(1);
    let snap_dur = |duration: u64| ((duration + tick / 2) / tick).max(1) * tick;
    let mut out: Vec<TrackNote> = Vec::new();
    let mut cursor: Option<u64> = None;
    let mut prev_end: Option<u64> = None;
    for note in notes {
        let linked = prev_end == Some(note.start);
        let start = if linked { cursor.unwrap() } else { note.start };
        let mut snapped = note.clone();
        snapped.start = start;
        snapped.duration = snap_dur(note.duration);
        cursor = Some(start + snapped.duration);
        prev_end = Some(note.start + note.duration);
        out.push(snapped);
    }
    out
}

/// Merge consecutive `param!(k=v)` items into multi-pair calls.
/// Keeps `item_setups` aligned (grouped item takes the post-group state).
fn group_params(
    items: Vec<String>,
    setups: Vec<HashMap<String, f32>>,
) -> (Vec<String>, Vec<HashMap<String, f32>>) {
    let mut out = Vec::new();
    let mut out_setups = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut pending_setup: Option<HashMap<String, f32>> = None;
    let flush = |pending: &mut Vec<String>,
                   pending_setup: &mut Option<HashMap<String, f32>>,
                   out: &mut Vec<String>,
                   out_setups: &mut Vec<HashMap<String, f32>>| {
        if !pending.is_empty() {
            out.push(format!("param!({})", pending.join(", ")));
            out_setups.push(pending_setup.take().unwrap_or_default());
            pending.clear();
        }
    };
    for (item, setup) in items.into_iter().zip(setups) {
        if let Some(inner) = item
            .strip_prefix("param!(")
            .and_then(|inner| inner.strip_suffix(')'))
        {
            pending.push(inner.to_string());
            pending_setup = Some(setup);
        } else {
            flush(&mut pending, &mut pending_setup, &mut out, &mut out_setups);
            out.push(item);
            out_setups.push(setup);
        }
    }
    flush(&mut pending, &mut pending_setup, &mut out, &mut out_setups);
    (out, out_setups)
}

/// One voice lane (one channel) rendered as a `ser!` lane plus extracted
/// `seg_N()` phrase functions. Returns (lane source, seg definitions).
pub fn emit_voice(
    notes: &[TrackNote],
    tick_samples: u64,
    instrument: &str,
    seg_prefix: &str,
) -> (String, Vec<String>) {
    let tick = tick_samples.max(1);
    let notes = snap_voice(notes, tick);
    let to_ticks = |samples: u64| (samples + tick / 2) / tick;
    let mut items: Vec<String> = Vec::new();
    let mut item_setups: Vec<HashMap<String, f32>> = Vec::new();
    let mut cursor = 0u64;
    let mut setup: HashMap<String, f32> = HashMap::new();
    let mut first = true;
    for note in &notes {
        let start_tick = (note.start + tick / 2) / tick;
        if start_tick > cursor {
            let rest = ticks_to_durations(start_tick - cursor);
            for (i, suffix) in rest.iter().enumerate() {
                items.push(if i == 0 { format!("r{suffix}") } else { suffix.to_string() });
                item_setups.push(setup.clone());
            }
        }
        let dur_ticks = to_ticks(note.duration).max(1);
        // Voice setup: full snapshot on the first note, diffs after.
        // TEMP-EXPERIMENT full snapshots (delete after bisect).
        let mut keys: Vec<&String> = note.params.iter().map(|(key, _)| key).collect();
        keys.sort();
        keys.dedup();
        if first {
            items.push(format!("param!(instrument=\"{instrument}\")"));
            item_setups.push(setup.clone());
            for key in &keys {
                let value = note.params.iter().find(|(k, _)| k == *key).unwrap().1;
                items.push(format!("param!({key}={value})"));
                setup.insert((*key).clone(), value);
                item_setups.push(setup.clone());
            }
            first = false;
        } else {
            for key in &keys {
                let value = note.params.iter().find(|(k, _)| k == *key).unwrap().1;
                if setup.get(*key) != Some(&value) {
                    items.push(format!("param!({key}={value})"));
                    setup.insert((*key).clone(), value);
                    item_setups.push(setup.clone());
                }
            }
        }
        let (name, octave) = midi_name(note.midi);
        let durations = ticks_to_durations(dur_ticks);
        let octave_str = if octave < 0 {
            format!("_{}", -octave)
        } else {
            octave.to_string()
        };
        // Velocity rides as a per-note attr in MIDI units (default 100).
        // FM omits it (TL carries loudness); PSG always carries it so the
        // nibble-derived loudness survives the /127 field scaling exactly.
        let base = if instrument == "psg" || (note.velocity - 1.0).abs() > 1e-6 {
            let midi_vel = if instrument == "psg" {
                note.velocity * 127.0
            } else {
                note.velocity
            };
            format!("{name}{octave_str}{}!(velocity={midi_vel:.2})", durations[0])
        } else {
            format!("{name}{octave_str}{}", durations[0])
        };
        items.push(base);
        item_setups.push(setup.clone());
        for tie in &durations[1..] {
            items.push(tie.to_string());
            item_setups.push(setup.clone());
        }
        if note.approx {
            items.push("comment!(\"approx timbre\")".to_string());
            item_setups.push(setup.clone());
        }
        cursor = start_tick + dur_ticks;
    }
    // Group single-pair params, then phrase extraction, then repeat! runs.
    let (items, item_setups) = group_params(items, item_setups);
    // Phrase extraction first (multi-item repeats), then repeat! runs.
    let (items, mut segs) = if std::env::var("MMLX_NO_SEGS").is_ok() { (items, Vec::new()) } else { extract_phrases(items, item_setups, seg_prefix, 16) };
    // Compress consecutive identical atoms into repeat!.
    // NEVER compress bare duration ties (w/q/hd/...): repeat! would clone
    // the preceding note/rest instead of extending it. Same for markers.
    let compressible = |item: &str| {
        !(item.starts_with("param!(")
            || item.starts_with("comment!(")
            || item.starts_with("repeat!(")
            || matches!(
                item,
                "w" | "h" | "q" | "e" | "i" | "t" | "x" | "o"
                    | "wd" | "hd" | "qd" | "ed" | "id" | "td" | "xd"
                    | "wdd" | "hdd" | "qdd" | "edd" | "idd" | "tdd"
                    | "wddd" | "hddd" | "qddd" | "eddd"
            ))
    };
    let mut compressed: Vec<String> = Vec::new();
    let mut index = 0;
    while index < items.len() {
        let mut run = 1;
        while index + run < items.len()
            && compressible(&items[index])
            && items[index + run] == items[index]
        {
            run += 1;
        }
        compressed.push(items[index].clone());
        if run > 1 {
            compressed.push(format!("repeat!({})", run - 1));
        }
        index += run;
    }
    let lane = format!(
        "ser!([\n{}\n    ])",
        pack_items(&compressed, "        ")
    );
    segs.sort();
    (lane, segs)
}

/// Extract repeated phrases into `seg_N()` functions.
///
/// A phrase merges only when items AND starting ambient setups match, so
/// every call site sounds identical (ambient evolves deterministically
/// through identical items). Returns (top-level items, seg definitions).
fn extract_phrases(
    items: Vec<String>,
    setups: Vec<HashMap<String, f32>>,
    prefix: &str,
    min_len: usize,
) -> (Vec<String>, Vec<String>) {
    let mut items = items;
    let mut setups = setups;
    let mut segs = Vec::new();
    loop {
        // Longest run first; occurrences counted once for the winner, which
        // must occur 3+ times (fast path: give up after the first miss).
        let mut best: Option<(usize, usize)> = None; // (len, first)
        let n = items.len();
        if n < min_len * 2 {
            break;
        }
        let mut i = 0;
        while i + min_len <= n {
            let mut j = i + 1;
            while j + min_len <= n {
                if items[i] == items[j] && setups[i] == setups[j] {
                    let mut len = 1;
                    while i + len < n
                        && j + len < n
                        && items[i + len] == items[j + len]
                        && not_overlapping(i, j, len)
                    {
                        len += 1;
                    }
                    if len >= min_len && len > best.map(|(l, _)| l).unwrap_or(0) {
                        best = Some((len, i));
                    }
                    j += len.max(1);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
        let Some((len, first)) = best else {
            break;
        };
        if count_occurrences(&items, &setups, first, len) < 3 {
            break;
        }
        let name = format!("{prefix}_seg_{}", segs.len());
        let body = pack_items(&items[first..first + len], "        ");
        segs.push(format!(
            "#[rustfmt::skip]\nfn {name}() -> Note {{\n    ser!([\n{body}\n    ])\n}}"
        ));
        let call = format!("{name}()");
        // Rebuild: splice calls at all occurrence positions except the first.
        let mut next_items = Vec::new();
        let mut next_setups = Vec::new();
        let mut k = 0;
        let mut first_kept = false;
        while k < items.len() {
            if k + len <= items.len()
                && items[k..k + len] == items[first..first + len]
                && setups[k] == setups[first]
            {
                if !first_kept && k == first {
                    next_items.extend(items[k..k + len].iter().cloned());
                    next_setups.extend(setups[k..k + len].iter().cloned());
                    first_kept = true;
                } else {
                    next_items.push(call.clone());
                    // Post-call ambient is the phrase's END state, not its
                    // start: later diffs compare against it.
                    next_setups.push(apply_items(&setups[k], &items[k..k + len]));
                }
                k += len;
            } else {
                next_items.push(items[k].clone());
                next_setups.push(setups[k].clone());
                k += 1;
            }
        }
        items = next_items;
        setups = next_setups;
        if segs.len() >= 200 {
            break;
        }
    }
    (items, segs)
}

fn not_overlapping(i: usize, j: usize, len: usize) -> bool {
    j >= i + len
}

/// Pack items comma-separated, ~100 columns per line (reads like bars).
fn pack_items(items: &[String], indent: &str) -> String {
    let mut lines = vec![String::new()];
    for item in items {
        let current = lines.last_mut().unwrap();
        let piece = if current.is_empty() {
            item.clone()
        } else {
            format!(", {item}")
        };
        if current.len() + piece.len() > 100 && !current.is_empty() {
            lines.push(item.clone());
        } else {
            current.push_str(&piece);
        }
    }
    lines
        .iter()
        .map(|line| format!("{indent}{line},"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Apply a run of items' param diffs to an ambient setup, yielding the
/// end state. String params (instrument) are lane-constant; repeats and
/// notes leave the ambient unchanged.
fn apply_items(ambient: &HashMap<String, f32>, items: &[String]) -> HashMap<String, f32> {
    let mut state = ambient.clone();
    for item in items {
        if let Some(inner) = item
            .strip_prefix("param!(")
            .and_then(|inner| inner.strip_suffix(')'))
        {
            for part in inner.split(',') {
                if let Some((key, value)) = part.split_once('=') {
                    let key = key.trim();
                    if let Ok(number) = value.trim().parse::<f32>() {
                        state.insert(key.to_string(), number);
                    }
                }
            }
        }
    }
    state
}

/// Greedy non-overlapping occurrence count of the run at `first`.
fn count_occurrences(
    items: &[String],
    setups: &[HashMap<String, f32>],
    first: usize,
    len: usize,
) -> usize {
    let mut count = 0;
    let mut k = 0;
    while k + len <= items.len() {
        if items[k..k + len] == items[first..first + len] && setups[k] == setups[first] {
            count += 1;
            k += len;
        } else {
            k += 1;
        }
    }
    count
}

/// Full song: intro + loop voices, each a `ser!` lane inside `par!`.
/// `voices`: (instrument, intro notes, loop notes). `tempo` heads the mix.
pub fn emit_song(
    name: &str,
    voices: &[(String, Vec<TrackNote>, Vec<TrackNote>)],
    tick_samples: u64,
    tempo: f32,
) -> String {
    let mut intro_lanes: Vec<String> = Vec::new();
    let mut loop_lanes: Vec<String> = Vec::new();
    let mut seg_defs: Vec<String> = Vec::new();
    for (index, (instrument, intro, looping)) in voices.iter().enumerate() {
        let (intro_src, mut intro_segs) =
            emit_voice(intro, tick_samples, instrument, &format!("v{index}i"));
        let (loop_src, mut loop_segs) =
            emit_voice(looping, tick_samples, instrument, &format!("v{index}l"));
        intro_lanes.push(format!("        {intro_src}"));
        loop_lanes.push(format!("        {loop_src}"));
        seg_defs.append(&mut intro_segs);
        seg_defs.append(&mut loop_segs);
    }
    format!(
        "/// Decompiled `{name}` (tempo {tempo}).\n\
         /// `{name}` plays the intro once; `loop_{name}` is the looping body.\n\
         /// Packed-bar layout; `#[rustfmt::skip]` keeps it stable.\n\
         use mmlx_core::prelude::*;\n\
         \n\
         {segs}\
         #[rustfmt::skip]\n\
         pub fn {name}() -> Note {{\n\
         \x20   par!([\n\
         \x20       param!(tempo={tempo}),\n\
         {intro}\n\
         \x20   ])\n\
         }}\n\
         \n\
         #[rustfmt::skip]\n\
         pub fn loop_{name}() -> Note {{\n\
         \x20   par!([\n\
         \x20       param!(tempo={tempo}),\n\
         {lp}\n\
         \x20   ])\n\
         }}",
        segs = if seg_defs.is_empty() {
            String::new()
        } else {
            seg_defs.join("\n\n") + "\n\n"
        },
        intro = intro_lanes.join(",\n"),
        lp = loop_lanes.join(",\n"),
    )
}
