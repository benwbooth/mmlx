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
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

/// YM2612 algorithm number (0-7) to routing.
/// Chart: 0:1>2>3>4, 1:(1+2)>3>4, 2:1>2>4&3>4, 3:1>2>4&3, 4:1>2&3>4,
/// 5:1>2>3&4, 6:1>2&3&4, 7:all parallel. Verify by ear per song.
pub const YM_ALGO_TO_ROUTING: [u8; 8] = [0, 5, 6, 7, 1, 2, 8, 3];

#[derive(Clone, Copy)]
struct FmPatchDef {
    routing: u8,
    feedback: f32,
    ops: [OpConfig; 4],
}

fn patch(name: &str) -> FmPatchDef {
    // (ratios, levels, attack, decay, sustain, release) per patch.
    let def = |routing: u8,
               feedback: f32,
               ratios: [f32; 4],
               levels: [f32; 4],
               attack: f32,
               decay: f32,
               sustain: f32,
               release: f32| {
        let mut ops = [OpConfig {
            ratio: 1.0,
            level: 1.0,
            attack,
            decay,
            sustain,
            release,
        }; 4];
        for (i, op) in ops.iter_mut().enumerate() {
            op.ratio = ratios[i];
            op.level = levels[i];
        }
        FmPatchDef {
            routing,
            feedback,
            ops,
        }
    };
    match name {
        "fm-bell" => def(
            0,
            0.0,
            [1.0, 2.0, 3.0, 4.0],
            [1.0, 0.8, 0.5, 0.4],
            0.005,
            0.4,
            0.0,
            0.3,
        ),
        "fm-bass" => def(
            1,
            0.0,
            [0.5, 1.0, 0.5, 1.0],
            [1.0, 0.7, 1.0, 0.7],
            0.01,
            0.1,
            0.8,
            0.1,
        ),
        "fm-pad" => def(
            3,
            0.0,
            [1.0, 1.0, 2.0, 3.0],
            [0.5, 0.5, 0.3, 0.2],
            0.4,
            0.4,
            0.7,
            0.6,
        ),
        "fm-brass" => def(
            2,
            0.0,
            [1.0, 1.0, 1.0, 2.0],
            [0.9, 0.8, 0.7, 0.5],
            0.08,
            0.2,
            0.8,
            0.15,
        ),
        _ => def(
            4,
            0.35,
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 0.8, 0.7, 0.6],
            0.01,
            0.1,
            0.9,
            0.1,
        ),
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
            let op = &self.def.ops[i];
            if self.released {
                if op.release <= 0.0 {
                    *value = 0.0;
                    *stage = EnvStage::Done;
                } else {
                    *value = (*value - dt / op.release).max(0.0);
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
                    *value = if op.attack <= 0.0 {
                        1.0
                    } else {
                        (*value + dt / op.attack).min(1.0)
                    };
                    if *value >= 1.0 {
                        *stage = EnvStage::Decay;
                    }
                }
                EnvStage::Decay => {
                    *value = if op.decay <= 0.0 {
                        op.sustain
                    } else {
                        (*value - dt / op.decay * (1.0 - op.sustain)).max(op.sustain)
                    };
                    if *value <= op.sustain {
                        *stage = EnvStage::Sustain;
                    }
                }
                EnvStage::Sustain | EnvStage::Release | EnvStage::Done => {}
            }
        }
    }

    fn finished(&self) -> bool {
        self.released
            && self
                .def
                .ops
                .iter()
                .all(|op| op.release <= 0.0 || self.release_age >= op.release + 0.01)
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
            // Stack of 4 (YM alg 0).
            0 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, v1);
                let v3 = op(2, v2);
                op(3, v3)
            }
            // Twin stacks (YM alg 4).
            1 => {
                let v1 = op(0, 0.0);
                let v3 = op(2, 0.0);
                op(1, v1) + op(3, v3)
            }
            // Stack of 3 plus dry carrier (YM alg 5).
            2 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, v1);
                let v3 = op(2, v2);
                v3 + op(3, 0.0)
            }
            // All parallel (YM alg 7).
            3 => op(0, 0.0) + op(1, 0.0) + op(2, 0.0) + op(3, 0.0),
            // Feedback lead: op1 feeds back into itself, then stacks.
            4 => {
                let v1 = op(0, self.fb_state * self.def.feedback);
                self.fb_state = v1;
                let v2 = op(1, v1);
                let v3 = op(2, v2);
                op(3, v3)
            }
            // (1+2) into 3 into 4 (YM alg 1).
            5 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, 0.0);
                let v3 = op(2, v1 + v2);
                op(3, v3)
            }
            // 1 into 2 into 4, plus 3 into 4 (YM alg 2).
            6 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, v1);
                let v3 = op(2, 0.0);
                op(3, v2 + v3)
            }
            // 1 into 2 into 4, plus dry 3 (YM alg 3).
            7 => {
                let v1 = op(0, 0.0);
                let v2 = op(1, v1);
                op(3, v2) + op(2, 0.0)
            }
            // 1 into 2, plus dry 3 and 4 (YM alg 6).
            _ => {
                let v1 = op(0, 0.0);
                op(1, v1) + op(2, 0.0) + op(3, 0.0)
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
                let mut def = patch(name);
                // FM programming via params: every patch field is overridable
                // per block or per note, e.g. `c4q!(op2_ratio = 2.0)`.
                let num = |key: &str| {
                    parameters.get(key).and_then(|value| match value {
                        mmlx_core::ParamValue::Number(number) => Some(*number),
                        _ => None,
                    })
                };
                if let Some(routing) = num("fm_routing") {
                    def.routing = routing.round().clamp(0.0, 8.0) as u8;
                }
                if let Some(feedback) = num("fm_feedback") {
                    def.feedback = feedback.clamp(0.0, 1.0);
                }
                for (i, op) in def.ops.iter_mut().enumerate() {
                    let prefix = format!("op{}", i + 1);
                    let field = |key: &str| num(&format!("{prefix}_{key}"));
                    if let Some(ratio) = field("ratio") {
                        op.ratio = ratio.clamp(0.01, 16.0);
                    }
                    if let Some(level) = field("level") {
                        op.level = level.clamp(0.0, 1.0);
                    }
                    if let Some(attack) = field("attack") {
                        op.attack = attack.max(0.0);
                    }
                    if let Some(decay) = field("decay") {
                        op.decay = decay.max(0.0);
                    }
                    if let Some(sustain) = field("sustain") {
                        op.sustain = sustain.clamp(0.0, 1.0);
                    }
                    if let Some(release) = field("release") {
                        op.release = release.max(0.0);
                    }
                }
                self.active.push(FmVoice::new(
                    *note_id,
                    midi_freq(*pitch_midi),
                    *velocity,
                    def,
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
