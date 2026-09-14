//! Render test: event stream -> BasicSynth -> audible, then idle.

#[macro_use]
extern crate mmlx_core;

use mmlx_core::prelude::*;
use mmlx_synth::BasicSynth;

#[test]
fn renders_audible_note_then_idles() {
    let song = ser!([param!(tempo = 120, patch = "sine"), c4q]);
    let events: Vec<_> = song.event_stream(0.0).collect();
    assert!(!events.is_empty());

    let mut synth = BasicSynth::new(44100);
    for ev in events.iter().filter(|ev| ev.time_seconds <= 0.0) {
        synth.process_event(ev, 44100);
    }
    // Quarter at 120bpm = 0.5s; render the first half mid-note.
    let attack = synth.generate_samples(11025, 44100);
    let peak = attack.iter().map(|s| s[0].abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.01, "synth renders audible sine, peak {peak}");

    for ev in events.iter().filter(|ev| ev.time_seconds > 0.0) {
        synth.process_event(ev, 44100);
    }
    let _ = synth.generate_samples(44100, 44100);
    assert!(
        synth.is_idle(),
        "voice off after NoteOff without release env"
    );
}
