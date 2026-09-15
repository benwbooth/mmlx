//! SN76489 voice: 3 square tones + noise, Genesis PSG flavor.
//!
//! Patches `sn-square` and `sn-noise-white` / `sn-noise-periodic` /
//! `sn-noise-tone3`. Params: `sn_channel` (0-2 tone, 3 noise, else
//! round-robin), `sn_volume` (0-15 loudness, converted to attenuation),
//! `sn_noise_mode` (0 white, 1 periodic, 2 follow tone 3). Clock 3579545 Hz.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;

const CLOCK: f32 = 3_579_545.0;

fn number(parameters: &HashMap<String, ParamValue>, key: &str) -> Option<f32> {
    parameters.get(key).and_then(|value| match value {
        ParamValue::Number(number) => Some(*number),
        _ => None,
    })
}

fn midi_freq(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0)
}

struct SnTone {
    id: u64,
    freq: f32,
    gain: f32,
    phase: f32,
}

struct SnNoise {
    id: u64,
    gain: f32,
    white: bool,
    lfsr: u16,
    acc: f32,
    step: f32,
    out: f32,
}

pub struct PsgVoice {
    tones: [Option<SnTone>; 3],
    noise: Option<SnNoise>,
    next_tone: usize,
    tone3_freq: f32,
}

impl PsgVoice {
    pub fn new() -> Self {
        PsgVoice {
            tones: [None, None, None],
            noise: None,
            next_tone: 0,
            tone3_freq: 440.0,
        }
    }
}

impl Default for PsgVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for PsgVoice {
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
                    .unwrap_or("sn-square");
                // DSL loudness 0-15 to attenuation gain (2 dB steps).
                let attenuation = 15.0
                    - number(parameters, "sn_volume")
                        .unwrap_or_else(|| *velocity * 15.0)
                        .round()
                        .clamp(0.0, 15.0);
                let gain = 10.0f32.powf(-attenuation * 2.0 / 20.0) * 0.5;
                if patch.starts_with("sn-noise") {
                    let white = !patch.ends_with("periodic")
                        && number(parameters, "sn_noise_mode").unwrap_or(0.0) < 1.5;
                    let follow = patch.ends_with("tone3")
                        || number(parameters, "sn_noise_mode").unwrap_or(0.0) > 1.5;
                    // Noise rate: fixed N/512, or clocked by tone 3.
                    let rate = if follow {
                        self.tone3_freq * 8.0
                    } else {
                        CLOCK / 512.0
                    };
                    self.noise = Some(SnNoise {
                        id: *note_id,
                        gain,
                        white,
                        lfsr: 0x8000,
                        acc: 0.0,
                        step: rate,
                        out: 0.4,
                    });
                } else {
                    let channel = number(parameters, "sn_channel")
                        .map(|channel| channel as usize % 3)
                        .unwrap_or_else(|| {
                            let channel = self.next_tone;
                            self.next_tone = (self.next_tone + 1) % 3;
                            channel
                        });
                    let freq = midi_freq(*pitch_midi);
                    if channel == 2 {
                        self.tone3_freq = freq;
                    }
                    self.tones[channel] = Some(SnTone {
                        id: *note_id,
                        freq,
                        gain,
                        phase: 0.0,
                    });
                }
            }
            MusicalEventType::NoteOff { note_id } => {
                for slot in self.tones.iter_mut() {
                    if matches!(slot, Some(tone) if tone.id == *note_id) {
                        *slot = None;
                    }
                }
                if matches!(&self.noise, Some(noise) if noise.id == *note_id) {
                    self.noise = None;
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

    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]> {
        let mut buffer = vec![[0.0f32; 2]; count];
        for frame in buffer.iter_mut() {
            let mut sample = 0.0;
            for slot in self.tones.iter_mut().flatten() {
                slot.phase = (slot.phase + slot.freq / rate as f32).fract();
                sample += if slot.phase < 0.5 {
                    slot.gain
                } else {
                    -slot.gain
                };
            }
            if let Some(noise) = self.noise.as_mut() {
                noise.acc += noise.step / rate as f32;
                while noise.acc >= 1.0 {
                    noise.acc -= 1.0;
                    if noise.white {
                        let bit = ((noise.lfsr >> 0) ^ (noise.lfsr >> 3)) & 1;
                        noise.lfsr = (noise.lfsr >> 1) | (bit << 15);
                        noise.out = if noise.lfsr & 1 == 1 { 0.4 } else { -0.4 };
                    } else {
                        // Periodic: square at the noise rate.
                        noise.out = -noise.out;
                    }
                }
                sample += noise.out * noise.gain;
            }
            frame[0] += sample;
            frame[1] += sample;
        }
        buffer
    }

    fn is_idle(&self) -> bool {
        self.noise.is_none() && self.tones.iter().all(Option::is_none)
    }
}
