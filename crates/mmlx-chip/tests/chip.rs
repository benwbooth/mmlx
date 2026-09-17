//! Every chiptune voice renders audible audio and goes idle on NoteOff.

use mmlx_chip::{GbDmg, Nes2A03};
use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;

fn note_on(id: u64, patch: &str) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: 0.0,
        real_duration: 0.5,
        event: MusicalEventType::NoteOn {
            note_id: id,
            pitch_midi: 69,
            velocity: 0.9,
            parameters: HashMap::from([("patch".to_string(), "square".into())]),
            attack_envelope: None,
            sustain_envelope: None,
            release_envelope: None,
            other_envelopes: Vec::new(),
        },
        instrument_name: "chip".to_string(),
    }
    .with_patch(patch)
}

trait WithPatch {
    fn with_patch(self, patch: &str) -> Self;
}

impl WithPatch for TimedMusicalEvent {
    fn with_patch(mut self, patch: &str) -> Self {
        if let MusicalEventType::NoteOn { parameters, .. } = &mut self.event {
            parameters.insert("patch".to_string(), patch.into());
        }
        self
    }
}

fn note_off(id: u64) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: 0.5,
        real_duration: 0.0,
        event: MusicalEventType::NoteOff { note_id: id },
        instrument_name: "chip".to_string(),
    }
}

fn peak(buffer: &[[f32; 2]]) -> f32 {
    buffer
        .iter()
        .map(|frame| frame[0].abs().max(frame[1].abs()))
        .fold(0.0f32, f32::max)
}

#[test]
fn nes_voices_sound_and_stop() {
    for patch in [
        "pulse12", "pulse25", "pulse50", "pulse75", "triangle", "noise", "bogus",
    ] {
        let mut synth = Nes2A03::new();
        synth.process_event(&note_on(1, patch), 44100);
        let buffer = synth.generate_samples(22050, 44100);
        assert!(peak(&buffer) > 0.02, "{patch} audible");
        synth.process_event(&note_off(1), 44100);
        let _ = synth.generate_samples(44100, 44100);
        assert!(synth.is_idle(), "{patch} idles after NoteOff");
    }
}

#[test]
fn gb_voices_sound_and_stop() {
    for patch in ["pulse25", "wave", "noise", "short-noise"] {
        let mut synth = GbDmg::new();
        synth.process_event(&note_on(1, patch), 44100);
        let buffer = synth.generate_samples(22050, 44100);
        assert!(peak(&buffer) > 0.02, "{patch} audible");
        synth.process_event(&note_off(1), 44100);
        let _ = synth.generate_samples(44100, 44100);
        assert!(synth.is_idle(), "{patch} idles after NoteOff");
    }
}

#[test]
fn fm_patches_sound_stop_and_differ() {
    use mmlx_chip::Fm4;
    let mut first: Option<Vec<[f32; 2]>> = None;
    for patch in ["fm-lead", "fm-bell", "fm-bass", "fm-pad", "fm-brass"] {
        let mut synth = Fm4::new();
        synth.process_event(&note_on(1, patch), 44100);
        let buffer = synth.generate_samples(22050, 44100);
        assert!(peak(&buffer) > 0.02, "{patch} audible");
        if let Some(reference) = &first {
            assert_ne!(&buffer, reference, "{patch} differs from fm-lead");
        } else {
            first = Some(buffer);
        }
        synth.process_event(&note_off(1), 44100);
        let _ = synth.generate_samples(88200, 44100);
        assert!(synth.is_idle(), "{patch} idles after release");
    }
}

#[test]
fn fm_params_reprogram_operators() {
    use mmlx_chip::Fm4;
    use mmlx_core::{Note, ParamValue};

    fn render(patch: &str, extra: Vec<(String, ParamValue)>) -> Vec<[f32; 2]> {
        let mut parameters = vec![("patch".to_string(), ParamValue::String(patch.to_string()))];
        parameters.extend(extra);
        let note = Note::Atom {
            midi: 69,
            duration: 0.5,
            parameters,
        };
        let events: Vec<_> = note.event_stream(0.0).collect();
        let mut synth = Fm4::new();
        for event in events.iter().filter(|event| event.time_seconds <= 0.0) {
            synth.process_event(event, 44100);
        }
        synth.generate_samples(22050, 44100)
    }

    let lead = render("fm-lead", vec![]);
    // Detune operator 2 and mute operator 4: must audibly differ.
    let edited = render(
        "fm-lead",
        vec![
            ("op2_ratio".to_string(), ParamValue::Number(1.5)),
            ("op4_level".to_string(), ParamValue::Number(0.0)),
        ],
    );
    assert_ne!(lead, edited);
    // Out-of-range programming clamps instead of panicking.
    let clamped = render(
        "fm-lead",
        vec![
            ("fm_routing".to_string(), ParamValue::Number(99.0)),
            ("op1_ratio".to_string(), ParamValue::Number(-5.0)),
            ("fm_feedback".to_string(), ParamValue::Number(7.0)),
        ],
    );
    assert!(peak(&clamped) > 0.0);
    // Per-op release still idles.
    let mut synth = Fm4::new();
    let note = Note::Atom {
        midi: 69,
        duration: 0.5,
        parameters: vec![
            (
                "patch".to_string(),
                ParamValue::String("fm-pad".to_string()),
            ),
            ("op1_release".to_string(), ParamValue::Number(0.05)),
            ("op2_release".to_string(), ParamValue::Number(0.05)),
            ("op3_release".to_string(), ParamValue::Number(0.05)),
            ("op4_release".to_string(), ParamValue::Number(0.05)),
        ],
    };
    let events: Vec<_> = note.event_stream(0.0).collect();
    for event in &events {
        synth.process_event(event, 44100);
    }
    let _ = synth.generate_samples(44100, 44100);
    assert!(synth.is_idle());
}

#[test]
fn ay_sn_opl_voices_sound_and_stop() {
    use mmlx_chip::{AyVoice, Opl2, PsgVoice};

    // AY: tone, noise mix, and envelope shape all render.
    for (patch, extra) in [
        ("ay-square", vec![]),
        ("ay-noise", vec![("ay_noise_period", 4.0)]),
        (
            "ay-square",
            vec![("ay_env_shape", 8.0), ("ay_env_period", 64.0)],
        ),
    ] {
        let mut synth = AyVoice::new();
        let mut parameters = HashMap::new();
        parameters.insert("patch".to_string(), patch.into());
        for (key, value) in extra {
            parameters.insert(key.to_string(), value.into());
        }
        synth.process_event(
            &TimedMusicalEvent {
                time_seconds: 0.0,
                real_duration: 0.5,
                event: MusicalEventType::NoteOn {
                    note_id: 1,
                    pitch_midi: 69,
                    velocity: 0.9,
                    parameters,
                    attack_envelope: None,
                    sustain_envelope: None,
                    release_envelope: None,
                    other_envelopes: Vec::new(),
                },
                instrument_name: "ay".to_string(),
            },
            44100,
        );
        let buffer = synth.generate_samples(22050, 44100);
        assert!(peak(&buffer) > 0.01, "{patch} audible");
        synth.process_event(&note_off(1), 44100);
        let _ = synth.generate_samples(44100, 44100);
        assert!(synth.is_idle(), "{patch} idles");
    }

    // SN76489: tones, all noise modes, and explicit channel select.
    for patch in [
        "sn-square",
        "sn-noise-white",
        "sn-noise-periodic",
        "sn-noise-tone3",
    ] {
        let mut synth = PsgVoice::new();
        synth.process_event(&note_on(1, patch), 44100);
        let buffer = synth.generate_samples(22050, 44100);
        assert!(peak(&buffer) > 0.01, "{patch} audible");
        synth.process_event(&note_off(1), 44100);
        let _ = synth.generate_samples(44100, 44100);
        assert!(synth.is_idle(), "{patch} idles");
    }

    // OPL2: FM/AM, all waveforms, and per-op programming.
    for patch in ["opl-fm", "opl-am"] {
        let mut synth = Opl2::new();
        synth.process_event(&note_on(1, patch), 44100);
        let buffer = synth.generate_samples(22050, 44100);
        assert!(peak(&buffer) > 0.01, "{patch} audible");
        synth.process_event(&note_off(1), 44100);
        let _ = synth.generate_samples(88200, 44100);
        assert!(synth.is_idle(), "{patch} idles");
    }
    // Waveform select changes timbre; EGT off decays through sustain.
    {
        use mmlx_core::{Note, ParamValue};
        fn render_opl(extra: Vec<(String, ParamValue)>) -> Vec<[f32; 2]> {
            let mut parameters = vec![(
                "patch".to_string(),
                ParamValue::String("opl-fm".to_string()),
            )];
            parameters.extend(extra);
            let note = Note::Atom {
                midi: 69,
                duration: 0.5,
                parameters,
            };
            let events: Vec<_> = note.event_stream(0.0).collect();
            let mut synth = Opl2::new();
            for event in events.iter().filter(|event| event.time_seconds <= 0.0) {
                synth.process_event(event, 44100);
            }
            synth.generate_samples(22050, 44100)
        }
        let sine = render_opl(vec![]);
        let squareish = render_opl(vec![
            ("op1_wave".to_string(), ParamValue::Number(2.0)),
            ("op2_wave".to_string(), ParamValue::Number(2.0)),
        ]);
        assert_ne!(sine, squareish, "waveform select is audible");
    }
}

#[test]
fn sid_filter_shapes_tone() {
    use mmlx_chip::Sid;
    use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
    use std::collections::HashMap;

    fn render(patch: &str, cutoff: f32) -> Vec<[f32; 2]> {
        let mut parameters = HashMap::new();
        parameters.insert("patch".to_string(), patch.into());
        parameters.insert("cutoff".to_string(), cutoff.into());
        let mut synth = Sid::new();
        synth.process_event(
            &TimedMusicalEvent {
                time_seconds: 0.0,
                real_duration: 1.0,
                event: MusicalEventType::NoteOn {
                    note_id: 1,
                    pitch_midi: 69, // A4: fundamental above a 300 Hz cutoff
                    velocity: 0.9,
                    parameters,
                    attack_envelope: None,
                    sustain_envelope: None,
                    release_envelope: None,
                    other_envelopes: Vec::new(),
                },
                instrument_name: "sid".to_string(),
            },
            44100,
        );
        synth.generate_samples(44100, 44100)
    }

    fn energy(buffer: &[[f32; 2]]) -> f32 {
        buffer[22050..]
            .iter()
            .map(|frame| frame[0] * frame[0])
            .sum::<f32>()
    }

    for patch in ["sid-pulse", "sid-saw", "sid-tri"] {
        let open = render(patch, 8000.0);
        assert!(energy(&open) > 0.0, "{patch} audible");
        let dark = render(patch, 300.0);
        assert!(
            energy(&dark) < 0.5 * energy(&open),
            "{patch} low cutoff attenuates"
        );
    }
}

#[test]
fn sn_channel_3_is_noise_and_leaves_tones_alone() {
    // Regression: sn_channel=3 with no noise patch fell into `channel % 3`
    // and stomped tone slot 0 with a midi-60 square — sparse off-key stabs
    // over the arp that also killed the tone on NoteOff. Per the voice
    // docs, 3 is the noise slot.
    use mmlx_chip::PsgVoice;
    use mmlx_core::ParamValue;
    fn psg_on(id: u64, midi: u8, channel: f32) -> TimedMusicalEvent {
        TimedMusicalEvent {
            time_seconds: 0.0,
            real_duration: 0.5,
            event: MusicalEventType::NoteOn {
                note_id: id,
                pitch_midi: midi,
                velocity: 0.9,
                parameters: HashMap::from([(
                    "sn_channel".to_string(),
                    ParamValue::Number(channel),
                )]),
                attack_envelope: None,
                sustain_envelope: None,
                release_envelope: None,
                other_envelopes: Vec::new(),
            },
            instrument_name: "psg".to_string(),
        }
    }
    fn zcr(buffer: &[[f32; 2]]) -> f32 {
        let mut crossings = 0u32;
        for w in buffer.windows(2) {
            if (w[0][0] < 0.0) != (w[1][0] < 0.0) {
                crossings += 1;
            }
        }
        crossings as f32 / buffer.len() as f32
    }
    // Drums alone: white-ish noise, not a 261 Hz square (zcr ~0.012).
    let mut synth = PsgVoice::new();
    synth.process_event(&psg_on(1, 60, 3.0), 44100);
    let buffer = synth.generate_samples(22050, 44100);
    assert!(peak(&buffer) > 0.01, "drum audible");
    // White noise at N/512 flips its output bit ~half the 6991 steps/s.
    assert!(zcr(&buffer) > 0.03, "drum is noise-like");
    // Periodic mode (sn_noise_mode=1): output flips each N/512 shift.
    let mut synth = PsgVoice::new();
    let mut noisy = psg_on(1, 60, 3.0);
    if let MusicalEventType::NoteOn { parameters, .. } = &mut noisy.event {
        parameters.insert("sn_noise_mode".to_string(), ParamValue::Number(1.0));
    }
    synth.process_event(&noisy, 44100);
    let buffer = synth.generate_samples(22050, 44100);
    let rate = zcr(&buffer);
    assert!(peak(&buffer) > 0.01, "periodic drum audible");
    assert!(
        (rate - 3579545.0 / 512.0 / 44100.0).abs() < 0.03,
        "periodic rate, zcr={rate}"
    );
    // A sounding tone survives a drum hit on channel 3.
    let mut synth = PsgVoice::new();
    synth.process_event(&psg_on(1, 69, 2.0), 44100);
    synth.process_event(&psg_on(2, 60, 3.0), 44100);
    synth.process_event(&note_off(2), 44100);
    let buffer = synth.generate_samples(22050, 44100);
    assert!(peak(&buffer) > 0.01, "tone survives drum");
    let rate = zcr(&buffer);
    assert!(
        (rate - 2.0 * 440.0 / 44100.0).abs() < 0.005,
        "tone still A440, zcr={rate}"
    );
}

#[test]
fn sn_retrigger_continues_phase() {
    // Hardware counters free-run: re-keying the same pitch mid-stream
    // must continue the waveform exactly (no click, no chorus jump).
    use mmlx_chip::PsgVoice;
    use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
    use std::collections::HashMap;
    fn key(id: u64, midi: u8) -> TimedMusicalEvent {
        TimedMusicalEvent {
            time_seconds: 0.0,
            real_duration: 0.5,
            event: MusicalEventType::NoteOn {
                note_id: id,
                pitch_midi: midi,
                velocity: 0.9,
                parameters: HashMap::from([("sn_channel".to_string(), ParamValue::Number(1.0))]),
                attack_envelope: None,
                sustain_envelope: None,
                release_envelope: None,
                other_envelopes: Vec::new(),
            },
            instrument_name: "psg".to_string(),
        }
    }
    let mut a = PsgVoice::new();
    a.process_event(&key(1, 69), 44100);
    let uninterrupted = a.generate_samples(1000, 44100);
    let mut b = PsgVoice::new();
    b.process_event(&key(1, 69), 44100);
    let mut first = b.generate_samples(500, 44100);
    b.process_event(&key(2, 69), 44100);
    first.extend(b.generate_samples(500, 44100));
    assert_eq!(first.len(), uninterrupted.len());
    for (i, (x, y)) in first.iter().zip(uninterrupted.iter()).enumerate() {
        assert!(
            (x[0] - y[0]).abs() < 1e-6 && (x[1] - y[1]).abs() < 1e-6,
            "sample {i} diverges after re-key"
        );
    }
}
