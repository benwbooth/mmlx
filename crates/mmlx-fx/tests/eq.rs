//! EQ behavior: flat passes through, boost/cut moves band energy.

use mmlx_fx::{BusMixer, Equalizer, GRAPHIC_FREQS_10};

fn sine_1k(frames: usize, sample_rate: f32) -> Vec<[f32; 2]> {
    (0..frames)
        .map(|i| {
            let sample = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sample_rate).sin() * 0.5;
            [sample, sample]
        })
        .collect()
}

fn energy(buffer: &[[f32; 2]]) -> f32 {
    buffer.iter().map(|frame| frame[0] * frame[0]).sum::<f32>() / buffer.len() as f32
}

#[test]
fn flat_eq_passes_through() {
    let mut buffer = sine_1k(44100, 44100.0);
    let reference = energy(&buffer);
    let mut eq = Equalizer::new(&GRAPHIC_FREQS_10, 44100.0);
    // Settle past the filter startup transient, then compare steady state.
    eq.process(&mut buffer);
    let settled = energy(&buffer[22050..]);
    let reference_settled = energy(&sine_1k(44100, 44100.0)[22050..]);
    assert!(
        (settled / reference_settled - 1.0).abs() < 0.02,
        "flat EQ is transparent: {settled} vs {reference_settled} (ref {reference})"
    );
}

#[test]
fn boost_and_cut_move_energy() {
    // 1kHz band is index 5 in GRAPHIC_FREQS_10.
    let mut boosted = sine_1k(44100, 44100.0);
    let mut eq = Equalizer::new(&GRAPHIC_FREQS_10, 44100.0);
    eq.set_gain(5, 12.0);
    eq.process(&mut boosted);
    let boosted_energy = energy(&boosted[22050..]);

    let mut cut = sine_1k(44100, 44100.0);
    let mut eq = Equalizer::new(&GRAPHIC_FREQS_10, 44100.0);
    eq.set_gain(5, -20.0);
    eq.process(&mut cut);
    let cut_energy = energy(&cut[22050..]);

    assert!(boosted_energy > 4.0 * cut_energy, "boost vs cut separation");
    let mut flat = sine_1k(44100, 44100.0);
    let mut eq = Equalizer::new(&GRAPHIC_FREQS_10, 44100.0);
    eq.process(&mut flat);
    let flat_energy = energy(&flat[22050..]);
    assert!(boosted_energy > 2.0 * flat_energy, "+12dB boosts");
    assert!(cut_energy < 0.1 * flat_energy, "-20dB cuts");
}

#[test]
fn bus_mixer_sums_gains_and_master() {
    use std::collections::HashMap;
    let mut mixer = BusMixer::new();
    mixer.set_gain("drums", 0.5);
    mixer.set_master(2.0);
    let buses: HashMap<String, Vec<[f32; 2]>> = HashMap::from([
        ("drums".to_string(), vec![[1.0, 1.0]; 4]),
        ("lead".to_string(), vec![[1.0, 0.0]; 2]),
    ]);
    let mixed = mixer.mixdown(&buses);
    // Longest bus wins; missing frames read as silence.
    assert_eq!(mixed.len(), 4);
    // drums: 2.0 * 0.5 * 1.0 = 1.0; lead: 2.0 * 1.0 * 1.0 on the left only.
    assert!(
        (mixed[0][0] - 3.0).abs() < 1e-6,
        "left sums both: {:?}",
        mixed[0]
    );
    assert!((mixed[0][1] - 1.0).abs() < 1e-6, "right is drums only");
    assert!((mixed[3][0] - 1.0).abs() < 1e-6, "tail is drums only");
}
