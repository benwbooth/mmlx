//! OPL2 voice: 2-operator FM with 4 waveforms, FM/AM connection.
//!
//! Patches `opl-fm` (default) and `opl-am`. Params: `opl_alg` (0 FM, 1 AM),
//! `opl_feedback` (0-7, operator 1 self-modulation), per operator
//! `op1_mult op1_level op1_attack op1_decay op1_sustain op1_release
//! op1_wave op1_egt` (same `op2_*`). Tremolo/vibrato depths, KSR and rhythm
//! mode are not modeled (documented).

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;
use std::f32::consts::TAU;

fn number(parameters: &HashMap<String, ParamValue>, key: &str) -> Option<f32> {
    parameters.get(key).and_then(|value| match value {
        ParamValue::Number(number) => Some(*number),
        _ => None,
    })
}

fn midi_freq(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0)
}

/// OPL2 waveform shapes 0-3.
fn wave(shape: u8, phase: f32) -> f32 {
    let sine = (phase * TAU).sin();
    match shape {
        1 => sine.max(0.0), // half sine
        2 => sine.abs(),    // abs sine
        3 => {
            if phase < 0.5 {
                (phase * 2.0 * TAU).sin()
            } else {
                0.0
            }
        } // pulse sine
        _ => sine,
    }
}

struct OplOp {
    ratio: f32,
    level: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    egt: bool,
    wave: u8,
    phase: f32,
    env: f32,
    released: bool,
    release_age: f32,
}

impl OplOp {
    fn step_env(&mut self, dt: f32) -> f32 {
        if self.released {
            self.release_age += dt;
            self.env = if self.release <= 0.0 {
                0.0
            } else {
                (self.env - dt / self.release).max(0.0)
            };
        } else if self.env < 1.0 {
            self.env = if self.attack <= 0.0 {
                1.0
            } else {
                (self.env + dt / self.attack).min(1.0)
            };
        } else if self.env > self.sustain {
            self.env = if self.decay <= 0.0 {
                self.sustain
            } else {
                (self.env - dt / self.decay * (1.0 - self.sustain)).max(self.sustain)
            };
            if !self.egt && self.env <= 0.0 {
                self.env = 0.0;
            }
        } else if !self.egt && self.sustain > 0.0 {
            // Non-EGT voices keep decaying past sustain at the decay rate.
            self.env = if self.decay <= 0.0 {
                0.0
            } else {
                (self.env - dt / self.decay * (1.0 - self.sustain)).max(0.0)
            };
        }
        self.env
    }
}

struct OplVoice {
    id: u64,
    freq: f32,
    velocity: f32,
    am: bool,
    feedback: f32,
    ops: [OplOp; 2],
    fb_state: f32,
}

impl OplVoice {
    fn sample(&mut self, sample_rate: f32) -> (f32, bool) {
        let dt = 1.0 / sample_rate;
        let e0 = self.ops[0].step_env(dt);
        let e1 = self.ops[1].step_env(dt);
        for op in self.ops.iter_mut() {
            let ratio = op.ratio;
            op.phase = (op.phase + self.freq * ratio / sample_rate).fract();
        }
        let m = wave(
            self.ops[0].wave,
            self.ops[0].phase + self.fb_state * self.feedback,
        ) * e0
            * self.ops[0].level;
        self.fb_state = m;
        let c = wave(
            self.ops[1].wave,
            self.ops[1].phase + if self.am { 0.0 } else { m },
        ) * e1
            * self.ops[1].level;
        let out = if self.am {
            (m + c) * 0.5 * self.velocity
        } else {
            c * self.velocity
        };
        let done = self.ops.iter().all(|op| op.released && op.env <= 0.0);
        (out, done)
    }
}

fn op_config(parameters: &HashMap<String, ParamValue>, prefix: &str) -> OplOp {
    let field = |key: &str| number(parameters, &format!("{prefix}_{key}"));
    let mult = field("mult").or_else(|| field("ratio")).unwrap_or(1.0);
    OplOp {
        ratio: if mult <= 0.5 {
            0.5
        } else {
            mult.clamp(0.5, 15.0)
        },
        level: field("level").unwrap_or(0.8).clamp(0.0, 1.0),
        attack: field("attack").unwrap_or(0.01).max(0.0),
        decay: field("decay").unwrap_or(0.1).max(0.0),
        sustain: field("sustain").unwrap_or(0.9).clamp(0.0, 1.0),
        release: field("release").unwrap_or(0.1).max(0.0),
        egt: field("egt").unwrap_or(1.0) > 0.5,
        wave: field("wave").unwrap_or(0.0).round().clamp(0.0, 3.0) as u8,
        phase: 0.0,
        env: 0.0,
        released: false,
        release_age: 0.0,
    }
}

/// OPL2-style 2-operator FM synth.
pub struct Opl2 {
    active: Vec<OplVoice>,
}

impl Opl2 {
    pub fn new() -> Self {
        Opl2 { active: Vec::new() }
    }
}

impl Default for Opl2 {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for Opl2 {
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
                    .unwrap_or("opl-fm");
                self.active.push(OplVoice {
                    id: *note_id,
                    freq: midi_freq(*pitch_midi),
                    velocity: *velocity,
                    am: patch == "opl-am" || number(parameters, "opl_alg").unwrap_or(0.0) > 0.5,
                    feedback: number(parameters, "opl_feedback")
                        .unwrap_or(0.0)
                        .clamp(0.0, 7.0)
                        / 7.0
                        * 0.8,
                    ops: [op_config(parameters, "op1"), op_config(parameters, "op2")],
                    fb_state: 0.0,
                });
            }
            MusicalEventType::NoteOff { note_id } => {
                for voice in &mut self.active {
                    if voice.id == *note_id {
                        voice.ops[0].released = true;
                        voice.ops[1].released = true;
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
        for voice in self.active.iter_mut() {
            for frame in buffer.iter_mut() {
                let (sample, _) = voice.sample(rate as f32);
                frame[0] += sample;
                frame[1] += sample;
            }
        }
        self.active.retain(|voice| {
            !(voice.ops[0].released
                && voice.ops[1].released
                && voice.ops[0].env <= 0.0
                && voice.ops[1].env <= 0.0)
        });
        buffer
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }

    fn all_notes_off(&mut self) {
        self.active.clear();
    }
}
