//! Single-FM-note probe: keys one Alisia lead patch (midi 60) on at t=0,
//! off at t=1.0, renders 2s, prints per-100ms window peak/RMS to show the
//! envelope shape (attack/decay/sustain/release) and any discontinuities.
//!
//! Usage: `cargo run -p mmlx-vgm --example nprobe`
use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;

fn params(pairs: &[(&str, f32)]) -> HashMap<String, ParamValue> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), ParamValue::Number(*v)))
        .collect()
}

fn main() {
    // Lead patch straight from the compare print (algo 2, fb 7).
    let prog = params(&[
        ("ym_algo", 2.0),
        ("ym_feedback", 7.0),
        ("ym_channel", 4.0),
        ("op1_ar", 25.0),
        ("op1_dr", 10.0),
        ("op1_mult", 1.0),
        ("op1_rr", 5.0),
        ("op1_sl", 1.0),
        ("op1_sr", 0.0),
        ("op1_tl", 31.0),
        ("op2_ar", 25.0),
        ("op2_dr", 11.0),
        ("op2_mult", 5.0),
        ("op2_rr", 8.0),
        ("op2_sl", 5.0),
        ("op2_sr", 0.0),
        ("op2_tl", 15.0),
        ("op3_ar", 28.0),
        ("op3_dr", 13.0),
        ("op3_mult", 1.0),
        ("op3_rr", 6.0),
        ("op3_sl", 2.0),
        ("op3_sr", 0.0),
        ("op3_tl", 47.0),
        ("op4_ar", 14.0),
        ("op4_dr", 4.0),
        ("op4_mult", 1.0),
        ("op4_rr", 6.0),
        ("op4_sl", 2.0),
        ("op4_sr", 0.0),
        ("op4_tl", 20.0),
    ]);
    let mut ym = mmlx_ym::Ym2612Voice::new(mmlx_ym::YM2612_CLOCK_NTSC, 44100).expect("ym");
    ym.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.0,
            real_duration: 1.0,
            event: MusicalEventType::NoteOn {
                note_id: 1,
                pitch_midi: 60,
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
    // Render 2s in event order: on, then off at 1.0s.
    let mut out: Vec<[f32; 2]> = Vec::new();
    out.extend(ym.generate_samples(44100, 44100));
    ym.process_event(
        &TimedMusicalEvent {
            time_seconds: 1.0,
            real_duration: 0.0,
            event: MusicalEventType::NoteOff { note_id: 1 },
            instrument_name: "ym".to_string(),
        },
        44100,
    );
    out.extend(ym.generate_samples(44100, 44100));
    println!("t(s)     peak     rms");
    for (w, window) in out.chunks(4410).enumerate() {
        let peak = window
            .iter()
            .map(|f| f[0].abs().max(f[1].abs()))
            .fold(0.0f32, f32::max);
        let rms = (window.iter().map(|f| (f[0] as f64) * (f[0] as f64)).sum::<f64>()
            / window.len() as f64)
            .sqrt();
        println!("{:.1}      {peak:.4}   {rms:.4}", w as f32 * 0.1);
    }
}
