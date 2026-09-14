// src/instruments/basic_synth.rs

use std::collections::HashMap;

// Import from mmlx_core instead of individual modules
use mmlx_core::{
    get_base_midi_and_offset,
    get_key_signature_adjustment, // MetricDuration import removed
    Envelope,
    Instrument,
    MusicalEventType,
    ParamValue,
    TimedMusicalEvent,
};

#[derive(Debug, Clone, PartialEq, Copy)]
enum NotePhase {
    Attack,  // Initial attack phase
    Sustain, // Sustain phase (loops until note off)
    Release, // Release phase (after note off)
    Off,     // Note is completely off
}

/// Represents a single note currently being played by the synth.
#[derive(Debug, Clone)]
struct ActiveNote {
    note_id: u64,
    pitch_midi: u8,

    // Duration of the ADS phase. For indefinite notes, this might be u64::MAX.
    // For fixed duration notes, this is the time until a NoteOff is expected or implicit release starts.
    ads_duration_samples: u64,

    velocity: f32,
    phase_osc: f32, // Renamed from phase to avoid conflict with NotePhase

    // Current state of the note (ADS, Release, Off)
    current_phase: NotePhase,
    // Samples elapsed within the current_phase (ADS or Release)
    time_in_current_phase_samples: u64,
    // Volume level when the release phase began, to scale relative release envelopes if needed.
    volume_at_release_start: f32,

    // Envelope definitions for the three phases
    attack_envelope: Option<Envelope>, // Attack/Decay phase (plays once)
    sustain_envelope: Option<Envelope>, // Sustain phase (loops until NoteOff)
    release_envelope: Option<Envelope>, // Release phase (plays once after NoteOff)

    // Duration tracking for each envelope phase
    attack_duration_samples: Option<u64>, // Duration of attack envelope
    sustain_duration_samples: Option<u64>, // Duration of one loop of sustain envelope
    release_duration_samples: Option<u64>, // Duration of release envelope

    pitch_envelope: Option<Envelope>,
    duty_envelope: Option<Envelope>,

    // Block context applicable when this note started
    block_context_envelopes: Vec<(Envelope, f32)>,

    // Cached parameters (resolved at NoteOn or SetParameter relevant to this note)
    patch_str: String,              // Store resolved patch string
    noise_type_str: Option<String>, // Store resolved noise_type string
    note_tempo: f32,
    note_time_note: u8,
    note_base_duty_cycle: f32,

    // Oscillator/Noise State
    white_noise_rng_state: u32,
    lfsr_state: u16, // For periodic noise
    brown_noise_last_value: f32,
    pink_voss_values: [f32; 5],
    pink_voss_counter: u32,
    // --- New fields for LFSR pitch control ---
    lfsr_target_update_period_samples: f32, // Target period for LFSR update, in samples
    lfsr_update_accumulator: f32,           // Accumulator for LFSR update timing
    current_lfsr_output_bit: f32,           // Current output of the LFSR (-0.4 to 0.4)

    // Cached values updated periodically
    last_env_update_sample: u64,
    cached_volume_mod: f32,
    cached_velocity_mod: f32,
    cached_pitch_semitone_offset: f32,
    cached_duty_cycle: f32,
    cached_effective_midi: u8,
    cached_base_frequency: f32, // Cached base frequency for current MIDI note
    env_update_interval: u64,

    // Exponential smoothing parameters
    smoothed_volume_mod: f32,
    smoothed_velocity_mod: f32,
    smoothed_pitch_semitone_offset: f32,
    smoothed_duty_cycle: f32,
}

impl ActiveNote {
    fn next_sample(&mut self, sample_rate: usize) -> (f32, bool) {
        if self.current_phase == NotePhase::Off {
            return (0.0, true);
        }

        let smoothing_coeff = 0.25_f32;

        let should_update_envelopes = self.time_in_current_phase_samples == 0
            || (self
                .time_in_current_phase_samples
                .saturating_sub(self.last_env_update_sample))
                >= self.env_update_interval;

        if should_update_envelopes {
            self.last_env_update_sample = self.time_in_current_phase_samples;

            let time_in_current_phase_secs =
                self.time_in_current_phase_samples as f32 / sample_rate as f32;

            let (current_main_volume_envelope_opt, total_duration_for_phase_secs) =
                match self.current_phase {
                    NotePhase::Attack => {
                        // During Attack phase, use the attack envelope
                        (
                            self.attack_envelope.as_ref(),
                            self.attack_duration_samples.map_or_else(
                                || {
                                    if let Some(env) = self.attack_envelope.as_ref() {
                                        let dur_secs = calculate_envelope_duration_secs(
                                            env,
                                            self.note_tempo,
                                            self.note_time_note,
                                        );
                                        self.attack_duration_samples =
                                            Some((dur_secs * sample_rate as f32).max(1.0) as u64);
                                        dur_secs
                                    } else {
                                        0.1 // Default attack time if no envelope
                                    }
                                },
                                |dur| dur as f32 / sample_rate as f32,
                            ),
                        )
                    }
                    NotePhase::Sustain => {
                        // During Sustain phase, use the sustain envelope (looping)
                        (
                            self.sustain_envelope.as_ref(),
                            self.sustain_duration_samples.map_or_else(
                                || {
                                    if let Some(env) = self.sustain_envelope.as_ref() {
                                        let dur_secs = calculate_envelope_duration_secs(
                                            env,
                                            self.note_tempo,
                                            self.note_time_note,
                                        );
                                        self.sustain_duration_samples =
                                            Some((dur_secs * sample_rate as f32).max(1.0) as u64);
                                        dur_secs
                                    } else {
                                        1.0 // Default loop time if no envelope
                                    }
                                },
                                |dur| dur as f32 / sample_rate as f32,
                            ),
                        )
                    }
                    NotePhase::Release => {
                        // During Release phase, use the release envelope
                        (
                            self.release_envelope.as_ref(),
                            self.release_duration_samples.map_or_else(
                                || {
                                    // If no duration is set yet, calculate it from the envelope
                                    if let Some(env) = self.release_envelope.as_ref() {
                                        let dur_secs = calculate_envelope_duration_secs(
                                            env,
                                            self.note_tempo,
                                            self.note_time_note,
                                        );
                                        // Cache calculated duration (convert to samples)
                                        self.release_duration_samples =
                                            Some((dur_secs * sample_rate as f32) as u64);
                                        dur_secs
                                    } else {
                                        0.25 // Default release time if no envelope (250ms)
                                    }
                                },
                                |dur| dur as f32 / sample_rate as f32,
                            ),
                        )
                    }
                    NotePhase::Off => (None, 0.0),
                };

            let tempo = self.note_tempo;
            let time_note = self.note_time_note;
            let base_duty_cycle = self.note_base_duty_cycle;

            let mut block_volume_mod = 1.0;
            let mut block_velocity_mod = 1.0;
            let mut block_duty_mod: Option<f32> = None;
            let mut block_note_offset = 0i8;
            let mut block_pitch_semitone_offset = 0.0f32;
            let mut block_transpose_offset = 0i8;

            let mut total_samples_since_note_on = self.time_in_current_phase_samples;
            if self.current_phase == NotePhase::Release {
                if self.ads_duration_samples != u64::MAX {
                    total_samples_since_note_on = self
                        .ads_duration_samples
                        .saturating_add(self.time_in_current_phase_samples);
                } else {
                    total_samples_since_note_on = self.time_in_current_phase_samples;
                }
            }
            let total_time_since_note_on_secs =
                total_samples_since_note_on as f32 / sample_rate as f32;

            for (block_env, block_start_time_secs) in &self.block_context_envelopes {
                let time_relative_to_block_start =
                    total_time_since_note_on_secs - block_start_time_secs;
                if time_relative_to_block_start >= 0.0 {
                    let block_env_total_dur_secs =
                        calculate_envelope_duration_secs(block_env, tempo, time_note);
                    let evaluated_value = if block_env_total_dur_secs > 0.0 {
                        block_env.evaluate(
                            time_relative_to_block_start,
                            tempo,
                            time_note,
                            Some(block_env_total_dur_secs),
                        )
                    } else {
                        block_env.points.last().map_or(0.0, |p| p.value)
                    };
                    match block_env.target.as_str() {
                        "volume" => block_volume_mod = evaluated_value,
                        "velocity" => block_velocity_mod = evaluated_value,
                        "duty" => block_duty_mod = Some(evaluated_value.clamp(0.0, 1.0)),
                        "note" => block_note_offset = evaluated_value.round() as i8,
                        "pitch" => block_pitch_semitone_offset = evaluated_value,
                        "transpose" => block_transpose_offset = evaluated_value.round() as i8,
                        _ => {}
                    }
                }
            }

            if let Some(env) = current_main_volume_envelope_opt {
                // Use explicit envelope for current phase
                let evaluated_vol = env.evaluate(
                    time_in_current_phase_secs,
                    tempo,
                    time_note,
                    Some(total_duration_for_phase_secs),
                );
                self.cached_volume_mod = evaluated_vol;
            } else if self.current_phase == NotePhase::Attack {
                // No attack envelope: fall back to block volume
                self.cached_volume_mod = block_volume_mod;
            } else if self.current_phase == NotePhase::Sustain {
                // No sustain envelope: hold the last value of the attack envelope if present
                if let Some(attack_env) = &self.attack_envelope {
                    // Determine full attack duration in seconds
                    let attack_dur_secs = self
                        .attack_duration_samples
                        .map(|samples| samples as f32 / sample_rate as f32)
                        .unwrap_or_else(|| {
                            calculate_envelope_duration_secs(attack_env, tempo, time_note)
                        });
                    let hold_val = attack_env.evaluate(
                        attack_dur_secs,
                        tempo,
                        time_note,
                        Some(attack_dur_secs),
                    );
                    self.cached_volume_mod = hold_val;
                } else {
                    self.cached_volume_mod = block_volume_mod;
                }
            } else if self.current_phase == NotePhase::Release {
                // During release without envelope, or after envelope end: silent
                self.cached_volume_mod = 0.0;
            }

            if self.current_phase == NotePhase::Release
                && self.time_in_current_phase_samples
                    == self.release_duration_samples.unwrap_or(0).saturating_sub(1)
                && self.release_duration_samples.unwrap_or(0) > 0
            {
                if let Some(env) = &self.release_envelope {
                    if let Some(last_point) = env.points.last() {
                        if last_point.value == 0.0_f32 {
                            self.cached_volume_mod = 0.0_f32;
                        }
                    }
                } else {
                    self.cached_volume_mod = 0.0_f32;
                }
            }

            let ads_phase_duration_for_mod_secs = if self.ads_duration_samples == u64::MAX {
                1.0
            } else {
                self.ads_duration_samples as f32 / sample_rate as f32
            };
            let time_for_mod_env_eval_secs = if self.current_phase == NotePhase::Attack
                || self.current_phase == NotePhase::Sustain
            {
                if self.ads_duration_samples == u64::MAX {
                    time_in_current_phase_secs
                } else {
                    time_in_current_phase_secs.min(ads_phase_duration_for_mod_secs)
                }
            } else {
                ads_phase_duration_for_mod_secs
            };

            self.cached_velocity_mod = block_velocity_mod;
            if let Some(p_env) = &self.pitch_envelope {
                self.cached_pitch_semitone_offset = p_env.evaluate(
                    time_for_mod_env_eval_secs,
                    tempo,
                    time_note,
                    Some(ads_phase_duration_for_mod_secs),
                );
            } else {
                self.cached_pitch_semitone_offset = block_pitch_semitone_offset;
            }
            if let Some(d_env) = &self.duty_envelope {
                self.cached_duty_cycle = d_env
                    .evaluate(
                        time_for_mod_env_eval_secs,
                        tempo,
                        time_note,
                        Some(ads_phase_duration_for_mod_secs),
                    )
                    .clamp(0.0, 1.0);
            } else {
                self.cached_duty_cycle = block_duty_mod.unwrap_or(base_duty_cycle);
            }

            let old_effective_midi = self.cached_effective_midi;
            let midi_note_offset = block_note_offset as i16;
            let midi_transpose_offset = block_transpose_offset as i16;
            self.cached_effective_midi =
                (self.pitch_midi as i16 + midi_note_offset + midi_transpose_offset).clamp(0, 127)
                    as u8;

            log::debug!(
                "ActiveNote Pre-Recalc (note_id: {}): self.pitch_midi={}, block_note_offset={}, block_transpose_offset={}, new_cached_effective_midi={}",
                self.note_id, self.pitch_midi, midi_note_offset, midi_transpose_offset, self.cached_effective_midi
            );
            log::debug!(
                "ActiveNote Pre-Recalc (note_id: {}): old_cached_base_freq={}",
                self.note_id,
                self.cached_base_frequency
            );

            if self.cached_effective_midi != old_effective_midi {
                self.cached_base_frequency =
                    440.0 * 2.0_f32.powf((self.cached_effective_midi as f32 - 69.0) / 12.0);
                log::debug!(
                    "ActiveNote Post-Recalc (note_id: {}): RECALCULATED cached_base_freq={} using effective_midi={}",
                    self.note_id, self.cached_base_frequency, self.cached_effective_midi
                );
            } else {
                log::debug!(
                    "ActiveNote Post-Recalc (note_id: {}): SKIPPED recalculation of cached_base_freq. Kept: {}",
                    self.note_id, self.cached_base_frequency
                );
            }

            // --- Update LFSR target period based on pitch envelope ---
            // Base LFSR update frequency (e.g., 2000 Hz) corresponds to a period in samples.
            // If sample_rate is 0, default to a sensible period to avoid div by zero.
            let base_lfsr_period_at_zero_offset = if sample_rate > 0 {
                (sample_rate as f32 / 2000.0).max(1.0)
            } else {
                24.0 // e.g., for 48kHz / 2kHz
            };

            self.lfsr_target_update_period_samples = (base_lfsr_period_at_zero_offset
                * 2.0_f32.powf(-self.cached_pitch_semitone_offset / 12.0))
            .max(1.0);
            // Note: Using cached_pitch_semitone_offset for now, could use smoothed later if needed
            // and if smoothing is applied to lfsr_target_update_period_samples itself.

            // End of envelope update block (if should_update_envelopes)
        }

        // --- LFSR state update (happens every sample, but LFSR itself updates based on period) ---
        // This check ensures we only try to use this for patches that might be periodic noise.
        // A more robust way would be to check self.patch_str if it's relevant only for "periodic_noise".
        if self.patch_str == "periodic_noise"
            || (self.patch_str == "noise"
                && self
                    .noise_type_str
                    .as_deref()
                    .map_or(false, |s| s == "periodic"))
        {
            self.lfsr_update_accumulator += 1.0;
            if self.lfsr_update_accumulator >= self.lfsr_target_update_period_samples {
                self.lfsr_update_accumulator -= self.lfsr_target_update_period_samples;

                // Corrected LFSR logic (removed unnecessary dereferences `*`)
                let bit = ((self.lfsr_state >> 0)
                    ^ (self.lfsr_state >> 2)
                    ^ (self.lfsr_state >> 3)
                    ^ (self.lfsr_state >> 5))
                    & 1;
                self.lfsr_state = (self.lfsr_state >> 1) | (bit << 15);
                if self.lfsr_state == 0 {
                    self.lfsr_state = 1;
                } // Avoid LFSR getting stuck at 0
                self.current_lfsr_output_bit = (bit as f32 * 2.0 - 1.0) * 0.4; // Scale to match other oscillators
            }
        }

        self.smoothed_volume_mod +=
            (self.cached_volume_mod - self.smoothed_volume_mod) * smoothing_coeff;
        self.smoothed_velocity_mod +=
            (self.cached_velocity_mod - self.smoothed_velocity_mod) * smoothing_coeff;
        self.smoothed_pitch_semitone_offset += (self.cached_pitch_semitone_offset
            - self.smoothed_pitch_semitone_offset)
            * smoothing_coeff;
        self.smoothed_duty_cycle +=
            (self.cached_duty_cycle - self.smoothed_duty_cycle) * smoothing_coeff;

        // For Release phase, ensure volume reaches exactly zero at the end
        if self.current_phase == NotePhase::Release {
            // If we're at the very end of the release phase
            if let Some(release_dur) = self.release_duration_samples {
                if release_dur > 0
                    && self.time_in_current_phase_samples >= release_dur.saturating_sub(32)
                {
                    // Force volume to reach exactly zero in the last few samples
                    let remaining = release_dur.saturating_sub(self.time_in_current_phase_samples);
                    // Linear fade in last few samples to avoid click
                    let fade_factor = remaining as f32 / 32.0;
                    // Ensure volume goes smoothly to zero in the final samples
                    self.smoothed_volume_mod = self.cached_volume_mod * fade_factor.min(1.0);

                    // If at the very end, ensure exactly zero
                    if self.time_in_current_phase_samples >= release_dur.saturating_sub(1) {
                        self.smoothed_volume_mod = 0.0;
                        println!(
                            "Note {}: End of release - volume forced to zero",
                            self.note_id
                        );
                    }
                }
            }
        }

        let final_frequency =
            self.cached_base_frequency * 2.0_f32.powf(self.smoothed_pitch_semitone_offset / 12.0);

        self.phase_osc += final_frequency / sample_rate as f32;
        self.phase_osc = self.phase_osc.fract();

        let raw_sample = generate_patch(
            &self.patch_str,
            self.phase_osc,
            self.smoothed_duty_cycle,
            self.noise_type_str.as_deref(),
            &mut self.white_noise_rng_state,
            &mut self.lfsr_state,
            &mut self.brown_noise_last_value,
            &mut self.pink_voss_values,
            &mut self.pink_voss_counter,
            self.current_lfsr_output_bit, // Pass the current LFSR output bit
        );

        let final_sample =
            raw_sample * self.smoothed_volume_mod * (self.velocity * self.smoothed_velocity_mod);

        self.time_in_current_phase_samples += 1;

        match self.current_phase {
            NotePhase::Attack => {
                // At the end of attack envelope, always switch to sustain (hold last level) until NoteOff
                if let Some(attack_duration) = self.attack_duration_samples {
                    if self.time_in_current_phase_samples >= attack_duration {
                        self.current_phase = NotePhase::Sustain;
                        self.time_in_current_phase_samples = 0;
                        self.last_env_update_sample = 0;
                        println!(
                            "Note {}: Attack ended, transitioning to Sustain phase.",
                            self.note_id
                        );
                    }
                }
            }
            NotePhase::Sustain => {
                // In sustain phase, we loop the sustain envelope if there is one
                if let Some(sustain_duration) = self.sustain_duration_samples {
                    if sustain_duration > 0
                        && self.time_in_current_phase_samples >= sustain_duration
                    {
                        // Loop the sustain envelope by resetting the time counter
                        self.time_in_current_phase_samples = 0;
                        self.last_env_update_sample = 0;
                        println!("Note {}: Sustain phase loop point reached, restarting sustain envelope.", self.note_id);
                    }
                }
                // Sustain keeps looping indefinitely until note off
            }
            NotePhase::Release => {
                // At the end of release envelope, turn off the note
                if let Some(release_duration) = self.release_duration_samples {
                    if self.time_in_current_phase_samples >= release_duration {
                        // Force volume to exactly zero when turning off
                        self.smoothed_volume_mod = 0.0;
                        self.cached_volume_mod = 0.0;
                        self.current_phase = NotePhase::Off;
                        println!(
                            "Note {}: Release ended, forcing volume to zero and turning off.",
                            self.note_id
                        );
                    }
                }
            }
            NotePhase::Off => {
                // Note is already off, nothing to do
            }
        }
        (final_sample, self.current_phase == NotePhase::Off)
    }
}

/// Basic subtractive synthesizer instrument.
#[derive(Debug)]
pub struct BasicSynth {
    active_notes: Vec<ActiveNote>,
    sample_rate: usize,
    current_transpose: i8,
    active_block_envelopes: HashMap<String, (Envelope, f32)>, // Target -> (Env, StartTimeSecs)
    static_volume: f32,
    // Store global tempo/time_signature for converting MetricDuration for release_duration
    current_tempo: f32,
    current_time_note: u8, // e.g. 4 for quarter note beats
    // Store resolved string representations of default patch/noise for efficiency
    default_patch_str: String,
    default_noise_type_str: Option<String>,
}

impl BasicSynth {
    pub fn new(rate: usize) -> Self {
        BasicSynth {
            active_notes: Vec::new(),
            sample_rate: rate,
            current_transpose: 0,
            active_block_envelopes: HashMap::new(),
            static_volume: 1.0,
            current_tempo: 120.0,                    // Default tempo
            current_time_note: 4,                    // Default time signature base
            default_patch_str: "square".to_string(), // Default patch
            default_noise_type_str: None,            // Default noise type (often implies white)
        }
    }
}

impl Instrument for BasicSynth {
    fn process_event(&mut self, timed_event: &TimedMusicalEvent, rate: usize) {
        if self.sample_rate != rate && rate > 0 {
            // Update sample_rate if it has changed and is valid
            // log::debug!("BasicSynth sample rate updated from {} to {}", self.sample_rate, rate);
            self.sample_rate = rate;
        }
        let current_time_samples_absolute =
            (timed_event.time_seconds * self.sample_rate as f32) as u64;

        match &timed_event.event {
            MusicalEventType::NoteOn {
                note_id,
                pitch_midi,
                velocity,
                parameters,
                attack_envelope,
                sustain_envelope,
                release_envelope,
                other_envelopes,
            } => {
                // All notes are indefinite until NoteOff by default
                // Debug: show envelope availability before and after block-level fallback
                let had_attack = attack_envelope.is_some();
                let had_sustain = sustain_envelope.is_some();
                let had_release = release_envelope.is_some();
                // let ads_duration_samples = u64::MAX; // Placeholder, will be determined below

                let note_params = parameters.clone(); // Clone to potentially use later, or just use `parameters` directly

                // Use explicit ADSR from the event or fall back to block-level envelopes
                let attack_env = attack_envelope.clone().or_else(|| {
                    self.active_block_envelopes
                        .get("attack_envelope")
                        .map(|(e, _)| e.clone())
                });
                let sustain_env = sustain_envelope.clone().or_else(|| {
                    self.active_block_envelopes
                        .get("sustain_envelope")
                        .map(|(e, _)| e.clone())
                });
                let release_env = release_envelope.clone().or_else(|| {
                    self.active_block_envelopes
                        .get("release_envelope")
                        .map(|(e, _)| e.clone())
                });

                println!(
                    "NoteOn ID {}: event_envs[atk={},sus={},rel={}] sel_envs[atk={},sus={},rel={}]",
                    *note_id,
                    had_attack,
                    had_sustain,
                    had_release,
                    attack_env.is_some(),
                    sustain_env.is_some(),
                    release_env.is_some()
                );

                let mut mod_envs: Vec<Envelope> = other_envelopes.clone();
                let mut pitch_env_opt: Option<Envelope> = None;
                if let Some(idx) = mod_envs.iter().position(|e| e.target == "pitch") {
                    pitch_env_opt = Some(mod_envs.remove(idx));
                }
                let mut duty_env_opt: Option<Envelope> = None;
                if let Some(idx) = mod_envs.iter().position(|e| e.target == "duty") {
                    duty_env_opt = Some(mod_envs.remove(idx));
                }
                let resolved_patch = note_params
                    .get("patch")
                    .and_then(|p| {
                        if let ParamValue::String(s) = p {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| self.default_patch_str.clone());

                let resolved_noise_type = note_params
                    .get("noise_type")
                    .and_then(|p| {
                        if let ParamValue::String(s) = p {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| self.default_noise_type_str.clone());

                let note_tempo_val = note_params
                    .get("tempo")
                    .and_then(|p| {
                        if let ParamValue::Number(n) = p {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(self.current_tempo);

                let note_time_note_val = note_params
                    .get("time_note")
                    .and_then(|p| {
                        if let ParamValue::Number(n) = p {
                            Some(n.round() as u8)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(self.current_time_note);

                let note_base_duty_val = note_params
                    .get("duty")
                    .and_then(|p| {
                        if let ParamValue::Number(n) = p {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.5);

                // Determine fixed duration of the note using timed_event.real_duration
                let note_fixed_duration_samples_opt: Option<u64> =
                    if timed_event.real_duration > 0.0f32 {
                        // real_duration is f32
                        Some((timed_event.real_duration * self.sample_rate as f32).max(1.0) as u64)
                    } else {
                        // If real_duration is 0 or not positive, this will lead to
                        // final_ads_duration_samples being u64::MAX (indefinite),
                        // and attack envelope duration logic will handle it appropriately.
                        None
                    };

                let final_ads_duration_samples =
                    note_fixed_duration_samples_opt.unwrap_or(u64::MAX);

                let mut final_attack_duration_samples: Option<u64> = None;
                if attack_env.is_some() {
                    if let Some(fixed_dur_samples) = note_fixed_duration_samples_opt {
                        // If the note has a fixed duration, the attack envelope spans this duration.
                        final_attack_duration_samples = Some(fixed_dur_samples);
                    } else {
                        // Indefinite note with an attack envelope.
                        // Try to calculate duration from envelope points (if they are absolute).
                        // If envelope is percentage-based (like cosen!), this will likely return 0.
                        let calculated_env_dur_secs = if let Some(env_ref) = attack_env.as_ref() {
                            calculate_envelope_duration_secs(
                                env_ref,
                                note_tempo_val,
                                note_time_note_val,
                            )
                        } else {
                            0.0
                        }; // Should not happen due to attack_env.is_some() check

                        if calculated_env_dur_secs > 1e-6 {
                            // If envelope defines its own non-zero duration
                            final_attack_duration_samples =
                                Some((calculated_env_dur_secs * self.sample_rate as f32).max(1.0)
                                    as u64);
                        } else {
                            // Envelope is percentage-based (e.g. cosen!) or has zero intrinsic duration,
                            // and note is indefinite. Default to a short attack duration (e.g., 50ms).
                            final_attack_duration_samples =
                                Some((0.05 * self.sample_rate as f32).max(1.0) as u64);
                            log::debug!("NoteOn ID {}: Indefinite note with percentage-based attack envelope. Defaulting attack_duration_samples to ~50ms.", *note_id);
                        }
                    }
                }

                // Explicit release_duration parameter, if provided
                let release_duration_smpls: Option<u64> =
                    note_params.get("release_duration").and_then(|param_value| {
                        if let ParamValue::Number(seconds) = param_value {
                            if *seconds >= 0.0 {
                                Some((*seconds * self.sample_rate as f32).max(1.0) as u64)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });

                let mut final_pitch_midi = *pitch_midi;
                // Apply key signature if applicable (simplified)
                if let Some(ParamValue::String(key_sig_str)) = note_params.get("key") {
                    let (base_midi, offset) = get_base_midi_and_offset(final_pitch_midi);
                    if offset == 0 {
                        // Only apply to natural notes
                        let key_adjust = get_key_signature_adjustment(key_sig_str, base_midi % 12);
                        final_pitch_midi = base_midi.saturating_add_signed(key_adjust);
                    }
                }
                final_pitch_midi = final_pitch_midi.saturating_add_signed(self.current_transpose);
                final_pitch_midi = final_pitch_midi.clamp(0, 127);

                log::info!(
                    "NoteOn Processing: input_midi={}, transpose={}, key_sig_adjusted_&_transposed_midi={}, raw_velocity={:.2}",
                    *pitch_midi,
                    self.current_transpose,
                    final_pitch_midi,
                    *velocity
                );

                let initial_base_freq =
                    440.0 * 2.0_f32.powf((final_pitch_midi as f32 - 69.0) / 12.0);
                log::info!(
                    "Calculated initial_base_freq = {} for final_pitch_midi={}",
                    initial_base_freq,
                    final_pitch_midi
                );

                let initial_rng_state = (*pitch_midi as u32)
                    .wrapping_add(current_time_samples_absolute as u32)
                    | 0x12345;
                let initial_lfsr_state: u16 = 0xACE1u16.rotate_left(*pitch_midi as u32);

                let mut active_block_envs_for_note: Vec<(Envelope, f32)> = Vec::new();
                for (_target, (block_env, block_start_time)) in self.active_block_envelopes.iter() {
                    active_block_envs_for_note.push((block_env.clone(), *block_start_time));
                }

                // Base LFSR update freq (e.g., 2kHz). Period = sample_rate / base_freq
                // This is the period when pitch offset is 0.
                // Initialize with a default, will be updated by pitch envelope.
                let initial_lfsr_period = if self.sample_rate > 0 {
                    (self.sample_rate as f32 / 2000.0).max(1.0)
                } else {
                    1.0 // Avoid division by zero if sample_rate isn't set yet
                };

                self.active_notes.push(ActiveNote {
                    note_id: *note_id,
                    pitch_midi: final_pitch_midi,
                    ads_duration_samples: final_ads_duration_samples,
                    velocity: *velocity,
                    phase_osc: 0.0,
                    current_phase: NotePhase::Attack, // Start in Attack phase
                    time_in_current_phase_samples: 0,
                    volume_at_release_start: 0.0,
                    attack_envelope: attack_env,
                    sustain_envelope: sustain_env,
                    release_envelope: release_env,

                    // Envelope durations
                    attack_duration_samples: final_attack_duration_samples,
                    sustain_duration_samples: None, // Will calculate if sustain_env exists
                    release_duration_samples: release_duration_smpls,
                    pitch_envelope: pitch_env_opt,
                    duty_envelope: duty_env_opt,
                    block_context_envelopes: active_block_envs_for_note,
                    patch_str: resolved_patch,
                    noise_type_str: resolved_noise_type,
                    note_tempo: note_tempo_val,
                    note_time_note: note_time_note_val,
                    note_base_duty_cycle: note_base_duty_val,
                    white_noise_rng_state: initial_rng_state,
                    lfsr_state: if initial_lfsr_state == 0 {
                        1
                    } else {
                        initial_lfsr_state
                    },
                    brown_noise_last_value: 0.0,
                    pink_voss_values: [0.0; 5],
                    pink_voss_counter: 0,
                    lfsr_target_update_period_samples: initial_lfsr_period,
                    lfsr_update_accumulator: 0.0,
                    current_lfsr_output_bit: 0.0, // Initial LFSR output can be 0
                    last_env_update_sample: 0,
                    cached_volume_mod: 0.0,
                    cached_velocity_mod: 1.0,
                    cached_pitch_semitone_offset: 0.0,
                    cached_duty_cycle: note_base_duty_val,
                    cached_effective_midi: final_pitch_midi,
                    cached_base_frequency: initial_base_freq,
                    env_update_interval: 64,
                    smoothed_volume_mod: 0.0,
                    smoothed_velocity_mod: 1.0,
                    smoothed_pitch_semitone_offset: 0.0,
                    smoothed_duty_cycle: note_base_duty_val,
                });
            }
            MusicalEventType::NoteOff { note_id } => {
                println!("Received NoteOff for ID: {}", note_id);
                if let Some(note) = self.active_notes.iter_mut().find(|n| n.note_id == *note_id) {
                    // When a NoteOff is received, transition to release phase if we're in attack or sustain
                    if note.current_phase == NotePhase::Attack
                        || note.current_phase == NotePhase::Sustain
                    {
                        if note.release_envelope.is_some() {
                            // Transition to release phase
                            note.current_phase = NotePhase::Release;
                            note.time_in_current_phase_samples = 0;
                            note.last_env_update_sample = 0;
                            note.volume_at_release_start = note.smoothed_volume_mod;
                            println!(
                                "Note {}: NoteOff received, transitioning to Release phase.",
                                note.note_id
                            );
                        } else {
                            // No release envelope, just turn off
                            note.current_phase = NotePhase::Off;
                            println!(
                                "Note {}: NoteOff received, no release envelope, turning off.",
                                note.note_id
                            );
                        }
                    } else {
                        println!("Note {}: NoteOff received but note is already in phase: {:?}. Ignoring.",
                                note.note_id, note.current_phase);
                    }
                }
            }
            MusicalEventType::SetParameter { key, value } => {
                // Global synth settings (tempo, patch, etc.)
                if key == "tempo" {
                    if let ParamValue::Number(n) = value {
                        self.current_tempo = *n;
                    }
                }
                if key == "time_note" {
                    if let ParamValue::Number(n) = value {
                        self.current_time_note = n.round() as u8;
                    }
                }
                if key == "transpose" {
                    if let ParamValue::Number(n) = value {
                        self.current_transpose = n.round() as i8;
                    }
                }
                if key == "volume" {
                    if let ParamValue::Number(n) = value {
                        self.static_volume = *n;
                    }
                }
                if key == "patch" {
                    if let ParamValue::String(s) = value {
                        self.default_patch_str = s.clone();
                    }
                }
                if key == "noise_type" {
                    if let ParamValue::String(s) = value {
                        self.default_noise_type_str = Some(s.clone());
                    } else if let ParamValue::Unset = value {
                        self.default_noise_type_str = None;
                    }
                }
                // Handle block envelopes
                if let ParamValue::Envelope(env) = value {
                    println!(
                        "SetParameter envelope '{}' registered at {:.3}s",
                        env.target, timed_event.time_seconds
                    );
                    self.active_block_envelopes
                        .insert(env.target.clone(), (env.clone(), timed_event.time_seconds));
                } else {
                    // Remove block envelope if a static value overrides it
                    if self.active_block_envelopes.contains_key(key) {
                        println!("Clearing block envelope for '{}' due to SetParameter", key);
                        self.active_block_envelopes.remove(key);
                    }
                }
                // Note: This doesn't update existing ActiveNotes with new global defaults like patch/tempo immediately.
                // ActiveNotes capture these at their NoteOn. This is typical synth behavior.
            }
            _ => {}
        }
    }

    fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32; 2]> {
        if self.active_notes.is_empty() {
            return vec![[0.0f32, 0.0f32]; count];
        }

        // Ensure the synth's internal understanding of sample_rate can be updated if needed,
        // though ActiveNote::next_sample takes the rate directly.
        // if self.sample_rate != rate && rate > 0 {
        //     self.sample_rate = rate;
        // }

        let mut buffer = vec![[0.0f32, 0.0f32]; count];

        for note_idx in (0..self.active_notes.len()).rev() {
            if let Some(note) = self.active_notes.get_mut(note_idx) {
                if note.current_phase == NotePhase::Off {
                } else {
                    for sample_idx in 0..count {
                        // Pass the host-provided 'rate' to next_sample.
                        let (sample_val, finished_during_buffer) = note.next_sample(rate);
                        if finished_during_buffer {
                            break;
                        }
                        buffer[sample_idx][0] += sample_val;
                        buffer[sample_idx][1] += sample_val;
                    }
                }
            }
        }

        if self.static_volume != 1.0 {
            for frame in buffer.iter_mut() {
                frame[0] *= self.static_volume;
                frame[1] *= self.static_volume;
            }
        }

        self.active_notes
            .retain(|note| note.current_phase != NotePhase::Off);

        buffer
    }

    fn is_idle(&self) -> bool {
        self.active_notes.is_empty()
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
}

// --- Helper functions ---
fn generate_patch(
    patch_str: &str,
    phase: f32,
    duty_cycle: f32,
    noise_type_str: Option<&str>,
    white_noise_rng_state: &mut u32,
    _lfsr_state: &mut u16,
    brown_noise_last_value: &mut f32,
    pink_voss_values: &mut [f32; 5],
    pink_voss_counter: &mut u32,
    current_lfsr_output_bit: f32, // New parameter for pre-calculated LFSR output
) -> f32 {
    let mut white_noise = || -> f32 {
        *white_noise_rng_state ^= *white_noise_rng_state << 13;
        *white_noise_rng_state ^= *white_noise_rng_state >> 17;
        *white_noise_rng_state ^= *white_noise_rng_state << 5;
        *white_noise_rng_state as f32 / u32::MAX as f32 * 2.0 - 1.0
    };

    match patch_str {
        "sine" => (phase * 2.0 * std::f32::consts::PI).sin() * 0.5,
        "triangle" => (2.0 * (phase * 2.0 - 1.0).abs() - 1.0) * 0.5,
        "sawtooth" => (phase * 2.0 - 1.0) * 0.5,
        "noise" => {
            match noise_type_str {
                Some("white") | None => white_noise() * 0.4,
                Some("periodic") => current_lfsr_output_bit, // Use the pre-calculated bit
                Some("brown") => {
                    let white = white_noise() * 0.05;
                    *brown_noise_last_value += white;
                    *brown_noise_last_value = brown_noise_last_value.clamp(-0.5, 0.5);
                    *brown_noise_last_value
                }
                Some("pink") => {
                    let white = white_noise();
                    *pink_voss_counter = pink_voss_counter.wrapping_add(1);
                    let mut sum = 0.0;
                    for i in 0..pink_voss_values.len() {
                        if (*pink_voss_counter & (1 << i)) != 0 {
                            pink_voss_values[i] = white * 0.2;
                        }
                        sum += pink_voss_values[i];
                    }
                    sum / (pink_voss_values.len() as f32 * 0.7)
                }
                Some(_other) => {
                    // debug!("Unknown noise_type: '{}', defaulting to white.", other);
                    white_noise() * 0.4
                }
            }
        }
        "white_noise" => white_noise() * 0.4,
        "periodic_noise" => current_lfsr_output_bit, // Use the pre-calculated bit
        _ => {
            let duty_deviation = (duty_cycle - 0.5).abs();
            let amplitude = (0.4 - duty_deviation * 0.5).max(0.05);
            if phase < duty_cycle {
                amplitude
            } else {
                -amplitude
            }
        }
    }
}

/// Helper function to calculate the total duration of an envelope by summing its points.
fn calculate_envelope_duration_secs(envelope: &Envelope, tempo: f32, time_note: u8) -> f32 {
    let mut total_duration = 0.0;
    for point in &envelope.points {
        total_duration += point.duration.to_seconds(tempo, time_note, None);
    }
    total_duration
}
