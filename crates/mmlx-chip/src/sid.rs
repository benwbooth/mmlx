//! MOS6581-flavored voice: pulse/saw/triangle + resonant lowpass.
//!
//! Patches `sid-pulse sid-saw sid-tri`. Filter `cutoff` (Hz, default 8000)
//! and `resonance` (0..1, default 0) ride as note params — new `cutoff` /
//! `resonance` keys in the core table.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};

fn midi_freq(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0)
}

fn number(parameters: &std::collections::HashMap<String, ParamValue>, key: &str) -> Option<f32> {
    parameters.get(key).and_then(|value| match value {
        ParamValue::Number(number) => Some(*number),
        _ => None,
    })
}

struct SidVoice {
    id: u64,
    freq: f32,
    velocity: f32,
    wave: u8, // 0 = pulse, 1 = saw, 2 = tri
    duty: f32,
    phase: f32,
    cutoff: f32,
    resonance: f32,
    low: f32,
    band: f32,
}

impl SidVoice {
    fn sample(&mut self, sample_rate: f32) -> f32 {
        self.phase = (self.phase + self.freq / sample_rate).fract();
        let raw = match self.wave {
            0 => {
                if self.phase < self.duty {
                    0.5
                } else {
                    -0.5
                }
            }
            1 => self.phase * 2.0 - 1.0,
            _ => (2.0 * (self.phase * 2.0 - 1.0).abs() - 1.0) * 0.7,
        };
        // Chamberlin state-variable lowpass.
        let cutoff = self.cutoff.clamp(40.0, sample_rate * 0.45);
        let f = 2.0 * (std::f32::consts::PI * cutoff / sample_rate).sin();
        let q = 1.0 - self.resonance.clamp(0.0, 0.95) * 0.9;
        self.low += f * self.band;
        let high = raw - self.low - q * self.band;
        self.band += f * high;
        self.low * self.velocity
    }
}

/// SID-flavored synth (`sid-pulse sid-saw sid-tri`, default pulse).
pub struct Sid {
    active: Vec<SidVoice>,
}

impl Sid {
    pub fn new() -> Self {
        Sid { active: Vec::new() }
    }
}

impl Default for Sid {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for Sid {
    fn process_event(&mut self, event: &TimedMusicalEvent, _rate: usize) {
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
                    .unwrap_or("sid-pulse");
                let wave = match patch {
                    "sid-saw" => 1,
                    "sid-tri" => 2,
                    _ => 0,
                };
                self.active.push(SidVoice {
                    id: *note_id,
                    freq: midi_freq(*pitch_midi),
                    velocity: *velocity,
                    wave,
                    duty: number(parameters, "duty").unwrap_or(0.5).clamp(0.01, 0.99),
                    phase: 0.0,
                    cutoff: number(parameters, "cutoff").unwrap_or(8000.0),
                    resonance: number(parameters, "resonance").unwrap_or(0.0),
                    low: 0.0,
                    band: 0.0,
                });
            }
            MusicalEventType::NoteOff { note_id } => {
                self.active.retain(|voice| voice.id != *note_id);
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

    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]> {
        let mut buffer = vec![[0.0f32; 2]; count];
        for voice in self.active.iter_mut() {
            for frame in buffer.iter_mut() {
                let sample = voice.sample(rate as f32);
                frame[0] += sample;
                frame[1] += sample;
            }
        }
        buffer
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}
