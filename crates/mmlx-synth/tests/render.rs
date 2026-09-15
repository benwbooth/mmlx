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

fn channel_energy(buffer: &[[f32; 2]], channel: usize) -> f32 {
    buffer
        .iter()
        .map(|frame| frame[channel] * frame[channel])
        .sum::<f32>()
        / buffer.len() as f32
}

fn render_panned(pan: f32) -> Vec<[f32; 2]> {
    let song = ser!([param!(tempo = 120, patch = "sine", pan = pan), c4q]);
    let events: Vec<_> = song.event_stream(0.0).collect();
    let mut synth = BasicSynth::new(44100);
    for ev in &events {
        if ev.time_seconds <= 0.0 {
            synth.process_event(ev, 44100);
        }
    }
    synth.generate_samples(11025, 44100)
}

#[test]
fn pan_moves_energy_between_channels() {
    // Skip the attack transient; compare steady state.
    let left = render_panned(0.0);
    let right = render_panned(1.0);
    let center = render_panned(0.5);
    let (ll, lr) = (
        channel_energy(&left[2000..], 0),
        channel_energy(&left[2000..], 1),
    );
    let (rl, rr) = (
        channel_energy(&right[2000..], 0),
        channel_energy(&right[2000..], 1),
    );
    let (cl, cr) = (
        channel_energy(&center[2000..], 0),
        channel_energy(&center[2000..], 1),
    );
    assert!(ll > 100.0 * lr, "hard left is silent on the right");
    assert!(rr > 100.0 * rl, "hard right is silent on the left");
    assert!(
        (cl / cr - 1.0).abs() < 0.05,
        "center is balanced: {cl} vs {cr}"
    );
}
