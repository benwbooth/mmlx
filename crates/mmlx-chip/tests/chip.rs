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
