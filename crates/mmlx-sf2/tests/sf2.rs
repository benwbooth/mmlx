//! SF2 render test against the in-memory minimal bank.

use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
use mmlx_sf2::minimal_soundfont;
use std::collections::HashMap;

fn note_on(id: u64, pitch: u8) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: 0.0,
        real_duration: 0.5,
        event: MusicalEventType::NoteOn {
            note_id: id,
            pitch_midi: pitch,
            velocity: 0.8,
            parameters: HashMap::new(),
            attack_envelope: None,
            sustain_envelope: None,
            release_envelope: None,
            other_envelopes: Vec::new(),
        },
        instrument_name: "sf2".to_string(),
    }
}

fn note_off(id: u64) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: 0.5,
        real_duration: 0.0,
        event: MusicalEventType::NoteOff { note_id: id },
        instrument_name: "sf2".to_string(),
    }
}

#[test]
fn sf2_renders_audible_then_idles() {
    let font = minimal_soundfont(44100, 440.0, 69);
    let mut synth = mmlx_sf2::Sf2Synth::new(&font, 44100);
    synth.process_event(&note_on(1, 69), 44100);
    assert_eq!(synth.voice_count(), 1);
    let buffer = synth.generate_samples(22050, 44100);
    let peak = buffer
        .iter()
        .map(|frame| frame[0].abs().max(frame[1].abs()))
        .fold(0.0f32, f32::max);
    assert!(peak > 0.01, "SF2 sine audible, peak {peak}");
    synth.process_event(&note_off(1), 44100);
    let _ = synth.generate_samples(44100, 44100);
    assert!(synth.is_idle(), "voice released after NoteOff");
}
