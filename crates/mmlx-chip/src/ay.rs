//! AY-3-8910 voice: 3 tone channels + shared noise + hardware envelope.
//!
//! Patches `ay-square` (tone) and `ay-noise`. Full register-level params:
//! `ay_channel` (0-2, else round-robin), `ay_volume` (0-15), `ay_noise`
//! (0/1 tone+noise mix), `ay_noise_period` (0-31), `ay_env_shape` (0-15,
//! 0 = off), `ay_env_period` (envelope steps). Clock fixed at 2 MHz.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;

const CLOCK: f32 = 2_000_000.0;

fn number(parameters: &HashMap<String, ParamValue>, key: &str) -> Option<f32> {
    parameters.get(key).and_then(|value| match value {
        ParamValue::Number(number) => Some(*number),
        _ => None,
    })
}

fn midi_freq(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0)
}

/// Hardware envelope state machine (shape bits: continue/attack/alternate/hold).
struct AyEnvelope {
    shape: u8,
    period_samples: f32,
    acc: f32,
    level: u8, // 0-15
    phase: u8, // 0 = first ramp, 1 = alternate ramp
    done: bool,
}

impl AyEnvelope {
    fn new(shape: u8, period_samples: f32) -> Self {
        let attack = shape & 0x04 != 0;
        AyEnvelope {
            shape: shape & 0x0F,
            period_samples: period_samples.max(1.0),
            acc: 0.0,
            level: if attack { 0 } else { 15 },
            phase: 0,
            done: false,
        }
    }

    /// Advance one sample; returns 0-15.
    fn step(&mut self) -> u8 {
        if self.done {
            return self.level;
        }
        self.acc += 1.0;
        if self.acc < self.period_samples {
            return self.level;
        }
        self.acc = 0.0;
        let attack = self.shape & 0x04 != 0;
        let alternate = self.shape & 0x02 != 0;
        let hold = self.shape & 0x01 != 0;
        let cont = self.shape & 0x08 != 0;
        let up = if self.phase == 0 {
            attack
        } else {
            attack ^ alternate
        };
        if up {
            if self.level < 15 {
                self.level += 1;
            } else {
                self.end_of_ramp(cont, hold, alternate);
            }
        } else if self.level > 0 {
            self.level -= 1;
        } else {
            self.end_of_ramp(cont, hold, alternate);
        }
        self.level
    }

    fn end_of_ramp(&mut self, cont: bool, hold: bool, alternate: bool) {
        if !cont {
            self.level = 0;
            self.done = true;
        } else if hold {
            self.done = true; // hold current extreme
        } else {
            self.phase ^= 1; // loop or alternate
            if !alternate {
                self.level = if self.shape & 0x04 != 0 { 0 } else { 15 };
            }
        }
    }
}

struct AyChannel {
    id: u64,
    freq: f32,
    tone_acc: f32,
    tone_high: bool,
    volume: u8,
    use_envelope: bool,
    noise_mix: bool,
    envelope: Option<AyEnvelope>,
}

pub struct AyVoice {
    channels: [Option<AyChannel>; 3],
    noise_lfsr: u32,
    noise_period: f32,
    noise_acc: f32,
    noise_bit: bool,
    next_channel: usize,
}

impl AyVoice {
    pub fn new() -> Self {
        AyVoice {
            channels: [None, None, None],
            noise_lfsr: 0x1FFFF,
            noise_period: 8.0,
            noise_acc: 0.0,
            noise_bit: false,
            next_channel: 0,
        }
    }

    fn channel_for(&mut self, requested: Option<usize>) -> usize {
        if let Some(channel) = requested {
            return channel.min(2);
        }
        let channel = self.next_channel;
        self.next_channel = (self.next_channel + 1) % 3;
        channel
    }
}

impl Default for AyVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for AyVoice {
    fn process_event(&mut self, event: &TimedMusicalEvent, rate: usize) {
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
                    .unwrap_or("ay-square");
                let channel = self
                    .channel_for(number(parameters, "ay_channel").map(|channel| channel as usize));
                let freq = midi_freq(*pitch_midi);
                let volume = number(parameters, "ay_volume")
                    .map(|volume| volume.round().clamp(0.0, 15.0) as u8)
                    .unwrap_or_else(|| (*velocity * 15.0).round().clamp(0.0, 15.0) as u8);
                let envelope = number(parameters, "ay_env_shape")
                    .map(|shape| shape as u8)
                    .filter(|shape| *shape != 0);
                self.channels[channel] = Some(AyChannel {
                    id: *note_id,
                    freq,
                    tone_acc: 0.0,
                    tone_high: false,
                    volume,
                    use_envelope: envelope.is_some(),
                    noise_mix: patch == "ay-noise"
                        || number(parameters, "ay_noise").unwrap_or(0.0) > 0.5,
                    envelope: envelope.map(|shape| {
                        // AY ticks (clock/256) to samples at this rate.
                        let ticks = number(parameters, "ay_env_period")
                            .unwrap_or(256.0)
                            .max(1.0);
                        AyEnvelope::new(shape, ticks * rate as f32 / (CLOCK / 256.0))
                    }),
                });
                if let Some(period) = number(parameters, "ay_noise_period") {
                    self.noise_period = period.clamp(1.0, 31.0);
                }
            }
            MusicalEventType::NoteOff { note_id } => {
                for slot in self.channels.iter_mut() {
                    if matches!(slot, Some(channel) if channel.id == *note_id) {
                        *slot = None;
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

    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]> {
        let mut buffer = vec![[0.0f32; 2]; count];
        // Noise advances at clock/16/noise_period.
        let noise_step = CLOCK / 16.0 / self.noise_period / rate as f32;
        for frame in buffer.iter_mut() {
            self.noise_acc += noise_step;
            while self.noise_acc >= 1.0 {
                self.noise_acc -= 1.0;
                let bit = ((self.noise_lfsr ^ (self.noise_lfsr >> 3)) & 1) as u32;
                self.noise_lfsr = (self.noise_lfsr >> 1) | (bit << 16);
                self.noise_bit = (self.noise_lfsr & 1) != 0;
            }
            let mut sample = 0.0;
            for slot in self.channels.iter_mut().flatten() {
                // Square wave: toggle every half period.
                let half = (rate as f32 / (2.0 * slot.freq)).max(1.0);
                slot.tone_acc += 1.0;
                if slot.tone_acc >= half {
                    slot.tone_acc = 0.0;
                    slot.tone_high = !slot.tone_high;
                }
                let mut on = slot.tone_high;
                if slot.noise_mix {
                    on = on && self.noise_bit;
                }
                let level = match &mut slot.envelope {
                    Some(envelope) => envelope.step(),
                    None => {
                        if slot.use_envelope {
                            0
                        } else {
                            slot.volume
                        }
                    }
                };
                if on {
                    sample += level as f32 / 15.0 * 0.3;
                }
            }
            frame[0] += sample;
            frame[1] += sample;
        }
        buffer
    }

    fn is_idle(&self) -> bool {
        self.channels.iter().all(Option::is_none)
    }

    fn all_notes_off(&mut self) {
        self.channels = [None, None, None];
    }
}
