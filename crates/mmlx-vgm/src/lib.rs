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
    let data_start = if u32v(0x34) == 0 {
        0x40
    } else {
        0x34 + u32le(0x34)
    };
    let loop_offset = if u32v(0x1C) == 0 {
        None
    } else {
        Some(0x1C + u32le(0x1C))
    };
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
pub fn parse_commands(
    data: &[u8],
    header: &VgmHeader,
) -> Result<Vec<(usize, u64, Command)>, String> {
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
                let size = u32::from_le_bytes([data[i + 2], data[i + 3], data[i + 4], data[i + 5]])
                    as usize;
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
    (69.0 + 12.0 * (freq / 440.0).log2())
        .round()
        .clamp(0.0, 127.0) as u8
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
    let sounding = channels[channel]
        .active
        .as_ref()
        .map(|(_, midi, _, _)| *midi);
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
                        let freq = fnum_freq(channels[target].fnum, channels[target].block, clock);
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
    let close = |sounding: &mut [Option<(u64, u8, f32)>; 4],
                 channel: usize,
                 at: u64,
                 notes: &mut Vec<TrackNote>| {
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
                            sounding[latched] = Some((start, freq_midi(tone.max(1.0)), velocity));
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
    const NAMES: [&str; 12] = [
        "c", "cs", "d", "ds", "e", "f", "fs", "g", "gs", "a", "as", "b",
    ];
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

/// A named voice program: the full ambient snapshot a lane section plays
/// under, bound once per song fn as `let voice_<role>: Note` and spliced
/// via `voice.clone()` (single-use programs inline as `param!(...)`).
/// Splicing is sound because nested `ser!` shares ambient params with its
/// siblings (covered by `nested_ser_shares_ambient_params` in mmlx-songs).
#[derive(Default)]
pub struct ProgramReg {
    /// (match key, var name, snapshot pairs, instrument), voice order.
    entries: Vec<(String, String, Vec<(String, f32)>, String)>,
}

impl ProgramReg {
    pub(crate) fn snapshot_key(snapshot: &[(String, f32)], instrument: &str) -> String {
        let mut parts: Vec<String> = snapshot
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        parts.sort();
        format!("{instrument}|{}", parts.join(","))
    }

    /// Find or create the program for a full snapshot. The first program
    /// of a role is `voice_<role>`, later ones `voice_<role>_2`, ….
    pub fn intern(
        &mut self,
        role: &str,
        instrument: &str,
        mut snapshot: Vec<(String, f32)>,
    ) -> String {
        snapshot.sort_by(|a, b| a.0.cmp(&b.0));
        let key = Self::snapshot_key(&snapshot, instrument);
        if let Some(entry) = self.entries.iter().find(|(k, ..)| k == &key) {
            return entry.1.clone();
        }
        let name = if self.entries.is_empty() {
            format!("voice_{role}")
        } else {
            format!("voice_{}_{}", role, self.entries.len() + 1)
        };
        self.entries
            .push((key, name.clone(), snapshot, instrument.to_string()));
        name
    }

    /// All collected programs in first-use order:
    /// (match key, var name, snapshot pairs, instrument).
    pub fn programs(&self) -> &[(String, String, Vec<(String, f32)>, String)] {
        &self.entries
    }
}

/// Full-program `param!(...)` group text: instrument first, then sorted
/// snapshot. Shared by lane emission and the let/inline post-pass so the
/// two can never drift.
fn program_param_group(instrument: &str, snapshot: &[(String, f32)]) -> String {
    let mut pairs = vec![format!("instrument=\"{instrument}\"")];
    pairs.extend(snapshot.iter().map(|(key, value)| format!("{key}={value}")));
    format!("param!({})", pairs.join(", "))
}

/// Duration-literal suffix back to 128th-note ticks (for bar layout).
fn suffix_ticks(suffix: &str) -> u64 {
    let (base, dots) = match suffix.strip_suffix("ddd") {
        Some(base) => (base, 15),
        None => match suffix.strip_suffix("dd") {
            Some(base) => (base, 14),
            None => match suffix.strip_suffix('d') {
                Some(base) => (base, 12),
                None => (suffix, 8),
            },
        },
    };
    let ticks = match base {
        "w" => 128,
        "h" => 64,
        "q" => 32,
        "e" => 16,
        "i" => 8,
        "t" => 4,
        "x" => 2,
        _ => 1, // "o" and anything unrecognized: one tick
    };
    ticks * dots / 8
}

/// Parse a sounding item into (pitch-or-None-for-rest, duration suffix,
/// attr-or-None). Returns None for non-sounding items.
fn parse_sounding(item: &str) -> Option<(Option<String>, String, Option<String>)> {
    if is_bare_tie(item) || item.ends_with("()") || item.starts_with("repeat!(") {
        return None;
    }
    if let Some(suffix) = item.strip_prefix('r') {
        if is_bare_tie(suffix) && !suffix.is_empty() {
            return Some((None, suffix.to_string(), None));
        }
        return None;
    }
    if item.starts_with("param!(") || item.starts_with("comment!(") {
        return None;
    }
    let (core, attr) = match item.find("!(") {
        Some(at) => (&item[..at], Some(item[at + 1..].to_string())),
        None => (item, None),
    };
    let bytes = core.as_bytes();
    if bytes.is_empty() || !(b'a'..=b'g').contains(&bytes[0]) {
        return None;
    }
    let mut pos = 1;
    if core[pos..].starts_with("ss") || core[pos..].starts_with("ff") {
        pos += 2;
    } else if pos < core.len() && matches!(core.as_bytes()[pos], b's' | b'f' | b'n') {
        pos += 1;
    }
    if pos < core.len() && core.as_bytes()[pos] == b'_' {
        pos += 2;
    } else if pos < core.len() && core.as_bytes()[pos].is_ascii_digit() {
        pos += 1;
    } else {
        return None;
    }
    let (pitch, dur) = core.split_at(pos);
    if dur.is_empty() || !is_bare_tie(dur) {
        return None;
    }
    Some((Some(pitch.to_string()), dur.to_string(), attr))
}

/// Split notes/rests crossing bar lines into head + bare-tie pieces so
/// every item lies within one bar. Setups copy to all pieces.
fn split_spanning(
    items: Vec<String>,
    lens: Vec<u64>,
    setups: Vec<HashMap<String, f32>>,
    bar_ticks: u64,
) -> (Vec<String>, Vec<u64>, Vec<HashMap<String, f32>>) {
    if bar_ticks == 0 {
        return (items, lens, setups);
    }
    let mut out_items = Vec::new();
    let mut out_lens = Vec::new();
    let mut out_setups = Vec::new();
    let mut position = 0u64;
    for ((item, len), setup) in items.into_iter().zip(lens).zip(setups) {
        let end = position + len;
        // Sounding notes/rests split into head + bare-tie pieces; rest-gap
        // continuations are bare ties carrying ticks (len > 0) and split
        // into bare-tie pieces. Note-duration ties ride len 0 and stay
        // glued to their head.
        let splittable = parse_sounding(&item).is_some() || (len > 0 && is_bare_tie(&item));
        let crosses =
            len > 0 && position / bar_ticks != end.saturating_sub(1) / bar_ticks && splittable;
        if !crosses {
            out_items.push(item);
            out_lens.push(len);
            out_setups.push(setup);
            position = end;
            continue;
        }
        // Boundaries strictly inside (start, end).
        let mut bounds = vec![position];
        let mut boundary = (position / bar_ticks + 1) * bar_ticks;
        while boundary < end {
            bounds.push(boundary);
            boundary += bar_ticks;
        }
        bounds.push(end);
        // Bare-tie rest continuations split into bare-tie pieces (no head:
        // the `r` lives in an earlier bar).
        if is_bare_tie(&item) {
            for window in bounds.windows(2) {
                let seg_len = window[1] - window[0];
                for suffix in ticks_to_durations(seg_len) {
                    out_items.push(suffix.to_string());
                    out_lens.push(suffix_ticks(suffix));
                    out_setups.push(setup.clone());
                }
            }
            position = end;
            continue;
        }
        let (pitch, _, attr) = parse_sounding(&item).unwrap();
        let attr_suffix = attr.map(|a| format!("!{a}")).unwrap_or_default();
        for window in bounds.windows(2) {
            let seg_len = window[1] - window[0];
            let suffixes = ticks_to_durations(seg_len);
            for (k, suffix) in suffixes.iter().enumerate() {
                let text = if pitch.is_none() {
                    // Rest: head keeps `r`, continuations are bare ties.
                    if window[0] == position && k == 0 {
                        format!("r{suffix}")
                    } else {
                        suffix.to_string()
                    }
                } else if window[0] == position && k == 0 {
                    format!("{}{suffix}{attr_suffix}", pitch.clone().unwrap())
                } else {
                    suffix.to_string()
                };
                out_items.push(text);
                out_lens.push(suffix_ticks(suffix));
                out_setups.push(setup.clone());
            }
        }
        position = end;
    }
    (out_items, out_lens, out_setups)
}

/// Score order top-to-bottom for lanes inside a bar.
fn role_rank(role: &str) -> u64 {
    match role {
        "melody" => 0,
        "lead" => 1,
        "lead2" => 2,
        "arp" => 3,
        "arp2" => 4,
        "harmony3" => 5,
        "harmony2" => 6,
        "harmony" => 7,
        "bass" => 8,
        "drums" => 9,
        _ => 50,
    }
}

/// Repeat runs within one bar's items (runs never cross bars by
/// construction, so partition-exactness holds).
fn compress_runs(items: &[String]) -> Vec<String> {
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
    compressed
}

/// A whole-bar rest (`rw` for 4/4, decomposed otherwise) for wholly-silent
/// bars, keeping bar numbering exact across lanes.
fn full_bar_rest(bar_ticks: u64) -> String {
    if bar_ticks == 0 {
        return "rw".to_string();
    }
    let mut pieces = Vec::new();
    for (i, suffix) in ticks_to_durations(bar_ticks).iter().enumerate() {
        if i == 0 {
            pieces.push(format!("r{suffix}"));
        } else {
            pieces.push(suffix.to_string());
        }
    }
    pieces.join(" ")
}

/// Rest pieces for `[start, start + gap)` with an explicit `r` head opening
/// every bar window (reads like multi-bar rests in staff notation).
/// Bare ties continue within a window only. With bars disabled this is one
/// head plus bare ties; every piece carries its own literal length.
fn rest_pieces(start: u64, gap: u64, bar_ticks: u64) -> Vec<String> {
    if bar_ticks == 0 {
        return ticks_to_durations(gap)
            .iter()
            .enumerate()
            .map(|(i, suffix)| {
                if i == 0 {
                    format!("r{suffix}")
                } else {
                    suffix.to_string()
                }
            })
            .collect();
    }
    let mut pieces = Vec::new();
    let mut cursor = start;
    let end = start + gap;
    while cursor < end {
        let window_end = ((cursor / bar_ticks) + 1) * bar_ticks;
        let window_end = window_end.min(end);
        for (i, suffix) in ticks_to_durations(window_end - cursor).iter().enumerate() {
            pieces.push(if i == 0 {
                format!("r{suffix}")
            } else {
                suffix.to_string()
            });
        }
        cursor = window_end;
    }
    pieces
}

/// Merge consecutive `param!(k=v)` items into multi-pair calls.
/// Keeps `item_setups` aligned (grouped item takes the post-group state).
/// Threaded `kinds`/`lens` collapse to a single (`Other`, 0) entry.
fn group_params(
    items: Vec<String>,
    setups: Vec<HashMap<String, f32>>,
    lens: Vec<u64>,
) -> (Vec<String>, Vec<HashMap<String, f32>>, Vec<u64>) {
    let mut out = Vec::new();
    let mut out_setups = Vec::new();
    let mut out_lens = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut pending_setup: Option<HashMap<String, f32>> = None;
    let flush = |pending: &mut Vec<String>,
                 pending_setup: &mut Option<HashMap<String, f32>>,
                 out: &mut Vec<String>,
                 out_setups: &mut Vec<HashMap<String, f32>>,
                 out_lens: &mut Vec<u64>| {
        if !pending.is_empty() {
            out.push(format!("param!({})", pending.join(", ")));
            out_setups.push(pending_setup.take().unwrap_or_default());
            out_lens.push(0);
            pending.clear();
        }
    };
    for ((item, setup), len) in items.into_iter().zip(setups).zip(lens) {
        if let Some(inner) = item
            .strip_prefix("param!(")
            .and_then(|inner| inner.strip_suffix(')'))
        {
            pending.push(inner.to_string());
            pending_setup = Some(setup);
        } else {
            flush(
                &mut pending,
                &mut pending_setup,
                &mut out,
                &mut out_setups,
                &mut out_lens,
            );
            out.push(item);
            out_setups.push(setup);
            out_lens.push(len);
        }
    }
    flush(
        &mut pending,
        &mut pending_setup,
        &mut out,
        &mut out_setups,
        &mut out_lens,
    );
    (out, out_setups, out_lens)
}

/// Sorted full-program snapshot of a note (params only; the lane
/// instrument rides the registry key separately).
fn program_snapshot(note: &TrackNote) -> Vec<(String, f32)> {
    let mut snapshot: Vec<(String, f32)> = note
        .params
        .iter()
        .map(|(key, value)| (key.clone(), *value))
        .collect();
    snapshot.sort_by(|a, b| a.0.cmp(&b.0));
    snapshot
}

/// One voice lane (one channel) rendered as a terse `ser!` lane plus
/// extracted `seg_N()` phrase functions. Voice programs recurring across
/// runs become shared `voice_<role>()` call items (rewritten downstream to
/// `let` bindings or inline groups); once-only programs stay inline;
/// tweaks of 2 or fewer keys stay inline diffs. Items lay out one
/// music bar per line (`bar_ticks` grid ticks per bar; 0 disables), each
/// with a `// bar N` comment so the lane reads like staff notation.
/// `program_runs` maps program keys to contiguous-run counts (built by
/// the caller across both sections). `section_end` pads trailing silence
/// so all lanes share the same bar count (vertical alignment). Returns
/// lane + segs.
pub fn emit_voice(
    notes: &[TrackNote],
    tick_samples: u64,
    instrument: &str,
    role: &str,
    seg_prefix: &str,
    bar_ticks: u64,
    programs: &mut ProgramReg,
    program_runs: &HashMap<String, usize>,
    section_end: u64,
) -> (String, Vec<String>) {
    let tick = tick_samples.max(1);
    let notes = snap_voice(notes, tick);
    let to_ticks = |samples: u64| (samples + tick / 2) / tick;
    let mut items: Vec<String> = Vec::new();
    let mut item_lens: Vec<u64> = Vec::new();
    let mut item_setups: Vec<HashMap<String, f32>> = Vec::new();
    let mut cursor = 0u64;
    let mut setup: HashMap<String, f32> = HashMap::new();
    let mut setup_vel: Option<f32> = None;
    let mut first = true;
    let mut head_setup: Option<HashMap<String, f32>> = None;
    let mut any_approx = false;
    for note in &notes {
        let start_tick = (note.start + tick / 2) / tick;
        if start_tick > cursor {
            // Rest gap with an explicit head per bar window (staff-style
            // multi-bar rests); positions advance per literal.
            for piece in rest_pieces(cursor, start_tick - cursor, bar_ticks) {
                let lens = if let Some(suffix) = piece.strip_prefix('r') {
                    suffix_ticks(suffix)
                } else {
                    suffix_ticks(&piece)
                };
                items.push(piece);
                item_lens.push(lens);
                item_setups.push(setup.clone());
            }
        }
        let dur_ticks = to_ticks(note.duration).max(1);
        // Voice setup: programs recurring across runs become `voice_<role>()`
        // calls (shared registry); once-only programs stay inline as one
        // grouped item; tweaks of 2 or fewer keys stay inline diffs.
        let mut keys: Vec<&String> = note.params.iter().map(|(key, _)| key).collect();
        keys.sort();
        keys.dedup();
        let snapshot = program_snapshot(note);
        let frequent = program_runs
            .get(&ProgramReg::snapshot_key(&snapshot, instrument))
            .copied()
            .unwrap_or(0)
            >= 2;
        // One grouped full-program item (instrument first, then sorted).
        let full_group = || program_param_group(instrument, &snapshot);
        if first {
            if frequent {
                let voice = programs.intern(role, instrument, snapshot.clone());
                items.push(format!("{voice}()"));
                item_lens.push(0);
                for (key, value) in &snapshot {
                    setup.insert(key.clone(), *value);
                }
                item_setups.push(setup.clone());
            } else {
                items.push(full_group());
                item_lens.push(0);
                for (key, value) in &snapshot {
                    setup.insert(key.clone(), *value);
                }
                item_setups.push(setup.clone());
            }
            head_setup = Some(setup.clone());
            first = false;
        } else {
            let mut diffs: Vec<(&String, f32)> = Vec::new();
            for key in &keys {
                let value = note.params.iter().find(|(k, _)| k == *key).unwrap().1;
                if setup.get(*key) != Some(&value) {
                    diffs.push((*key, value));
                }
            }
            if diffs.len() > 2 {
                if frequent {
                    let voice = programs.intern(role, instrument, snapshot.clone());
                    items.push(format!("{voice}()"));
                    item_lens.push(0);
                    for (key, value) in &snapshot {
                        setup.insert(key.clone(), *value);
                    }
                    item_setups.push(setup.clone());
                } else {
                    items.push(full_group());
                    item_lens.push(0);
                    for (key, value) in &snapshot {
                        setup.insert(key.clone(), *value);
                    }
                    item_setups.push(setup.clone());
                }
            } else {
                for (key, value) in diffs {
                    items.push(format!("param!({key}={value})"));
                    item_lens.push(0);
                    setup.insert(key.clone(), value);
                    item_setups.push(setup.clone());
                }
            }
        }
        // Velocity rides the ambient `velocity` param (MIDI units, core
        // default 100): one param per run of equal loudness instead of a
        // per-note attr on every note. FM omits it (TL carries loudness).
        if instrument == "psg" {
            let midi_vel = (note.velocity * 127.0 * 100.0).round() / 100.0;
            if setup_vel != Some(midi_vel) {
                if (midi_vel - 100.0).abs() > 1e-6 {
                    items.push(format!("param!(velocity={midi_vel})"));
                    item_lens.push(0);
                    setup.insert("velocity".to_string(), midi_vel);
                    item_setups.push(setup.clone());
                }
                setup_vel = Some(midi_vel);
            }
        }
        let (name, octave) = midi_name(note.midi);
        let durations = ticks_to_durations(dur_ticks);
        let octave_str = if octave < 0 {
            format!("_{}", -octave)
        } else {
            octave.to_string()
        };
        // Notes are bare atoms now; loudness rides the ambient setup.
        // Each piece carries its own literal length (head + ties sum to
        // the true duration), so the bar partition sees exact time.
        let base = format!("{name}{octave_str}{}", durations[0]);
        items.push(base);
        item_lens.push(suffix_ticks(durations[0]));
        item_setups.push(setup.clone());
        for tie in &durations[1..] {
            items.push(tie.to_string());
            item_lens.push(suffix_ticks(tie));
            item_setups.push(setup.clone());
        }
        if note.approx {
            any_approx = true;
        }
        cursor = start_tick + dur_ticks;
    }
    // Trailing silence to the section end so every lane covers the same
    // bars (vertical alignment), explicit per bar window like all rests.
    if section_end > cursor {
        for piece in rest_pieces(cursor, section_end - cursor, bar_ticks) {
            let lens = if let Some(suffix) = piece.strip_prefix('r') {
                suffix_ticks(suffix)
            } else {
                suffix_ticks(&piece)
            };
            items.push(piece);
            item_lens.push(lens);
            item_setups.push(setup.clone());
        }
    }
    // One timbre note per lane beats per-note spam: SSG-EG and slide
    // approximations hold for the whole lane, flagged right up front.
    if any_approx {
        if let Some(head) = head_setup {
            items.insert(
                1.min(items.len()),
                "comment!(\"approx timbre\")".to_string(),
            );
            item_lens.insert(1.min(item_lens.len()), 0);
            item_setups.insert(1.min(item_setups.len()), head);
        }
    }
    // Group singles, split bar-crossers, extract bar-local phrases.
    // Lengths thread through so the bar partition stays exact.
    let (items, item_setups, item_lens) = group_params(items, item_setups, item_lens);
    let (items, item_lens, item_setups) = split_spanning(items, item_lens, item_setups, bar_ticks);
    // Phrase extraction (multi-item repeats, bar-local), then an exact
    // bar partition: after splitting nothing crosses a barline, so
    // cumulative lengths give absolute bar numbers. Repeat runs compress
    // within each bar (never across: a run would hide the barline).
    let (items, mut segs, item_lens) =
        extract_phrases(items, item_setups, item_lens, seg_prefix, 8, bar_ticks);
    let mut bars: Vec<(u64, Vec<String>)> = Vec::new();
    let mut position = 0u64;
    let mut current_bar = 0u64;
    let mut current: Vec<String> = Vec::new();
    let mut started = false;
    for (item, len) in items.iter().zip(item_lens.iter()) {
        let bar = if bar_ticks > 0 {
            position / bar_ticks
        } else {
            0
        };
        if !started {
            current_bar = bar;
            started = true;
        } else if bar != current_bar {
            bars.push((current_bar, compress_runs(&current)));
            current = Vec::new();
            // Fill wholly-silent bars so numbering stays exact.
            let mut missing = current_bar + 1;
            while missing < bar {
                bars.push((missing, vec![full_bar_rest(bar_ticks)]));
                missing += 1;
            }
            current_bar = bar;
        }
        current.push(item.clone());
        position += len;
    }
    if !current.is_empty() || bars.is_empty() {
        bars.push((current_bar, compress_runs(&current)));
    }
    // One line per bar with a bar-number comment (Rust `//`, stripped by
    // the macro tokenizer, so playback is unaffected). Reads like staff
    // systems: `notes // bar 3`.
    let mut lines: Vec<String> = Vec::new();
    for (bar, bar_items) in &bars {
        lines.push(format!("{} // bar {}", bar_items.join(" "), bar + 1));
    }
    let lane = format!("ser!(\n{}\n        )", lines.join("\n        "));
    segs.sort();
    (lane, segs)
}

/// Bare duration-tie literal (`q`, `o`, `qdd`, …): extends the previous
/// note/rest instead of sounding. Never repeat-compressed.
fn is_bare_tie(item: &str) -> bool {
    matches!(
        item,
        "w" | "h"
            | "q"
            | "e"
            | "i"
            | "t"
            | "x"
            | "o"
            | "wd"
            | "hd"
            | "qd"
            | "ed"
            | "id"
            | "td"
            | "xd"
            | "wdd"
            | "hdd"
            | "qdd"
            | "edd"
            | "idd"
            | "tdd"
            | "wddd"
            | "hddd"
            | "qddd"
            | "eddd"
    )
}

fn compressible(item: &str) -> bool {
    !(item.starts_with("param!(")
        || item.starts_with("comment!(")
        || item.starts_with("repeat!(")
        || item.ends_with("()")
        || is_bare_tie(item))
}

/// Extract repeated phrases into `seg_N()` functions.
///
/// A phrase merges only when items AND starting ambient setups match, so
/// every call site sounds identical (ambient evolves deterministically
/// through identical items). Returns (top-level items, seg definitions).
fn extract_phrases(
    items: Vec<String>,
    setups: Vec<HashMap<String, f32>>,
    lens: Vec<u64>,
    prefix: &str,
    min_len: usize,
    bar_ticks: u64,
) -> (Vec<String>, Vec<String>, Vec<u64>) {
    let mut items = items;
    let mut setups = setups;
    let mut lens = lens;
    let mut segs = Vec::new();
    loop {
        // Longest run first; the winner must PAY: at least 3 occurrences
        // with net item savings (definition + calls beat inline copies).
        // Runs must also lie within one bar (the partition downstream is
        // exact: nothing may cross a barline). Fast path: give up after
        // the first miss.
        let mut best: Option<(usize, usize)> = None; // (len, first)
        let n = items.len();
        if n < min_len * 2 {
            break;
        }
        let pos = item_positions(&lens);
        let mut i = 0;
        while i + min_len <= n {
            // A phrase must stand alone as a `ser!` body: it may not open
            // with a bare tie (nothing precedes it to extend) or a repeat
            // marker (nothing precedes it to clone).
            if is_bare_tie(&items[i]) || items[i].starts_with("repeat!(") {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            while j + min_len <= n {
                if items[i] == items[j] && setups[i] == setups[j] {
                    let mut len = 1;
                    let mut runlen = lens[i];
                    while i + len < n
                        && j + len < n
                        && items[i + len] == items[j + len]
                        && not_overlapping(i, j, len)
                        && same_bar(pos[i], runlen + lens[i + len], bar_ticks)
                        && same_bar(pos[j], runlen + lens[i + len], bar_ticks)
                    {
                        runlen += lens[i + len];
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
        let occurrences = count_occurrences(&items, &setups, first, len);
        // Inline cost occ*len vs def (len) + calls (occ): net must win.
        // And a phrase must truly recur: a single call site (2 occurrences
        // total) is not worth breaking out — keep it inline instead.
        let saved = occurrences as isize * len as isize - len as isize - occurrences as isize;
        if occurrences < 3 || saved <= 0 {
            break;
        }
        // A call site followed by a bare tie would panic (`Serial` then
        // tie): the tie must belong to the run, so reject runs with a
        // tie follower anywhere.
        if occurrence_followed_by_tie(&items, &setups, first, len) {
            break;
        }
        let name = format!("{prefix}_seg_{}", segs.len());
        let body = pack_items(&items[first..first + len], "        ");
        segs.push(format!(
            "#[rustfmt::skip]\nfn {name}() -> Note {{\n    ser!(\n{body}\n    )\n}}"
        ));
        let call = format!("{name}()");
        // The call occupies its expansion's tick length for layout.
        let call_len: u64 = lens[first..first + len].iter().sum();
        // Rebuild: splice calls at all occurrence positions except the first.
        let mut next_items = Vec::new();
        let mut next_setups = Vec::new();
        let mut next_lens = Vec::new();
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
                    next_lens.extend(lens[k..k + len].iter().cloned());
                    first_kept = true;
                } else {
                    next_items.push(call.clone());
                    // Post-call ambient is the phrase's END state, not its
                    // start: later diffs compare against it.
                    next_setups.push(apply_items(&setups[k], &items[k..k + len]));
                    next_lens.push(call_len);
                }
                k += len;
            } else {
                next_items.push(items[k].clone());
                next_setups.push(setups[k].clone());
                next_lens.push(lens[k]);
                k += 1;
            }
        }
        items = next_items;
        setups = next_setups;
        lens = next_lens;
        if segs.len() >= 200 {
            break;
        }
    }
    (items, segs, lens)
}

fn not_overlapping(i: usize, j: usize, len: usize) -> bool {
    j >= i + len
}

/// True when any occurrence of the run at `first` is immediately followed
/// by a bare tie outside the run. Splicing a call there would leave
/// `seg(), <tie>`, which panics (a tie must follow a note or rest).
fn occurrence_followed_by_tie(
    items: &[String],
    setups: &[HashMap<String, f32>],
    first: usize,
    len: usize,
) -> bool {
    let mut k = 0;
    while k + len <= items.len() {
        if items[k..k + len] == items[first..first + len] && setups[k] == setups[first] {
            if k + len < items.len() && is_bare_tie(&items[k + len]) {
                return true;
            }
            k += len;
        } else {
            k += 1;
        }
    }
    false
}

/// Pack items space-separated, ~100 columns per line. Commas terminate
/// each line (valid separators in the terse form); bars use their own
/// one-line-per-bar layout instead.
fn pack_items(items: &[String], indent: &str) -> String {
    let mut lines = vec![String::new()];
    for item in items {
        let current = lines.last_mut().unwrap();
        let piece = if current.is_empty() {
            item.clone()
        } else {
            format!(" {item}")
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

/// Cumulative start tick per item (plus total at the end).
fn item_positions(lens: &[u64]) -> Vec<u64> {
    let mut pos = Vec::with_capacity(lens.len() + 1);
    let mut acc = 0u64;
    for len in lens {
        pos.push(acc);
        acc += len;
    }
    pos.push(acc);
    pos
}

/// True when the span `[start, start + runlen)` lies within one bar
/// (or bars are disabled, or the span is empty).
fn same_bar(start: u64, runlen: u64, bar_ticks: u64) -> bool {
    bar_ticks == 0 || runlen == 0 || start / bar_ticks == (start + runlen - 1) / bar_ticks
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

/// Count how many notes use each full-program snapshot (across both
/// sections) so once-only programs stay inline instead of earning a
/// `voice_*()` definition.
fn program_run_counts(
    intro: &[TrackNote],
    looping: &[TrackNote],
    instrument: &str,
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for note in intro.iter().chain(looping.iter()) {
        let key = ProgramReg::snapshot_key(&program_snapshot(note), instrument);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Latest note end in grid ticks (for padding all lanes of a section to
/// the same bar count).
fn section_end_ticks(notes: &[TrackNote], tick: u64) -> u64 {
    let tick = tick.max(1);
    notes
        .iter()
        .map(|note| (note.start + tick / 2) / tick + (note.duration + tick / 2) / tick.max(1))
        .max()
        .unwrap_or(0)
}

/// Indent every line of a lane block for the mix body.
fn lane_indent(lane: &str) -> String {
    lane.replace('\n', "\n        ")
}

/// Full song: intro + loop voices, each a terse `ser!` lane inside `par!`.
/// `voices`: (instrument, role, intro notes, loop notes). `tempo` heads
/// the mix; `bar_ticks` is grid ticks per bar for the lane layout.
/// Voice programs bind once per song fn as `let voice_*` variables
/// (`voice.clone()` splices); single-use programs inline as `param!(...)`.
/// Lanes emit in score order (melody on top, drums at the
/// bottom) with one `// bar N` line per bar so the parts read like staff
/// systems and align vertically.
pub fn emit_song(
    name: &str,
    voices: &[(String, String, Vec<TrackNote>, Vec<TrackNote>)],
    tick_samples: u64,
    tempo: f32,
    bar_ticks: u64,
) -> String {
    let tick = tick_samples.max(1);
    // Score order for readability; section ends pad all lanes to the same
    // bar count so bar lines align vertically across staves.
    let mut ordered: Vec<(String, String, Vec<TrackNote>, Vec<TrackNote>)> = voices.to_vec();
    ordered.sort_by_key(|(_, role, _, _)| role_rank(role));
    let intro_end = ordered
        .iter()
        .map(|(_, _, intro, _)| section_end_ticks(intro, tick))
        .max()
        .unwrap_or(0);
    let loop_end = ordered
        .iter()
        .map(|(_, _, _, looping)| section_end_ticks(looping, tick))
        .max()
        .unwrap_or(0);
    let mut intro_lanes: Vec<String> = Vec::new();
    let mut loop_lanes: Vec<String> = Vec::new();
    let mut seg_defs: Vec<String> = Vec::new();
    // One program registry per role, shared across intro and loop so a
    // program used in both binds under one name in each song fn.
    // Order follows voices.
    let mut regs: Vec<(String, ProgramReg)> = Vec::new();
    for (instrument, role, intro, looping) in &ordered {
        let reg_index = match regs.iter().position(|(name, _)| name == role) {
            Some(index) => index,
            None => {
                regs.push((role.clone(), ProgramReg::default()));
                regs.len() - 1
            }
        };
        let reg = &mut regs[reg_index].1;
        let program_runs = program_run_counts(intro, looping, instrument);
        let (intro_src, mut intro_segs) = emit_voice(
            intro,
            tick_samples,
            instrument,
            role,
            &format!("{role}i"),
            bar_ticks,
            reg,
            &program_runs,
            intro_end,
        );
        let (loop_src, mut loop_segs) = emit_voice(
            looping,
            tick_samples,
            instrument,
            role,
            &format!("{role}l"),
            bar_ticks,
            reg,
            &program_runs,
            loop_end,
        );
        // Staff labels: `// melody (psg)` above each lane, like an
        // instrument name at the start of a staff system.
        intro_lanes.push(format!(
            "        // {role} ({instrument})\n        {}",
            lane_indent(&intro_src)
        ));
        loop_lanes.push(format!(
            "        // {role} ({instrument})\n        {}",
            lane_indent(&loop_src)
        ));
        seg_defs.append(&mut intro_segs);
        seg_defs.append(&mut loop_segs);
    }
    // Collect programs in first-use order across all roles.
    let programs: Vec<(String, String, Vec<(String, f32)>, String)> = regs
        .iter()
        .flat_map(|(_, reg)| reg.programs().iter().cloned())
        .collect();
    // Seg bodies are shared across call sites and sections: always inline
    // programs there as `param!(...)` groups so seg fns stay self-contained
    // (no cross-fn bindings to resolve).
    for (_, name, snapshot, instrument) in &programs {
        let pat = format!("{name}()");
        let group = program_param_group(instrument, snapshot);
        for seg in seg_defs.iter_mut() {
            if seg.contains(pat.as_str()) {
                *seg = seg.replace(pat.as_str(), &group);
            }
        }
    }
    let mut intro_mix = intro_lanes.join(",\n");
    let mut loop_mix = loop_lanes.join(",\n");
    // Lane programs: a single call site inlines the `param!(...)` group;
    // the rest bind once per song fn (`let voice: Note`) and splice via
    // `voice.clone()` (same nested-serial value a `voice()` call returned).
    let mut intro_lets: Vec<String> = Vec::new();
    let mut loop_lets: Vec<String> = Vec::new();
    for (_, name, snapshot, instrument) in &programs {
        let pat = format!("{name}()");
        let uses = intro_mix.matches(pat.as_str()).count() + loop_mix.matches(pat.as_str()).count();
        if uses == 0 {
            continue;
        }
        let group = program_param_group(instrument, snapshot);
        if uses == 1 {
            if intro_mix.contains(pat.as_str()) {
                intro_mix = intro_mix.replacen(pat.as_str(), &group, 1);
            } else {
                loop_mix = loop_mix.replacen(pat.as_str(), &group, 1);
            }
        } else {
            let binding = format!("let {name}: Note = ser!({group});");
            let use_site = format!("{name}.clone()");
            if intro_mix.contains(pat.as_str()) {
                intro_mix = intro_mix.replace(pat.as_str(), &use_site);
                intro_lets.push(binding.clone());
            }
            if loop_mix.contains(pat.as_str()) {
                loop_mix = loop_mix.replace(pat.as_str(), &use_site);
                loop_lets.push(binding);
            }
        }
    }
    let intro_lets_block = if intro_lets.is_empty() {
        String::new()
    } else {
        format!("    // voices\n    {}\n", intro_lets.join("\n    "))
    };
    let loop_lets_block = if loop_lets.is_empty() {
        String::new()
    } else {
        format!("    // voices\n    {}\n", loop_lets.join("\n    "))
    };
    format!(
        "/// Decompiled `{name}` (tempo {tempo}, bar = {bar_ticks} ticks).\n\
         /// `{name}` plays the intro once; `loop_{name}` is the looping body.\n\
         /// Terse form (no brackets, space-separated): one `// bar N` line\n\
         /// per bar, lanes in score order (melody on top, drums at the\n\
         /// bottom) so parts align vertically like staff systems;\n\
         /// `#[rustfmt::skip]` keeps it. Voice programs bind once per song\n\
         /// fn as `let voice_*` variables (`voice.clone()` splices them);\n\
         /// single-use programs inline as `param!(...)`; `seg_*()` phrases\n\
         /// are bar-local repeats.\n\
         use mmlx_core::prelude::*;\n\
         \n\
         {segs}\
         #[rustfmt::skip]\n\
         pub fn {name}() -> Note {{\n\
         {intro_lets_block}\
         \x20   par!(\n\
         \x20       param!(tempo={tempo}),\n\
         {intro}\n\
         \x20   )\n\
         }}\n\
         \n\
         #[rustfmt::skip]\n\
         pub fn loop_{name}() -> Note {{\n\
         {loop_lets_block}\
         \x20   par!(\n\
         \x20       param!(tempo={tempo}),\n\
         {lp}\n\
         \x20   )\n\
         }}",
        segs = if seg_defs.is_empty() {
            String::new()
        } else {
            seg_defs.join("\n\n") + "\n\n"
        },
        intro = intro_mix,
        lp = loop_mix,
    )
}
