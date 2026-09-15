//! `mmlx-chip`: authentic-architecture chiptune voices, pure Rust.
//!
//! [`Nes2A03`] (pulse 12.5/25/50/75%, 32-step triangle, 15-bit LFSR noise)
//! and [`GbDmg`] (pulse, 32×4-bit wavetable channel, 7/15-bit noise).
//! Chiptune-style: constant volume while gated, instant off — no ADSR.
//! Voice selected per note by `patch`.

use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};

fn midi_freq(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0)
}

#[derive(Clone)]
enum Voice {
    Pulse { duty: f32, phase: f32 },
    Triangle { phase: f32 },
    Noise { lfsr: u16, short: bool, acc: f32 },
    Wave { table: [u8; 32], pos: f32 },
}

impl Voice {
    fn step(&mut self, freq: f32, sample_rate: f32) -> f32 {
        match self {
            Voice::Pulse { duty, phase } => {
                *phase = (*phase + freq / sample_rate).fract();
                if *phase < *duty {
                    0.4
                } else {
                    -0.4
                }
            }
            Voice::Triangle { phase } => {
                *phase = (*phase + freq / sample_rate).fract();
                let step = (*phase * 32.0) as usize & 31;
                let level = if step < 16 { 15 - step } else { step - 16 };
                (level as f32 / 15.0 - 0.5) * 0.9
            }
            Voice::Noise { lfsr, short, acc } => {
                *acc += freq * 16.0 / sample_rate;
                while *acc >= 1.0 {
                    *acc -= 1.0;
                    let tap = if *short { 6 } else { 1 };
                    let bit = ((*lfsr ^ (*lfsr >> tap)) & 1) as u16;
                    *lfsr = (*lfsr >> 1) | (bit << 14);
                    if *lfsr == 0 {
                        *lfsr = 1;
                    }
                }
                ((*lfsr & 1) as f32 * 2.0 - 1.0) * 0.3
            }
            Voice::Wave { table, pos } => {
                *pos = (*pos + freq * 32.0 / sample_rate) % 32.0;
                (table[*pos as usize] as f32 / 15.0 - 0.5) * 0.8
            }
        }
    }
}

fn default_wave_table() -> [u8; 32] {
    let mut table = [0u8; 32];
    for (i, slot) in table.iter_mut().enumerate() {
        // Sine quantized to 4 bits (audible default wavetable).
        *slot = ((0.5 + 0.5 * (i as f32 / 32.0 * std::f32::consts::TAU).sin()) * 15.0) as u8;
    }
    table
}

struct ChipVoice {
    id: u64,
    freq: f32,
    velocity: f32,
    voice: Voice,
}

fn drive(
    active: &mut Vec<ChipVoice>,
    event: &TimedMusicalEvent,
    select: &dyn Fn(&str) -> Voice,
    _rate: usize,
) {
    match &event.event {
        MusicalEventType::NoteOn {
            note_id,
            pitch_midi,
            velocity,
            parameters,
            ..
        } => {
            let patch = parameters
                .get("patch")
                .and_then(|value| value.as_string())
                .map(String::as_str)
                .unwrap_or("pulse50");
            active.push(ChipVoice {
                id: *note_id,
                freq: midi_freq(*pitch_midi),
                velocity: *velocity,
                voice: select(patch),
            });
        }
        MusicalEventType::NoteOff { note_id } => {
            active.retain(|voice| voice.id != *note_id);
        }
        _ => {}
    }
}

fn render(active: &mut Vec<ChipVoice>, count: usize, sample_rate: f32) -> Vec<[f32; 2]> {
    let mut buffer = vec![[0.0f32; 2]; count];
    for voice in active.iter_mut() {
        for frame in buffer.iter_mut() {
            let sample = voice.voice.step(voice.freq, sample_rate) * voice.velocity;
            frame[0] += sample;
            frame[1] += sample;
        }
    }
    buffer
}

/// NES 2A03. Patches: `pulse12 pulse25 pulse50 pulse75 triangle noise`.
pub struct Nes2A03 {
    active: Vec<ChipVoice>,
}

impl Nes2A03 {
    pub fn new() -> Self {
        Nes2A03 { active: Vec::new() }
    }
}

impl Default for Nes2A03 {
    fn default() -> Self {
        Self::new()
    }
}

fn nes_select(patch: &str) -> Voice {
    match patch {
        "pulse12" => Voice::Pulse {
            duty: 0.125,
            phase: 0.0,
        },
        "pulse25" => Voice::Pulse {
            duty: 0.25,
            phase: 0.0,
        },
        "pulse75" => Voice::Pulse {
            duty: 0.75,
            phase: 0.0,
        },
        "triangle" => Voice::Triangle { phase: 0.0 },
        "noise" => Voice::Noise {
            lfsr: 0x4000,
            short: false,
            acc: 0.0,
        },
        _ => Voice::Pulse {
            duty: 0.5,
            phase: 0.0,
        },
    }
}

impl Instrument for Nes2A03 {
    fn process_event(&mut self, event: &TimedMusicalEvent, rate: usize) {
        drive(&mut self.active, event, &nes_select, rate);
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

    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]> {
        render(&mut self.active, count, rate as f32)
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}

/// GameBoy DMG. Patches: `pulse12 pulse25 pulse50 pulse75 wave noise short-noise`.
pub struct GbDmg {
    active: Vec<ChipVoice>,
}

impl GbDmg {
    pub fn new() -> Self {
        GbDmg { active: Vec::new() }
    }
}

impl Default for GbDmg {
    fn default() -> Self {
        Self::new()
    }
}

fn gb_select(patch: &str) -> Voice {
    match patch {
        "wave" => Voice::Wave {
            table: default_wave_table(),
            pos: 0.0,
        },
        "noise" => Voice::Noise {
            lfsr: 0x4000,
            short: false,
            acc: 0.0,
        },
        "short-noise" => Voice::Noise {
            lfsr: 0x4000,
            short: true,
            acc: 0.0,
        },
        other => nes_select(other),
    }
}

impl Instrument for GbDmg {
    fn process_event(&mut self, event: &TimedMusicalEvent, rate: usize) {
        drive(&mut self.active, event, &gb_select, rate);
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

    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]> {
        render(&mut self.active, count, rate as f32)
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}
