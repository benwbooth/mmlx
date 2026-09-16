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
/// via `voice.clone()` (never inline: full groups crowd bar lines).
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
    if item.starts_with("param!(") {
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

/// Split note/rest units crossing bar lines so every emitted item lies
/// within one bar (a hard requirement for per-bar `ser!` calls, where a
/// leading bare tie would panic). A crossing NOTE becomes
/// `legato!(head, advance)` — full duration sounds, only `advance` ticks
/// elapse — plus explicit rest pieces covering the remainder span (rests
/// never cut sustains: voices ignore them). A crossing REST (defensive;
/// `rest_pieces` pre-splits, so this should not happen) becomes explicit
/// `r` heads, audio-identical for silence. Units contained in one bar
/// pass through untouched, ties glued to their head. Setups copy to all
/// pieces; positions stay exact.
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
    let mut index = 0;
    let n = items.len();
    while index < n {
        let item = &items[index];
        let len = lens[index];
        // Zero-length items (params, voice calls, comments) and seg/repeat
        // markers pass through in order; they never cross a barline.
        let sounding = parse_sounding(item);
        if sounding.is_none() {
            out_items.push(item.clone());
            out_lens.push(len);
            out_setups.push(setups[index].clone());
            position += len;
            index += 1;
            continue;
        }
        // Gather the unit: head plus following bare-tie pieces, which glue
        // to it regardless of length (note ties ride zero; rest-window
        // ties carry their own span). Emission lays head + ties adjacently
        // and grouping preserves sounding order, so a bare tie here always
        // continues this head.
        let (pitch, _, attr) = sounding.unwrap();
        let head_setup = setups[index].clone();
        let start = position;
        let mut pieces: Vec<(String, u64)> = vec![(item.clone(), len)];
        let mut end = position + len;
        position = end;
        index += 1;
        while index < n && is_bare_tie(&items[index]) {
            pieces.push((items[index].clone(), lens[index]));
            end += lens[index];
            position = end;
            index += 1;
        }
        let in_same_bar = end <= start || start / bar_ticks == end.saturating_sub(1) / bar_ticks;
        if in_same_bar {
            for (piece, piece_len) in &pieces {
                out_items.push(piece.clone());
                out_lens.push(*piece_len);
                out_setups.push(head_setup.clone());
            }
            continue;
        }
        // Crossing unit: advance to the first barline under legato, then
        // cover the remainder with explicit rests.
        let advance = (start / bar_ticks + 1) * bar_ticks - start;
        if pitch.is_none() {
            // Defensive rest path: explicit head per piece (silence is silence).
            for (piece, piece_len) in &pieces {
                let suffix = piece.strip_prefix('r').unwrap_or(piece);
                out_items.push(format!("r{suffix}"));
                out_lens.push(*piece_len);
                out_setups.push(head_setup.clone());
            }
            continue;
        }
        let attr_suffix = attr.map(|a| format!("!{a}")).unwrap_or_default();
        // Single-suffix head: legato wraps the atom directly.
        // Multi-suffix (e.g. `c4h t`): merge textually inside a nested
        // ser so ties still resolve against their head.
        let inner = if pieces.len() == 1 {
            pieces[0].0.clone()
        } else {
            let mut inner_pieces = Vec::new();
            for (k, (piece, _)) in pieces.iter().enumerate() {
                if k == 0 {
                    inner_pieces.push(format!("{piece}{attr_suffix}"));
                } else {
                    inner_pieces.push(piece.clone());
                }
            }
            format!("ser!({})", inner_pieces.join(" "))
        };
        out_items.push(format!("legato!({inner}, {advance})"));
        out_lens.push(advance);
        out_setups.push(head_setup.clone());
        for piece in rest_pieces(start + advance, end - (start + advance), bar_ticks) {
            let piece_len = suffix_ticks(piece.strip_prefix('r').unwrap_or(&piece));
            out_items.push(piece);
            out_lens.push(piece_len);
            out_setups.push(head_setup.clone());
        }
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
/// construction, so partition-exactness holds). Lengths thread through:
/// a `repeat!(k)` marker carries the `k` elided copies' ticks, so the
/// total span is unchanged and downstream layout stays exact.
fn compress_runs(items: &[String], lens: &[u64]) -> (Vec<String>, Vec<u64>) {
    let mut compressed: Vec<String> = Vec::new();
    let mut compressed_lens: Vec<u64> = Vec::new();
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
        compressed_lens.push(lens[index]);
        if run > 1 {
            // The marker carries the elided copies' exact ticks.
            let elided: u64 = lens[index + 1..index + run].iter().sum();
            compressed.push(format!("repeat!({})", run - 1));
            compressed_lens.push(elided);
        }
        index += run;
    }
    (compressed, compressed_lens)
}

/// A whole-bar rest (`rw` for 4/4, decomposed otherwise) as separate
/// pieces with exact tick lengths, keeping bar numbering exact across
/// lanes and feeding time-aligned bar columns.
fn full_bar_pieces(bar_ticks: u64) -> (Vec<String>, Vec<u64>) {
    if bar_ticks == 0 {
        return (vec!["rw".to_string()], vec![0]);
    }
    let mut items = Vec::new();
    let mut lens = Vec::new();
    for (i, suffix) in ticks_to_durations(bar_ticks).iter().enumerate() {
        let piece = if i == 0 {
            format!("r{suffix}")
        } else {
            suffix.to_string()
        };
        lens.push(suffix_ticks(suffix));
        items.push(piece);
    }
    (items, lens)
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

/// One bar of one lane: compressed display items with their exact tick
/// lengths plus the ambient setup at the bar's start (for program
/// restatement in bar-major assembly). Lengths make time-aligned
/// columns exact downstream (restatement prefixes are zero-length).
/// `start_setup` is the running setup after the last pre-bar item
/// (empty for the opening bar).
pub struct LaneBar {
    pub bar: u64,
    pub items: Vec<String>,
    pub lens: Vec<u64>,
    pub start_setup: HashMap<String, f32>,
}

/// One voice lane (one channel) rendered as bar-partitioned items plus
/// extracted `seg_N()` phrase functions. Every voice program becomes a
/// shared `voice_<role>()` call item (bound downstream as a `let`);
/// tweaks of 2 or fewer keys stay inline diffs. Items partition into one
/// exact bar each (`bar_ticks` grid ticks per bar; 0 disables), with the
/// ambient start setup recorded per bar for program restatement.
/// `section_end` pads trailing silence so all lanes share the same bar
/// count (vertical alignment). Returns per-bar items + segs; the caller
/// assembles bars into staves.
pub fn emit_voice(
    notes: &[TrackNote],
    tick_samples: u64,
    instrument: &str,
    role: &str,
    seg_prefix: &str,
    bar_ticks: u64,
    programs: &mut ProgramReg,
    section_end: u64,
) -> (Vec<LaneBar>, Vec<String>) {
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
        // Voice setup: every program binds once per song fn as a `let
        // voice_*` variable (never inline: full groups crowd bar lines);
        // tweaks of 2 or fewer keys stay inline diffs.
        let mut keys: Vec<&String> = note.params.iter().map(|(key, _)| key).collect();
        keys.sort();
        keys.dedup();
        let snapshot = program_snapshot(note);
        // One registry call per program head; the song tail binds each
        // used one as a `let voice_*` variable (never inline).
        if first {
            let voice = programs.intern(role, instrument, snapshot.clone());
            items.push(format!("{voice}()"));
            item_lens.push(0);
            for (key, value) in &snapshot {
                setup.insert(key.clone(), *value);
            }
            item_setups.push(setup.clone());
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
                let voice = programs.intern(role, instrument, snapshot.clone());
                items.push(format!("{voice}()"));
                item_lens.push(0);
                for (key, value) in &snapshot {
                    setup.insert(key.clone(), *value);
                }
                item_setups.push(setup.clone());
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
        // The head carries the FULL duration and tie pieces ride zero:
        // ties merge into their head at resolution (no independent
        // advance), so layout lengths match true time exactly. This keeps
        // phrase call lengths and bar spans truthful downstream.
        let base = format!("{name}{octave_str}{}", durations[0]);
        items.push(base);
        item_lens.push(dur_ticks);
        item_setups.push(setup.clone());
        for tie in &durations[1..] {
            items.push(tie.to_string());
            item_lens.push(0);
            item_setups.push(setup.clone());
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
    // Group singles, split bar-crossers, extract bar-local phrases.
    // Lengths thread through so the bar partition stays exact.
    let (items, item_setups, item_lens) = group_params(items, item_setups, item_lens);
    let (items, item_lens, item_setups) = split_spanning(items, item_lens, item_setups, bar_ticks);
    // Phrase extraction (multi-item repeats, bar-local), then an exact
    // bar partition: after splitting nothing crosses a barline, so
    // cumulative lengths give absolute bar numbers. Repeat runs compress
    // within each bar (never across: a run would hide the barline).
    // Each bar records its start setup (running state after the last
    // pre-bar item) for program restatement downstream.
    let (items, mut segs, item_lens, item_setups) =
        extract_phrases(items, item_setups, item_lens, seg_prefix, 8, bar_ticks);
    let mut bars: Vec<LaneBar> = Vec::new();
    let mut position = 0u64;
    let mut current_bar = 0u64;
    let mut current: Vec<String> = Vec::new();
    let mut current_lens: Vec<u64> = Vec::new();
    let mut current_start_setup: HashMap<String, f32> = HashMap::new();
    let mut running_setup: HashMap<String, f32> = HashMap::new();
    let mut started = false;
    // Flush the open bar (compressed) with its recorded start setup.
    // Lengths compress alongside so every bar keeps exact tick spans.
    for (item, (len, setup)) in items.iter().zip(item_lens.iter().zip(item_setups.iter())) {
        // Zero-len bare ties merge BACKWARD into their head's atom at
        // resolution, so they join the head's bar even when sitting
        // exactly on a barline (forward assignment would strand them at a
        // fresh ser! start and panic). All other items assign by position
        // (params/voice calls apply forward to following notes).
        let bar = if bar_ticks > 0 && *len == 0 && is_bare_tie(item) && started {
            current_bar
        } else if bar_ticks > 0 {
            position / bar_ticks
        } else {
            0
        };
        if !started {
            current_bar = bar;
            current_start_setup = running_setup.clone();
            started = true;
        } else if bar != current_bar {
            let (flushed, flushed_lens) = compress_runs(&current, &current_lens);
            bars.push(LaneBar {
                bar: current_bar,
                items: flushed,
                lens: flushed_lens,
                start_setup: std::mem::take(&mut current_start_setup),
            });
            current = Vec::new();
            current_lens = Vec::new();
            // Fill wholly-silent bars so numbering stays exact.
            let mut missing = current_bar + 1;
            while missing < bar {
                let (fill_items, fill_lens) = full_bar_pieces(bar_ticks);
                bars.push(LaneBar {
                    bar: missing,
                    items: fill_items,
                    lens: fill_lens,
                    start_setup: running_setup.clone(),
                });
                missing += 1;
            }
            current_bar = bar;
            current_start_setup = running_setup.clone();
        }
        current.push(item.clone());
        current_lens.push(*len);
        running_setup = setup.clone();
        position += len;
    }
    if !current.is_empty() || bars.is_empty() {
        let (flushed, flushed_lens) = compress_runs(&current, &current_lens);
        bars.push(LaneBar {
            bar: current_bar,
            items: flushed,
            lens: flushed_lens,
            start_setup: current_start_setup,
        });
    }
    segs.sort();
    (bars, segs)
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
        || item.starts_with("repeat!(")
        || item.starts_with("legato!(")
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
) -> (
    Vec<String>,
    Vec<String>,
    Vec<u64>,
    Vec<HashMap<String, f32>>,
) {
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
        // Rebuild: splice calls at fitting occurrence positions except the
        // first. A call must fit its bar (bar-major pars advance exactly
        // one bar); crossing occurrences stay inline and partition by
        // position like any items.
        let pos = item_positions(&lens);
        let fits = |at: usize| same_bar(pos[at], call_len, bar_ticks);
        let mut fitting_calls = 0;
        {
            let mut k = 0;
            let mut seen_first = false;
            while k + len <= items.len() {
                if items[k..k + len] == items[first..first + len] && setups[k] == setups[first] {
                    if !seen_first && k == first {
                        seen_first = true;
                    } else if fits(k) {
                        fitting_calls += 1;
                    }
                    k += len;
                } else {
                    k += 1;
                }
            }
        }
        // A breakout with no fittable call is pure overhead.
        if fitting_calls == 0 {
            segs.pop();
            break;
        }
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
                } else if fits(k) {
                    next_items.push(call.clone());
                    // Post-call ambient is the phrase's END state, not its
                    // start: later diffs compare against it.
                    next_setups.push(apply_items(&setups[k], &items[k..k + len]));
                    next_lens.push(call_len);
                } else {
                    next_items.extend(items[k..k + len].iter().cloned());
                    next_setups.extend(setups[k..k + len].iter().cloned());
                    next_lens.extend(lens[k..k + len].iter().cloned());
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
    (items, segs, lens, setups)
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

/// True when a channel-bar needs program restatement: it holds a note
/// head, legato sustain, or seg call (any NoteOn source). Rest heads and
/// zero-len items (params, voice calls, comments, repeats, bare ties)
/// need no program; voices ignore rests.
fn bar_sounds(items: &[String]) -> bool {
    items.iter().any(|item| {
        item.starts_with("legato!(")
            || (item.ends_with("()") && !item.starts_with("param!(") && !item.starts_with("voice_"))
            || parse_sounding(item)
                .map(|(pitch, _, _)| pitch.is_some())
                .unwrap_or(false)
    })
}

/// Program restatement opening a sounding channel-bar: the `voice()`
/// call for the bar-start snapshot (interned on demand) plus a velocity
/// param when it differs from default. Empty setups need nothing: the
/// bar's own items establish everything (lane heads). Voice calls use
/// the `{name}()` form here; the downstream let/inline pass rewrites them.
/// Time-aligned bar columns: one `ser!` body per channel with items
/// starting at the same tick in the same column, so rhythm reads
/// vertically like staff systems. Subdivides the bar at every segment
/// start, sizes each slice by its widest content (plus one gap), pads
/// the rest. Zero-length items glue forward (params/voice shape what
/// follows) except bare ties (merge backward into their head) and
/// `repeat!` (extends the repeated hit). Whitespace-only change: the
/// note stream resolves identically.
fn align_bar(channels: &[Vec<String>], lens: &[Vec<u64>], bar_ticks: u64) -> Vec<String> {
    struct Seg {
        start: u64,
        len: u64,
        text: String,
    }
    let mut all: Vec<Vec<Seg>> = Vec::new();
    for (items, ls) in channels.iter().zip(lens.iter()) {
        let mut segs: Vec<Seg> = Vec::new();
        let mut pending: Vec<String> = Vec::new();
        let mut pos = 0u64;
        for (item, len) in items.iter().zip(ls.iter()) {
            if *len == 0 && is_bare_tie(item) {
                // Ties merge backward into their head (resolution glues
                // them there too).
                match segs.last_mut() {
                    Some(prev) => {
                        prev.text.push(' ');
                        prev.text.push_str(item);
                    }
                    None => pending.push(item.clone()),
                }
            } else if item.starts_with("repeat!(") {
                // Repeats extend the hit they follow (elided ticks ride
                // the marker); other channels' starts inside still slice.
                match segs.last_mut() {
                    Some(prev) => {
                        prev.text.push(' ');
                        prev.text.push_str(item);
                        prev.len += len;
                    }
                    None => {
                        let mut text = pending.join(" ");
                        pending.clear();
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(item);
                        segs.push(Seg {
                            start: pos,
                            len: *len,
                            text,
                        });
                        pos += len;
                    }
                }
            } else if *len == 0 {
                pending.push(item.clone());
            } else {
                let mut text = pending.join(" ");
                pending.clear();
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(item);
                segs.push(Seg {
                    start: pos,
                    len: *len,
                    text,
                });
                pos += len;
            }
        }
        if !pending.is_empty() {
            // Defensive: trailing setup with nothing following (emission
            // always leaves a sounding item last); glue backward.
            match segs.last_mut() {
                Some(prev) => {
                    prev.text.push(' ');
                    prev.text.push_str(&pending.join(" "));
                }
                None => segs.push(Seg {
                    start: 0,
                    len: 0,
                    text: pending.join(" "),
                }),
            }
        }
        all.push(segs);
    }
    if all.iter().all(|segs| segs.is_empty()) {
        return channels.iter().map(|items| items.join(" ")).collect();
    }
    // Slice at every segment start; the span end closes the last slice.
    let mut bounds: Vec<u64> = vec![0];
    let mut end = bar_ticks;
    for segs in &all {
        for seg in segs {
            bounds.push(seg.start);
            end = end.max(seg.start + seg.len);
        }
    }
    bounds.push(end);
    bounds.sort();
    bounds.dedup();
    let nslices = bounds.len().saturating_sub(1);
    let slice_of = |start: u64| bounds.iter().position(|b| *b == start).unwrap_or(0);
    let mut widths = vec![0usize; nslices];
    for segs in &all {
        for seg in segs {
            if let Some(width) = widths.get_mut(slice_of(seg.start)) {
                *width = (*width).max(seg.text.len());
            }
        }
    }
    for (j, width) in widths.iter_mut().enumerate() {
        if j + 1 < nslices {
            *width += 1;
        }
    }
    let mut bodies = Vec::new();
    for segs in &all {
        let mut line = String::new();
        for j in 0..nslices {
            let text = segs
                .iter()
                .find(|seg| slice_of(seg.start) == j)
                .map(|seg| seg.text.as_str())
                .unwrap_or("");
            line.push_str(text);
            if j + 1 < nslices {
                for _ in text.len()..widths[j] {
                    line.push(' ');
                }
            }
        }
        bodies.push(line.trim_end().to_string());
    }
    bodies
}

fn restate_bar(
    reg: &mut ProgramReg,
    role: &str,
    instrument: &str,
    start_setup: &HashMap<String, f32>,
    bar_items: &[String],
) -> Vec<String> {
    if start_setup.is_empty() {
        return Vec::new();
    }
    // Snapshot pairs are the setup minus velocity (instrument rides the key).
    let mut snapshot: Vec<(String, f32)> = start_setup
        .iter()
        .filter(|(key, _)| key.as_str() != "velocity")
        .map(|(key, value)| (key.clone(), *value))
        .collect();
    snapshot.sort_by(|a, b| a.0.cmp(&b.0));
    let voice = reg.intern(role, instrument, snapshot);
    let call = format!("{voice}()");
    let mut prefix = Vec::new();
    // The bar's own head program already establishes everything.
    let opens_with_program = bar_items
        .first()
        .map(|first| first == &call || first.starts_with("param!(instrument="))
        .unwrap_or(false);
    if !opens_with_program {
        prefix.push(call);
    }
    if let Some(velocity) = start_setup.get("velocity") {
        if (velocity - 100.0).abs() > 1e-6 {
            prefix.push(format!("param!(velocity={velocity})"));
        }
    }
    prefix
}

/// One assembled bar: score-ordered channel items with exact tick
/// lengths, pre-substitution (`voice()` calls, literals). Alignment and
/// all text substitutions run on this structure so every width they
/// change is accounted before columns are laid out.
struct AsmCh {
    role: String,
    instrument: String,
    items: Vec<String>,
    lens: Vec<u64>,
}
struct AsmBar {
    no: u64,
    channels: Vec<AsmCh>,
}

/// Substring occurrences of `pat` across structured bars (matches never
/// span items, so this equals the old whole-mix text count).
fn bar_uses(bars: &[AsmBar], pat: &str) -> usize {
    bars.iter()
        .flat_map(|bar| bar.channels.iter())
        .flat_map(|ch| ch.items.iter())
        .map(|item| item.matches(pat).count())
        .sum()
}

/// First item containing `pat` (bar-major order) gets one replacement.
fn replace_first_in_bars(bars: &mut [AsmBar], pat: &str, rep: &str) -> bool {
    for bar in bars {
        for ch in &mut bar.channels {
            for item in &mut ch.items {
                if item.contains(pat) {
                    *item = item.replacen(pat, rep, 1);
                    return true;
                }
            }
        }
    }
    false
}

/// Every occurrence in every item replaced; true when anything matched.
fn replace_all_in_bars(bars: &mut [AsmBar], pat: &str, rep: &str) -> bool {
    let mut hit = false;
    for bar in bars {
        for ch in &mut bar.channels {
            for item in &mut ch.items {
                if item.contains(pat) {
                    *item = item.replace(pat, rep);
                    hit = true;
                }
            }
        }
    }
    hit
}

/// Render structured bars to bar-major text, laying time-aligned columns
/// per bar (labels on the first system only, sheet-music convention).
fn render_section(bars: &[AsmBar], bar_ticks: u64) -> String {
    let mut bar_texts: Vec<String> = Vec::new();
    for (bar_idx, bar) in bars.iter().enumerate() {
        let ch_items: Vec<Vec<String>> = bar.channels.iter().map(|ch| ch.items.clone()).collect();
        let ch_lens: Vec<Vec<u64>> = bar.channels.iter().map(|ch| ch.lens.clone()).collect();
        let bodies = align_bar(&ch_items, &ch_lens, bar_ticks);
        let mut channels: Vec<String> = Vec::new();
        for (body, ch) in bodies.iter().zip(bar.channels.iter()) {
            if bar_idx == 0 {
                channels.push(format!(
                    "            // {} ({})\n            track!({body})",
                    ch.role, ch.instrument
                ));
            } else {
                channels.push(format!("            track!({body})"));
            }
        }
        bar_texts.push(format!(
            "        bar!( // bar {}\n{},\n        )",
            bar.no + 1,
            channels.join(",\n")
        ));
    }
    bar_texts.join(",\n")
}

/// Full song: intro + loop voices as bar-major scores — outer `ser!` of
/// per-bar `par!`s, each a score-ordered stack of channel `ser!`s.
/// `voices`: (instrument, role, intro notes, loop notes). `tempo` heads
/// the mix; `bar_ticks` is grid ticks per bar for the layout.
/// Voice programs bind once per song fn as `let voice_*` variables
/// (`voice.clone()` splices; never inline).
/// Every sounding channel restates its program per bar (par branches
/// reset ambient); rest-only bars carry bare rests. Within a bar, notes
/// align vertically by point in time (whitespace only).
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
    let mut intro_lanes: Vec<(String, String, Vec<LaneBar>)> = Vec::new();
    let mut loop_lanes: Vec<(String, String, Vec<LaneBar>)> = Vec::new();
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
        let (intro_bars, mut intro_segs) = emit_voice(
            intro,
            tick_samples,
            instrument,
            role,
            &format!("{role}i"),
            bar_ticks,
            reg,
            intro_end,
        );
        let (loop_bars, mut loop_segs) = emit_voice(
            looping,
            tick_samples,
            instrument,
            role,
            &format!("{role}l"),
            bar_ticks,
            reg,
            loop_end,
        );
        intro_lanes.push((role.clone(), instrument.clone(), intro_bars));
        loop_lanes.push((role.clone(), instrument.clone(), loop_bars));
        seg_defs.append(&mut intro_segs);
        seg_defs.append(&mut loop_segs);
    }
    // Bar-major assembly (structured): outer ser of per-bar pars, each a
    // score-ordered stack of channel sers. Par branches reset ambient, so
    // every sounding channel restates its program (voice call + velocity);
    // rest-only bars need nothing (voices ignore rests). Stays structured
    // through every text substitution below; columns lay out last.
    fn assemble_section(
        lanes: &[(String, String, Vec<LaneBar>)],
        regs: &mut [(String, ProgramReg)],
        bar_ticks: u64,
    ) -> Vec<AsmBar> {
        let nbars = lanes
            .iter()
            .map(|(_, _, bars)| bars.len())
            .max()
            .unwrap_or(0);
        let mut bars: Vec<AsmBar> = Vec::new();
        for bar_idx in 0..nbars {
            let mut channels: Vec<AsmCh> = Vec::new();
            for (role, instrument, lane_bars) in lanes {
                let reg = regs
                    .iter_mut()
                    .find(|(name, _)| name == role)
                    .map(|(_, reg)| reg)
                    .expect("registry per role");
                let (items, lens, start_setup) = match lane_bars.get(bar_idx) {
                    Some(bar) => (bar.items.clone(), bar.lens.clone(), bar.start_setup.clone()),
                    None => {
                        let (fill_items, fill_lens) = full_bar_pieces(bar_ticks);
                        (fill_items, fill_lens, HashMap::new())
                    }
                };
                let mut full_items = if bar_sounds(&items) {
                    restate_bar(reg, role, instrument, &start_setup, &items)
                } else {
                    Vec::new()
                };
                let mut full_lens = vec![0u64; full_items.len()];
                full_items.extend(items);
                full_lens.extend(lens);
                channels.push(AsmCh {
                    role: role.clone(),
                    instrument: instrument.clone(),
                    items: full_items,
                    lens: full_lens,
                });
            }
            let no = lanes
                .iter()
                .filter_map(|(_, _, lane_bars)| lane_bars.get(bar_idx).map(|bar| bar.bar))
                .next()
                .unwrap_or(bar_idx as u64);
            bars.push(AsmBar { no, channels });
        }
        bars
    }
    // Assemble bar-major bars (interning restatement programs on demand),
    // then collect the full program list including those.
    let mut intro_bars = assemble_section(&intro_lanes, &mut regs, bar_ticks);
    let mut loop_bars = assemble_section(&loop_lanes, &mut regs, bar_ticks);
    // Staff order headline (first-system labels live on bar 1).
    let staff_order = ordered
        .iter()
        .map(|(instrument, role, _, _)| format!("{role} ({instrument})"))
        .collect::<Vec<_>>()
        .join(", ");
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
    // Bar programs all bind once per song fn (`let voice: Note`) and
    // splice bare (blocks borrow items, cloning inside) — never inline,
    // so bar lines stay readable. Runs on structured bars so the final
    // widths feed column layout below.
    let mut intro_lets: Vec<String> = Vec::new();
    let mut loop_lets: Vec<String> = Vec::new();
    for (_, name, snapshot, instrument) in &programs {
        let pat = format!("{name}()");
        let group = program_param_group(instrument, snapshot);
        let binding = format!("let {name}: Note = {group};");
        let use_site = name.clone();
        if replace_all_in_bars(&mut intro_bars, pat.as_str(), use_site.as_str()) {
            intro_lets.push(binding.clone());
        }
        if replace_all_in_bars(&mut loop_bars, pat.as_str(), use_site.as_str()) {
            loop_lets.push(binding);
        }
    }
    // One voice roster shared by intro and loop (both yields borrow it),
    // in registry order.
    let mut voice_lets: Vec<String> = Vec::new();
    for binding in intro_lets.into_iter().chain(loop_lets) {
        if !voice_lets.contains(&binding) {
            voice_lets.push(binding);
        }
    }
    let voice_lets_block = if voice_lets.is_empty() {
        String::new()
    } else {
        format!("    // voices\n    {}\n", voice_lets.join("\n    "))
    };
    // Recurring velocity levels become named dynamics constants
    // (`param!(velocity=VEL_F)`); rarer ones stay inline literals.
    // Same recurrence philosophy as voices/segs; replacement is
    // bit-exact (matched literal text, never reformatted floats).
    let vel_consts_block = {
        fn scan_velocities(text: &str, freq: &mut HashMap<String, usize>) {
            let mut rest = text;
            while let Some(pos) = rest.find("velocity=") {
                let num_start = pos + "velocity=".len();
                let num_len = rest[num_start..]
                    .bytes()
                    .take_while(|b| b.is_ascii_digit() || *b == b'.')
                    .count();
                if num_len > 0 {
                    *freq
                        .entry(rest[num_start..num_start + num_len].to_string())
                        .or_default() += 1;
                }
                rest = &rest[num_start.max(pos + 1)..];
            }
        }
        fn replace_velocity(text: &mut String, lit: &str, name: &str) {
            let pat = format!("velocity={lit}");
            let mut out = String::with_capacity(text.len());
            let mut rest = text.as_str();
            while let Some(pos) = rest.find(&pat) {
                let after = pos + pat.len();
                let boundary = rest[after..]
                    .chars()
                    .next()
                    .map(|c| !c.is_ascii_digit() && c != '.')
                    .unwrap_or(true);
                out.push_str(&rest[..pos]);
                if boundary {
                    out.push_str("velocity=");
                    out.push_str(name);
                } else {
                    out.push_str(&pat);
                }
                rest = &rest[after..];
            }
            out.push_str(rest);
            *text = out;
        }
        // Relative dynamics by loudness fraction (documented per const
        // with its exact fraction + source nibble).
        fn dynamics_name(frac: f32) -> &'static str {
            if frac < 0.15 {
                "VEL_PPP"
            } else if frac < 0.45 {
                "VEL_P"
            } else if frac < 0.57 {
                "VEL_MP"
            } else if frac < 0.63 {
                "VEL_MF"
            } else if frac < 0.70 {
                "VEL_F"
            } else if frac < 0.77 {
                "VEL_FF"
            } else {
                "VEL_FFF"
            }
        }
        let mut freq: HashMap<String, usize> = HashMap::new();
        // Scan structured bar items plus seg defs (mix syntax carries no
        // velocities, so this matches the old whole-mix scan exactly).
        let scan_bars = |bars: &[AsmBar], freq: &mut HashMap<String, usize>| {
            for bar in bars {
                for ch in &bar.channels {
                    for item in &ch.items {
                        scan_velocities(item, freq);
                    }
                }
            }
        };
        scan_bars(&intro_bars, &mut freq);
        scan_bars(&loop_bars, &mut freq);
        for seg in &seg_defs {
            scan_velocities(seg, &mut freq);
        }
        // Frequent first; a band name goes to its first claimant, later
        // same-band values stay inline (exactness over naming).
        let mut lits: Vec<(f32, String)> = freq
            .into_iter()
            .filter(|(_, count)| *count >= 3)
            .filter_map(|(lit, _)| lit.parse::<f32>().ok().map(|v| (v, lit)))
            .collect();
        lits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut claimed: HashMap<&str, String> = HashMap::new();
        let mut named: Vec<(f32, String, String)> = Vec::new();
        for (value, lit) in &lits {
            let band = dynamics_name(value / 127.0);
            if claimed.contains_key(band) {
                continue;
            }
            claimed.insert(band, lit.clone());
            named.push((*value, lit.clone(), band.to_string()));
        }
        for (_, lit, name) in &named {
            for bar in [&mut intro_bars, &mut loop_bars] {
                for ch in bar.iter_mut().flat_map(|bar| bar.channels.iter_mut()) {
                    for item in ch.items.iter_mut() {
                        replace_velocity(item, lit, name);
                    }
                }
            }
            for seg in seg_defs.iter_mut() {
                replace_velocity(seg, lit, name);
            }
        }
        if named.is_empty() {
            String::new()
        } else {
            let mut lines =
                vec!["// Voice dynamics (PSG nibble loudness as MIDI velocity).".to_string()];
            for (value, lit, name) in &named {
                let nibble = (value / 127.0 * 15.0).round();
                let note = if (nibble / 15.0 * 127.0 * 100.0).round() / 100.0 == *value {
                    format!("{}/15", nibble as i64)
                } else {
                    format!("{:.2}", value / 127.0)
                };
                lines.push(format!("const {name}: f32 = {lit}; // {note}"));
            }
            lines.join("\n") + "\n\n"
        }
    };
    // Seg phrases called exactly once inline as nested `ser!` blocks and
    // drop their definitions (same single-use rule as voices). Fixpoint:
    // inlining can strand nested single-call segs; unreferenced defs drop.
    loop {
        // Parse current defs into (name, one-line ser expr).
        let mut defs: Vec<(String, String)> = Vec::new();
        for def in &seg_defs {
            let Some(fn_pos) = def.find("fn ") else {
                continue;
            };
            let rest = &def[fn_pos + 3..];
            let Some(paren) = rest.find("()") else {
                continue;
            };
            let seg_name = rest[..paren].to_string();
            let Some(ser_pos) = def.find("ser!(\n") else {
                continue;
            };
            let Some(end_pos) = def.rfind("\n    )\n}") else {
                continue;
            };
            let body = def[ser_pos + "ser!(\n".len()..end_pos]
                .lines()
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(" ");
            defs.push((seg_name, format!("ser!({body})")));
        }
        // Names referenced anywhere (bars + seg bodies), for dead-def GC.
        // Matches never span items, so structured counts equal text counts.
        let uses_of = |seg_name: &str| {
            let pat = format!("{seg_name}()");
            let mut n = bar_uses(&intro_bars, pat.as_str()) + bar_uses(&loop_bars, pat.as_str());
            for def in &seg_defs {
                n += def.matches(pat.as_str()).count();
            }
            n.saturating_sub(1)
        };
        let singles: Vec<(String, String)> = defs
            .iter()
            .filter(|(seg_name, _)| uses_of(seg_name) == 1)
            .cloned()
            .collect();
        if singles.is_empty() {
            let live: std::collections::HashSet<String> = defs
                .iter()
                .filter(|(seg_name, _)| uses_of(seg_name) > 0)
                .map(|(seg_name, _)| seg_name.clone())
                .collect();
            seg_defs.retain(|def| {
                let Some(fn_pos) = def.find("fn ") else {
                    return true;
                };
                let rest = &def[fn_pos + 3..];
                let Some(paren) = rest.find("()") else {
                    return true;
                };
                live.contains(&rest[..paren])
            });
            break;
        }
        for (seg_name, expr) in &singles {
            let pat = format!("{seg_name}()");
            // The nested expr keeps its call site's item slot (whose tick
            // length already equals the expansion), so layout stays exact.
            if !replace_first_in_bars(&mut intro_bars, pat.as_str(), expr) {
                if !replace_first_in_bars(&mut loop_bars, pat.as_str(), expr) {
                    for def in seg_defs.iter_mut() {
                        if def.contains(pat.as_str()) {
                            *def = def.replacen(pat.as_str(), expr, 1);
                            break;
                        }
                    }
                }
            }
        }
        seg_defs.retain(|def| {
            let Some(fn_pos) = def.find("fn ") else {
                return true;
            };
            let rest = &def[fn_pos + 3..];
            let Some(paren) = rest.find("()") else {
                return true;
            };
            !singles
                .iter()
                .any(|(seg_name, _)| seg_name == &rest[..paren])
        });
    }
    // Render bar-major mixes last, after every substitution, so column
    // layout sees final widths. Loop bars sit one level deeper (inside
    // `loop { yield_!(`).
    let intro_mix = render_section(&intro_bars, bar_ticks);
    let loop_mix = render_section(&loop_bars, bar_ticks);
    let loop_indented = if loop_mix.is_empty() {
        String::new()
    } else {
        format!("    {}", loop_mix.replace('\n', "\n    "))
    };
    format!(
        "/// Decompiled `{name}` (tempo {tempo}, bar = {bar_ticks} ticks).\n\
         /// One performance: intro once, then the loop body forever.\n\
         /// Bar-major score: outer `ser!` of `bar!` bars, each a stack of\n\
         /// channel `track!`s in score order ({staff_order}); every sounding\n\
         /// channel restates its voice (`#[rustfmt::skip]` keeps it).\n\
         /// Each `bar!` outlines into its own closure for parallel codegen;\n\
         /// Voice programs bind once as `let voice_*` variables (spliced
         /// bare, never inline); `seg_*()` phrases are bar-local repeats
         /// (single-call segs inline too); cross-bar sustains are `legato!`
         /// plus rest cover.\n\
         /// One generator fn streams the whole performance: intro `yield_!`
         /// once, then `loop` yielding the loop body forever. Voices and
         /// music read top-to-bottom in one flow; `gen!`/`yield_!` come
         /// from our vendored genawaiter (de-hacked for rust-analyzer).\n\
         use genawaiter::sync::gen;\n\
         use genawaiter::yield_;\n\
         use mmlx_core::prelude::*;\n\
         \n\
         {velconsts}\
         {segs}\
         #[rustfmt::skip]\n\
         pub fn {name}() -> NoteIterator {{\n\
         \x20   Box::new(gen!({{\n\
         {voicelets}\
         \x20   yield_!(ser!(\n\
         \x20       param!(tempo={tempo}),\n\
         {intro}\n\
         \x20   ));\n\
         \x20   loop {{\n\
         \x20       yield_!(ser!(\n\
         \x20           param!(tempo={tempo}),\n\
         {lp}\n\
         \x20       ));\n\
         \x20   }}\n\
         \x20   }}).into_iter())\n\
         }}",
        segs = if seg_defs.is_empty() {
            String::new()
        } else {
            seg_defs.join("\n\n") + "\n\n"
        },
        velconsts = vel_consts_block,
        intro = intro_mix,
        lp = loop_indented,
        voicelets = voice_lets_block,
    )
}
