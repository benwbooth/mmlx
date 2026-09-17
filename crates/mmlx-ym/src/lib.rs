//! Exact YM2612 streaming voice over Nuked-OPN2 (GME's backend).
//!
//! [`Ym2612`] owns the emulated chip (reset/register writes/stereo render).
//! [`Ym2612Voice`] is an [`Instrument`]: each note allocates one of the six
//! FM channels and programs it **entirely from mmlx params** — `fm_routing`
//! (mapped to the YM algorithm), `fm_feedback`, per-operator
//! `op{i}_ratio op{i}_level op{i}_attack op{i}_decay op{i}_sustain
//! op{i}_release`, `pan`, plus `ym_channel` to pin a channel (else first
//! free). No hidden state: the complete FM program is in the event stream.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;
use std::os::raw::c_void;

unsafe extern "C" {
    fn mmlx_nuked_new() -> *mut c_void;
    fn mmlx_nuked_free(chip: *mut c_void);
    fn mmlx_nuked_set_rate(chip: *mut c_void, sample_rate: f64, clock_rate: f64) -> i32;
    fn mmlx_nuked_reset(chip: *mut c_void);
    fn mmlx_nuked_write(chip: *mut c_void, port: i32, addr: i32, data: i32);
    fn mmlx_nuked_mute(chip: *mut c_void, mask: i32);
    fn mmlx_nuked_run(chip: *mut c_void, frames: i32, out_stereo: *mut i16);
}

/// Genesis NTSC master clock driving the YM2612.
pub const YM2612_CLOCK_NTSC: u32 = 7_670_453;

/// Our routing (0-8) back to YM2612 algorithms (0-7).
pub const ROUTING_TO_YM_ALGO: [u8; 9] = [0, 4, 5, 7, 0, 1, 2, 3, 6];

/// Register rows (+0/+4/+8/+12) hold slots S1/S3/S2/S4 (Plutiedev-confirmed).
const ROW_TO_SLOT: [usize; 4] = [0, 2, 1, 3];

fn number(parameters: &HashMap<String, ParamValue>, key: &str) -> Option<f32> {
    parameters.get(key).and_then(|value| match value {
        ParamValue::Number(number) => Some(*number),
        _ => None,
    })
}

/// Cycle-accurate YM2612 (Nuked-OPN2 via GME's backend): the same DSP
/// the GME audit reference renders, so per-lane A/B compares like with
/// like. Resamples internally to any rate; needs no DC strip.
pub struct Ym2612 {
    chip: *mut c_void,
    rate: u32,
}

// The chip is only touched from the render path.
unsafe impl Send for Ym2612 {}

impl Ym2612 {
    pub fn new(clock: u32, rate: u32) -> Option<Self> {
        unsafe {
            let chip = mmlx_nuked_new();
            if chip.is_null() {
                None
            } else if mmlx_nuked_set_rate(chip, rate as f64, clock as f64) != 0 {
                mmlx_nuked_free(chip);
                None
            } else {
                mmlx_nuked_reset(chip);
                Some(Ym2612 { chip, rate })
            }
        }
    }

    pub fn reset(&mut self) {
        unsafe { mmlx_nuked_reset(self.chip) };
    }

    /// Raw register write: VGM 0x52 = port 0, 0x53 = port 1.
    pub fn write(&mut self, port: u8, addr: u8, data: u8) {
        unsafe {
            mmlx_nuked_write(self.chip, port as i32, addr as i32, data as i32);
        }
    }

    /// Mute chip channels by bitmask (1 << channel).
    pub fn set_mute(&mut self, mask: u8) {
        unsafe { mmlx_nuked_mute(self.chip, mask as i32) };
    }

    /// Render interleaved stereo float in [-1, 1].
    pub fn render(&mut self, frames: usize) -> Vec<[f32; 2]> {
        let mut pcm = vec![0i16; frames * 2];
        unsafe {
            mmlx_nuked_run(self.chip, frames as i32, pcm.as_mut_ptr());
        }
        pcm.chunks_exact(2)
            .map(|s| [s[0] as f32 / 32768.0, s[1] as f32 / 32768.0])
            .collect()
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }
}

impl Drop for Ym2612 {
    fn drop(&mut self) {
        unsafe { mmlx_nuked_free(self.chip) };
    }
}

/// MIDI note to (block, fnum11) for the YM2612 frequency registers.
/// Calibrated: A440 renders at 440 Hz on the Nuked core.
pub fn midi_to_fnum(midi: u8, clock: u32) -> (u8, u16) {
    let freq = 440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0);
    for block in 0..=7u8 {
        let fnum = (freq * 2.0f32.powi(21 - block as i32) * 144.0 / clock as f32).round();
        if fnum <= 0x7FF as f32 {
            return (block, fnum.max(1.0) as u16);
        }
    }
    (7, 0x7FF)
}

/// Chip frequency for (block, fnum) — the inverse used for verification.
pub fn fnum_to_freq(block: u8, fnum: u16, clock: u32) -> f32 {
    fnum as f32 * (clock as f32 / 144.0) / 2.0f32.powi(21 - block as i32)
}

/// Linear 0-1 gain to YM total level (0.75 dB steps).
pub fn gain_to_tl(gain: f32) -> u8 {
    if gain <= 0.0001 {
        127
    } else {
        (-20.0 * gain.log10() / 0.75).round().clamp(0.0, 127.0) as u8
    }
}

/// Seconds to YM rate (attack/decay share the 1.5/3.0-base curves).
pub fn secs_to_rate(secs: f32, base: f32, max: u8) -> u8 {
    if secs <= 0.0 {
        31.min(max)
    } else {
        (-4.0 * (secs / base).log2()).round().clamp(0.0, max as f32) as u8
    }
}

/// One programmed FM channel on the chip.
pub struct Ym2612Voice {
    chip: Ym2612,
    /// note_id -> chip channel (0-5).
    active: HashMap<u64, u8>,
    /// Per-channel legato state: current occupant, last full program, and
    /// when it was last freed. An abutting same-program NoteOn glides
    /// (retune only) instead of re-keying, so pitch slides render smooth
    /// instead of machine-gunning the envelope.
    occupant: [Option<u64>; 6],
    program: [HashMap<String, ParamValue>; 6],
    freed_at: [f32; 6],
    has_history: [bool; 6],
}

/// Notes abutting within ~4 samples count as legato (same tick grid).
const LEGATO_EPS: f32 = 0.0001;

/// Parameter keys that ride the ambient setup but never define the voice
/// program (loudness, clock, glide markers): legato compares the rest.
fn is_program_key(key: &str) -> bool {
    !matches!(
        key,
        "tied" | "velocity" | "tempo" | "time_note" | "time_beat"
    )
}

fn same_program(a: &HashMap<String, ParamValue>, b: &HashMap<String, ParamValue>) -> bool {
    fn filtered(params: &HashMap<String, ParamValue>) -> Vec<(&String, &ParamValue)> {
        params
            .iter()
            .filter(|(key, _)| is_program_key(key))
            .collect::<Vec<_>>()
    }
    let (mut left, mut right) = (filtered(a), filtered(b));
    left.sort_by(|x, y| x.0.cmp(y.0));
    right.sort_by(|x, y| x.0.cmp(y.0));
    left == right
}

// The chip is only touched from the render path.
unsafe impl Send for Ym2612Voice {}
unsafe impl Sync for Ym2612Voice {}

impl Ym2612Voice {
    pub fn new(clock: u32, rate: u32) -> Option<Self> {
        Ym2612::new(clock, rate).map(|chip| Ym2612Voice {
            chip,
            active: HashMap::new(),
            occupant: [None; 6],
            program: Default::default(),
            freed_at: [f32::NEG_INFINITY; 6],
            has_history: [false; 6],
        })
    }

    fn channel_regs(channel: u8) -> (u8, u8) {
        // (port, base): channels 0-2 on port 0, 3-5 on port 1.
        if channel < 3 {
            (0, channel)
        } else {
            (1, channel - 3)
        }
    }

    /// Program a channel fully from params and key it on.
    fn key_on(&mut self, channel: u8, midi: u8, parameters: &HashMap<String, ParamValue>) {
        let num = |key: &str| number(parameters, key);
        let (port, base) = Self::channel_regs(channel);
        // Raw YM program wins when present (decompiler-exact path);
        // otherwise cooked mmlx FM params are translated.
        let algo = num("ym_algo")
            .map(|algo| algo.round().clamp(0.0, 7.0) as u8)
            .unwrap_or_else(|| {
                let routing = num("fm_routing").unwrap_or(0.0).round().clamp(0.0, 8.0) as usize;
                ROUTING_TO_YM_ALGO[routing]
            });
        let feedback = num("ym_feedback")
            .map(|feedback| feedback.round().clamp(0.0, 7.0) as u8)
            .unwrap_or_else(|| {
                (num("fm_feedback").unwrap_or(0.0).clamp(0.0, 1.0) * 7.0).round() as u8
            });
        self.chip.write(port, 0xB0 + base, (feedback << 3) | algo);
        let pan = num("pan").unwrap_or(0.5);
        let stereo = if pan < 0.25 {
            0x80
        } else if pan > 0.75 {
            0x40
        } else {
            0xC0
        };
        self.chip.write(port, 0xB4 + base, stereo);
        // Operators in slot order S1..S4; rows are +0/+4/+8/+12 = S1/S3/S2/S4.
        for slot in 0..4 {
            let row = ROW_TO_SLOT
                .iter()
                .position(|slot_row| *slot_row == slot)
                .unwrap() as u8;
            let prefix = format!("op{}", slot + 1);
            let field = |key: &str| num(&format!("{prefix}_{key}"));
            // Raw register values win when present (decompiler-exact path).
            let raw =
                |key: &str, mask: f32| field(key).map(|value| value.round().clamp(0.0, mask) as u8);
            let mult_reg = raw("mult", 15.0).unwrap_or_else(|| {
                let mult = field("ratio").unwrap_or(1.0);
                if mult <= 0.75 {
                    0
                } else {
                    mult.round().clamp(1.0, 15.0) as u8
                }
            });
            // DT lives in the MUL high nibble (bits 4-6).
            let detune = raw("dt", 7.0).unwrap_or(0);
            self.chip
                .write(port, 0x30 + row * 4 + base, mult_reg & 0x0F | (detune << 4));
            let tl = raw("tl", 127.0)
                .unwrap_or_else(|| gain_to_tl(field("level").unwrap_or(0.8).clamp(0.0, 1.0)));
            self.chip.write(port, 0x40 + row * 4 + base, tl);
            let attack = raw("ar", 31.0)
                .unwrap_or_else(|| secs_to_rate(field("attack").unwrap_or(0.01).max(0.0), 1.5, 31));
            self.chip.write(port, 0x50 + row * 4 + base, attack);
            let decay = raw("dr", 31.0)
                .unwrap_or_else(|| secs_to_rate(field("decay").unwrap_or(0.1).max(0.0), 3.0, 31));
            self.chip.write(port, 0x60 + row * 4 + base, decay);
            // Sustain rate: hold unless programmed raw.
            let sustain_rate = raw("sr", 31.0).unwrap_or(0);
            self.chip.write(port, 0x70 + row * 4 + base, sustain_rate);
            let sustain_level = raw("sl", 15.0).unwrap_or_else(|| {
                ((1.0 - field("sustain").unwrap_or(0.9).clamp(0.0, 1.0)) * 15.0).round() as u8
            });
            let release_rate = raw("rr", 15.0)
                .unwrap_or_else(|| secs_to_rate(field("release").unwrap_or(0.1).max(0.0), 3.0, 15));
            self.chip.write(
                port,
                0x80 + row * 4 + base,
                (sustain_level << 4) | release_rate,
            );
            let ssg = raw("ssg", 15.0).unwrap_or(0);
            self.chip.write(port, 0x90 + row * 4 + base, ssg);
        }
        let (block, fnum) = match (num("ym_block"), num("ym_fnum")) {
            (Some(block), Some(fnum)) => (
                block.round().clamp(0.0, 7.0) as u8,
                fnum.round().clamp(0.0, 0x7FF as f32) as u16,
            ),
            _ => midi_to_fnum(midi, YM2612_CLOCK_NTSC),
        };
        self.retune(channel, block, fnum);
        // Key on, all slots.
        let code = (channel % 3) | ((channel / 3) << 2);
        self.chip.write(0, 0x28, 0xF0 | code);
    }

    /// Retune a sounding channel without touching its program or envelope
    /// (legato glide: pitch slides and tied phrases, no re-attack).
    fn retune(&mut self, channel: u8, block: u8, fnum: u16) {
        let (port, base) = Self::channel_regs(channel);
        self.chip
            .write(port, 0xA4 + base, (block << 3) | ((fnum >> 8) as u8 & 0x07));
        self.chip.write(port, 0xA0 + base, (fnum & 0xFF) as u8);
    }

    fn retune_midi(&mut self, channel: u8, midi: u8, parameters: &HashMap<String, ParamValue>) {
        let num = |key: &str| number(parameters, key);
        let (block, fnum) = match (num("ym_block"), num("ym_fnum")) {
            (Some(block), Some(fnum)) => (
                block.round().clamp(0.0, 7.0) as u8,
                fnum.round().clamp(0.0, 0x7FF as f32) as u16,
            ),
            _ => midi_to_fnum(midi, YM2612_CLOCK_NTSC),
        };
        self.retune(channel, block, fnum);
    }

    fn key_off(&mut self, channel: u8) {
        let code = (channel % 3) | ((channel / 3) << 2);
        self.chip.write(0, 0x28, code);
    }
}

impl Instrument for Ym2612Voice {
    fn process_event(&mut self, event: &TimedMusicalEvent, _rate: usize) {
        match &event.event {
            MusicalEventType::NoteOn {
                note_id,
                pitch_midi,
                parameters,
                ..
            } => {
                let channel = number(parameters, "ym_channel")
                    .map(|channel| (channel as u8).min(5))
                    .or_else(|| {
                        (0..6u8).find(|channel| !self.active.values().any(|used| used == channel))
                    });
                if let Some(channel) = channel {
                    let slot = channel as usize;
                    // Glide marker from the decompiler (FNUM-split slides
                    // and ties glide; keyed notes re-attack). Unmarked
                    // streams fall back to the abutment heuristic below.
                    let tied = match parameters.get("tied") {
                        Some(ParamValue::Number(value)) => value.round() as i64,
                        _ => -1,
                    };
                    if tied == 1 && self.has_history[slot] {
                        self.retune_midi(channel, *pitch_midi, parameters);
                    } else if tied == 0 {
                        self.key_on(channel, *pitch_midi, parameters);
                        self.program[slot] = parameters.clone();
                        self.has_history[slot] = true;
                    } else {
                        // Heuristic: still sounding, or freed this same tick,
                        // with an identical program (slides, ties) — glide
                        // the pitch instead of restarting the envelope.
                        let continuous = self.occupant[slot].is_some()
                            || (self.has_history[slot]
                                && (event.time_seconds - self.freed_at[slot]).abs() <= LEGATO_EPS);
                        if continuous && same_program(&self.program[slot], parameters) {
                            self.retune_midi(channel, *pitch_midi, parameters);
                        } else {
                            self.key_on(channel, *pitch_midi, parameters);
                            self.program[slot] = parameters.clone();
                            self.has_history[slot] = true;
                        }
                    }
                    self.occupant[slot] = Some(*note_id);
                    self.active.insert(*note_id, channel);
                }
            }
            MusicalEventType::NoteOff { note_id } => {
                if let Some(channel) = self.active.remove(note_id) {
                    // Only release when the off names the live occupant;
                    // stale offs (superseded by legato) must not cut it.
                    let slot = channel as usize;
                    if self.occupant[slot] == Some(*note_id) {
                        self.key_off(channel);
                        self.occupant[slot] = None;
                        self.freed_at[slot] = event.time_seconds;
                    }
                }
            }
            _ => {}
        }
    }

    fn process_events(
        &mut self,
        events: Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync>,
        rate: usize,
    ) {
        for event in events {
            self.process_event(&event, rate);
        }
    }

    fn generate_samples(&mut self, count: usize, _rate: usize) -> Vec<[f32; 2]> {
        self.chip.render(count)
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }

    fn all_notes_off(&mut self) {
        let channels: Vec<u8> = self.active.drain().map(|(_, channel)| channel).collect();
        for channel in channels {
            self.key_off(channel);
            let slot = channel as usize;
            self.occupant[slot] = None;
            self.freed_at[slot] = f32::NEG_INFINITY;
        }
    }
}
