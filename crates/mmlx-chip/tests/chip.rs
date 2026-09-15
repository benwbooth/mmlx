//! Every chiptune voice renders audible audio and goes idle on NoteOff.

use mmlx_chip::{GbDmg, Nes2A03};
use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
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
