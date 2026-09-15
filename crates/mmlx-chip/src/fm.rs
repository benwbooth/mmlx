//! 4-operator FM voices (YM2612/OPL-inspired).
//!
//! Five fixed routings (stack-4, twin stacks, stack-3 + carrier, parallel,
//! feedback lead) with per-operator ADSR. Patches: `fm-lead fm-bell
//! fm-bass fm-pad fm-brass`. Compact and honest: inspired by, not a clone of,
//! Yamaha's 8 algorithms.

use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
use std::f32::consts::TAU;

fn midi_freq(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0)
}

#[derive(Clone, Copy)]
struct OpConfig {
    ratio: f32,
    level: f32,
}

#[derive(Clone, Copy)]
struct FmPatchDef {
    routing: u8,
    feedback: f32,
    ops: [OpConfig; 4],
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

fn patch(name: &str) -> FmPatchDef {
    let op = |ratio: f32, level: f32| OpConfig { ratio, level };
    match name {
        "fm-bell" => FmPatchDef {
            routing: 0,
            feedback: 0.0,
            ops: [op(1.0, 1.0), op(2.0, 0.8), op(3.0, 0.5), op(4.0, 0.4)],
            attack: 0.005,
            decay: 0.4,
            sustain: 0.0,
            release: 0.3,
        },
        "fm-bass" => FmPatchDef {
            routing: 1,
            feedback: 0.0,
            ops: [op(0.5, 1.0), op(1.0, 0.7), op(0.5, 1.0), op(1.0, 0.7)],
            attack: 0.01,
            decay: 0.1,
            sustain: 0.8,
            release: 0.1,
        },
        "fm-pad" => FmPatchDef {
            routing: 3,
            feedback: 0.0,
            ops: [op(1.0, 0.5), op(1.0, 0.5), op(2.0, 0.3), op(3.0, 0.2)],
            attack: 0.4,
            decay: 0.4,
            sustain: 0.7,
            release: 0.6,
        },
        "fm-brass" => FmPatchDef {
            routing: 2,
            feedback: 0.0,
            ops: [op(1.0, 0.9), op(1.0, 0.8), op(1.0, 0.7), op(2.0, 0.5)],
            attack: 0.08,
            decay: 0.2,
            sustain: 0.8,
            release: 0.15,
        },
        _ => FmPatchDef {
            // fm-lead: feedback stack.
            routing: 4,
            feedback: 0.35,
            ops: [op(1.0, 1.0), op(1.0, 0.8), op(1.0, 0.7), op(1.0, 0.6)],
            attack: 0.01,
            decay: 0.1,
            sustain: 0.9,
            release: 0.1,
        },
    }
}

#[derive(Clone, Copy, PartialEq)]
enum EnvStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

struct FmVoice {
    id: u64,
    freq: f32,
    velocity: f32,
    def: FmPatchDef,
    phases: [f32; 4],
    env: [f32; 4],
    stages: [EnvStage; 4],
    age: f32,
    release_age: f32,
    released: bool,
    fb_state: f32,
}

impl FmVoice {
    fn new(id: u64, freq: f32, velocity: f32, def: FmPatchDef) -> Self {
        FmVoice {
            id,
            freq,
            velocity,
            def,
            phases: [0.0; 4],
            env: [0.0; 4],
            stages: [EnvStage::Attack; 4],
            age: 0.0,
            release_age: 0.0,
            released: false,
            fb_state: 0.0,
        }
    }

    fn advance_env(&mut self, dt: f32) {
        if self.released {
            self.release_age += dt;
        } else {
            self.age += dt;
        }
        for i in 0..4 {
            let (value, stage) = (&mut self.env[i], &mut self.stages[i]);
            if self.released {
                if self.def.release <= 0.0 {
                    *value = 0.0;
                    *stage = EnvStage::Done;
                } else {
                    *value = (*value - dt / self.def.release).max(0.0);
                    *stage = if *value <= 0.0 {
                        EnvStage::Done
                    } else {
                        EnvStage::Release
                    };
                }
                continue;
            }
            match stage {
                EnvStage::Attack => {
                    *value = if self.def.attack <= 0.0 {
                        1.0
                    } else {
                        (*value + dt / self.def.attack).min(1.0)
                    };
                    if *value >= 1.0 {
                        *stage = EnvStage::Decay;
                    }
                }
                EnvStage::Decay => {
                    *value = if self.def.decay <= 0.0 {
                        self.def.sustain
                    } else {
                        (*value - dt / self.def.decay * (1.0 - self.def.sustain))
                            .max(self.def.sustain)
                    };
                    if *value <= self.def.sustain {
                        *stage = EnvStage::Sustain;
                    }
                }
                EnvStage::Sustain | EnvStage::Release | EnvStage::Done => {}
            }
        }
    }

    fn finished(&self) -> bool {
        self.released && (self.def.release <= 0.0 || self.release_age >= self.def.release + 0.01)
    }

    fn sample(&mut self, sample_rate: f32) -> f32 {
        let dt = 1.0 / sample_rate;
        self.advance_env(dt);
        for i in 0..4 {
            self.phases[i] =
                (self.phases[i] + self.freq * self.def.ops[i].ratio / sample_rate).fract();
        }
        let osc = |phase: f32| (phase * TAU).sin();
        let op = |i: usize, modulation: f32| {
            osc(self.phases[i] + modulation) * self.env[i] * self.def.ops[i].level
        };
        let out = match self.def.routing {
            // Stack of 4.
            0 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, v1);
                let v3 = op(2, v2);
                op(3, v3)
            }
            // Twin stacks.
            1 => {
                let v1 = op(0, 0.0);
                let v3 = op(2, 0.0);
                op(1, v1) + op(3, v3)
            }
            // Stack of 3 plus dry carrier.
            2 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, v1);
                let v3 = op(2, v2);
                v3 + op(3, 0.0)
            }
            // All parallel.
            3 => op(0, 0.0) + op(1, 0.0) + op(2, 0.0) + op(3, 0.0),
            // Feedback lead: op1 feeds back into itself, then stacks.
            _ => {
                let v1 = op(0, self.fb_state * self.def.feedback);
                self.fb_state = v1;
                let v2 = op(1, v1);
                let v3 = op(2, v2);
                op(3, v3)
            }
        };
        out * 0.25 * self.velocity
    }
}

/// 4-operator FM synth. Patch selects routing (`fm-lead fm-bell fm-bass
/// fm-pad fm-brass`, default `fm-lead`).
pub struct Fm4 {
    active: Vec<FmVoice>,
}

impl Fm4 {
    pub fn new() -> Self {
        Fm4 { active: Vec::new() }
    }
}

impl Default for Fm4 {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument for Fm4 {
    fn process_event(&mut self, event: &TimedMusicalEvent, _rate: usize) {
        match &event.event {
            MusicalEventType::NoteOn {
                note_id,
                pitch_midi,
                velocity,
                parameters,
                ..
            } => {
                let name = parameters
                    .get("patch")
                    .and_then(|value| value.as_string())
                    .map(String::as_str)
                    .unwrap_or("fm-lead");
                self.active.push(FmVoice::new(
                    *note_id,
                    midi_freq(*pitch_midi),
                    *velocity,
                    patch(name),
                ));
            }
            MusicalEventType::NoteOff { note_id } => {
                for voice in &mut self.active {
                    if voice.id == *note_id {
                        voice.released = true;
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
                let sample = voice.sample(rate as f32);
                frame[0] += sample;
                frame[1] += sample;
            }
        }
        self.active.retain(|voice| !voice.finished());
        buffer
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}
