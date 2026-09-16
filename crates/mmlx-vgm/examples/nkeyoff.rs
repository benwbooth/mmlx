//! Key-off isolation: instant-attack, max-release patch on channel 1.
//! On at t=0, off at t=0.3, render to 1.3s. A working release silences
//! the tail in <100ms; anything else means the off path is broken.
//!
//! Usage: `cargo run -p mmlx-vgm --example nkeyoff`
use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;

fn params(pairs: &[(&str, f32)]) -> HashMap<String, ParamValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), ParamValue::Number(*v)))
        .collect()
}

fn main() {
    let ch: u8 = std::env::args()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(1);
    let mut prog = params(&[
        ("ym_algo", 0.0),
        ("ym_feedback", 0.0),
        ("ym_channel", ch as f32),
    ]);
    for op in 1..=4 {
        for (k, v) in [
            ("ar", 31.0),
            ("dr", 0.0),
            ("sr", 0.0),
            ("sl", 0.0),
            ("rr", 15.0),
            ("tl", 20.0),
            ("mult", 1.0),
        ] {
            prog.insert(format!("op{op}_{k}"), ParamValue::Number(v));
        }
    }
    let mut ym = mmlx_ym::Ym2612Voice::new(mmlx_ym::YM2612_CLOCK_NTSC, 44100).expect("ym");
    ym.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.0,
            real_duration: 0.3,
            event: MusicalEventType::NoteOn {
                note_id: 1,
                pitch_midi: 69,
                velocity: 1.0,
                parameters: prog,
                attack_envelope: None,
                sustain_envelope: None,
                release_envelope: None,
                other_envelopes: Vec::new(),
            },
            instrument_name: "ym".to_string(),
        },
        44100,
    );
    let mut out = ym.generate_samples((44100.0 * 0.3) as usize, 44100);
    ym.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.3,
            real_duration: 0.0,
            event: MusicalEventType::NoteOff { note_id: 1 },
            instrument_name: "ym".to_string(),
        },
        44100,
    );
    out.extend(ym.generate_samples(44100, 44100));
    println!("t(s)     peak     rms");
    for (w, window) in out.chunks(2205).enumerate() {
        let peak = window
            .iter()
            .map(|f| f[0].abs().max(f[1].abs()))
            .fold(0.0f32, f32::max);
        let rms = (window
            .iter()
            .map(|f| (f[0] as f64) * (f[0] as f64))
            .sum::<f64>()
            / window.len() as f64)
            .sqrt();
        println!("{:.2}      {peak:.4}   {rms:.4}", w as f32 * 0.05);
    }
}
