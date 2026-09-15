//! Exact YM2612 streaming voice over the vendored GENS core.
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
    fn YM2612_Init(clock: u32, rate: u32, interpolation: u8) -> *mut c_void;
    fn YM2612_End(chip: *mut c_void);
    fn YM2612_Reset(chip: *mut c_void);
    fn YM2612_Write(chip: *mut c_void, adr: u8, data: u8);
    fn YM2612_Update(chip: *mut c_void, buf: *mut *mut i32, length: u32);
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

pub struct Ym2612 {
    chip: *mut c_void,
    rate: u32,
    // DC-blocking highpass state (the core idles with a large DC offset,
    // which every player strips; R ≈ 0.995 at 44.1 kHz).
    hp_in_l: f32,
    hp_in_r: f32,
    hp_out_l: f32,
    hp_out_r: f32,
}

// The chip is only touched from the render path.
unsafe impl Send for Ym2612 {}

impl Ym2612 {
    pub fn new(clock: u32, rate: u32) -> Option<Self> {
        unsafe {
            let chip = YM2612_Init(clock, rate, 1);
            if chip.is_null() {
                None
            } else {
                // Reset is mandatory: it seeds the envelope/DT table pointers
                // that register writes dereference (skipping it segfaults on
                // the first TL write via Special_Update).
                YM2612_Reset(chip);
                Some(Ym2612 {
                    chip,
                    rate,
                    hp_in_l: 0.0,
                    hp_in_r: 0.0,
                    hp_out_l: 0.0,
                    hp_out_r: 0.0,
                })
            }
        }
    }

    pub fn reset(&mut self) {
        unsafe { YM2612_Reset(self.chip) };
    }

    /// Raw register write: VGM 0x52 = port 0, 0x53 = port 1.
    pub fn write(&mut self, port: u8, addr: u8, data: u8) {
        unsafe {
            let base = if port == 0 { 0 } else { 2 };
            YM2612_Write(self.chip, base, addr);
            YM2612_Write(self.chip, base + 1, data);
        }
    }

    /// Render interleaved stereo float in [-1, 1].
    pub fn render(&mut self, frames: usize) -> Vec<[f32; 2]> {
        let mut left = vec![0i32; frames];
        let mut right = vec![0i32; frames];
        let mut buffers = [left.as_mut_ptr(), right.as_mut_ptr()];
        unsafe {
            YM2612_Update(self.chip, buffers.as_mut_ptr(), frames as u32);
        }
        // GENS core outputs 14-bit-ish samples; normalize conservatively.
        // Then strip the core's idle DC offset with a one-pole highpass.
        let mut out = Vec::with_capacity(frames);
        for (l, r) in left.into_iter().zip(right) {
            let l = (l as f32 / 16384.0).clamp(-1.0, 1.0);
            let r = (r as f32 / 16384.0).clamp(-1.0, 1.0);
            self.hp_out_l = l - self.hp_in_l + 0.995 * self.hp_out_l;
            self.hp_out_r = r - self.hp_in_r + 0.995 * self.hp_out_r;
            self.hp_in_l = l;
            self.hp_in_r = r;
            out.push([self.hp_out_l, self.hp_out_r]);
        }
        out
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }
}

impl Drop for Ym2612 {
    fn drop(&mut self) {
        unsafe { YM2612_End(self.chip) };
    }
}

/// MIDI note to (block, fnum11) for the YM2612 frequency registers.
/// Calibrated against the GENS core: A440 renders at 440 Hz.
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
}

// The chip is only touched from the render path.
unsafe impl Send for Ym2612Voice {}
unsafe impl Sync for Ym2612Voice {}

impl Ym2612Voice {
    pub fn new(clock: u32, rate: u32) -> Option<Self> {
        Ym2612::new(clock, rate).map(|chip| Ym2612Voice {
            chip,
            active: HashMap::new(),
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
        let routing = num("fm_routing").unwrap_or(0.0).round().clamp(0.0, 8.0) as usize;
        let algo = ROUTING_TO_YM_ALGO[routing];
        let feedback = (num("fm_feedback").unwrap_or(0.0).clamp(0.0, 1.0) * 7.0).round() as u8;
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
            let mult = field("ratio").or_else(|| field("mult")).unwrap_or(1.0);
            let mult_reg = if mult <= 0.75 {
                0
            } else {
                mult.round().clamp(1.0, 15.0) as u8
            };
            self.chip
                .write(port, 0x30 + row * 4 + base, mult_reg & 0x0F);
            let level = field("level").unwrap_or(0.8).clamp(0.0, 1.0);
            self.chip
                .write(port, 0x40 + row * 4 + base, gain_to_tl(level));
            let attack = field("attack").unwrap_or(0.01).max(0.0);
            self.chip
                .write(port, 0x50 + row * 4 + base, secs_to_rate(attack, 1.5, 31));
            let decay = field("decay").unwrap_or(0.1).max(0.0);
            self.chip
                .write(port, 0x60 + row * 4 + base, secs_to_rate(decay, 3.0, 31));
            // Sustain rate: hold (our voices sustain indefinitely).
            self.chip.write(port, 0x70 + row * 4 + base, 0);
            let sustain = field("sustain").unwrap_or(0.9).clamp(0.0, 1.0);
            let release = field("release").unwrap_or(0.1).max(0.0);
            let sustain_level = ((1.0 - sustain) * 15.0).round() as u8;
            let release_rate = secs_to_rate(release, 3.0, 15);
            self.chip.write(
                port,
                0x80 + row * 4 + base,
                (sustain_level << 4) | release_rate,
            );
            self.chip.write(port, 0x90 + row * 4 + base, 0); // SSG-EG off
        }
        let (block, fnum) = midi_to_fnum(midi, YM2612_CLOCK_NTSC);
        self.chip
            .write(port, 0xA4 + base, (block << 3) | ((fnum >> 8) as u8 & 0x07));
        self.chip.write(port, 0xA0 + base, (fnum & 0xFF) as u8);
        // Key on, all slots.
        let code = (channel % 3) | ((channel / 3) << 2);
        self.chip.write(0, 0x28, 0xF0 | code);
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
                    self.key_on(channel, *pitch_midi, parameters);
                    self.active.insert(*note_id, channel);
                }
            }
            MusicalEventType::NoteOff { note_id } => {
                if let Some(channel) = self.active.remove(note_id) {
                    self.key_off(channel);
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
}
