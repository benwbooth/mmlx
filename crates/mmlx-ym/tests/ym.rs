//! Exact YM2612: program a note, verify pitch and idleness.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use mmlx_ym::{gain_to_tl, midi_to_fnum, Ym2612, YM2612_CLOCK_NTSC};
use std::collections::HashMap;

#[test]
fn fnum_table_hits_concert_pitch() {
    let (block, fnum) = midi_to_fnum(69, YM2612_CLOCK_NTSC);
    // A440: block 4, FNUM ~1081 (matches the Plutiedev table value).
    assert_eq!(block, 4);
    assert!((fnum as i32 - 1081).abs() < 3, "fnum {fnum:#x}");
    assert!((mmlx_ym::fnum_to_freq(block, fnum, YM2612_CLOCK_NTSC) - 440.0).abs() < 2.0);
    assert_eq!(gain_to_tl(1.0), 0);
    assert_eq!(gain_to_tl(0.0), 127);
}

#[test]
fn chip_renders_programmed_note() {
    let mut voice = mmlx_ym::Ym2612Voice::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
    let mut parameters = HashMap::new();
    parameters.insert("fm_routing".to_string(), ParamValue::Number(0.0));
    parameters.insert("op1_level".to_string(), ParamValue::Number(0.6));
    parameters.insert("op2_level".to_string(), ParamValue::Number(0.8));
    parameters.insert("op3_level".to_string(), ParamValue::Number(0.8));
    parameters.insert("op4_level".to_string(), ParamValue::Number(0.8));
    voice.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.0,
            real_duration: 1.0,
            event: MusicalEventType::NoteOn {
                note_id: 7,
                pitch_midi: 69,
                velocity: 1.0,
                parameters,
                attack_envelope: None,
                sustain_envelope: None,
                release_envelope: None,
                other_envelopes: Vec::new(),
            },
            instrument_name: "ym".to_string(),
        },
        44100,
    );
    let buffer = voice.generate_samples(44100, 44100);
    let peak = buffer
        .iter()
        .map(|frame| frame[0].abs().max(frame[1].abs()))
        .fold(0.0f32, f32::max);
    assert!(peak > 0.01 && peak <= 1.0, "sane peak {peak}");
    // Autocorrelation at the A440 lag (100 samples @44100Hz): FM stacks are
    // harmonically rich, so zero-crossings overcount — correlate instead.
    let mid: Vec<f32> = buffer[22050..44100].iter().map(|frame| frame[0]).collect();
    let energy: f32 = mid.iter().map(|sample| sample * sample).sum();
    let lag = 100;
    let corr: f32 = mid
        .iter()
        .zip(mid[lag..].iter())
        .map(|(a, b)| a * b)
        .sum::<f32>()
        / energy;
    assert!(corr > 0.5, "440Hz periodic, corr {corr}");
    voice.process_event(
        &TimedMusicalEvent {
            time_seconds: 1.0,
            real_duration: 0.0,
            event: MusicalEventType::NoteOff { note_id: 7 },
            instrument_name: "ym".to_string(),
        },
        44100,
    );
    let _ = voice.generate_samples(44100, 44100);
    assert!(voice.is_idle());
}

#[test]
fn raw_register_writes_work() {
    let mut chip = Ym2612::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
    // Silence: key off everything, render, expect near-zero.
    for channel in 0..6u8 {
        let code = (channel % 3) | ((channel / 3) << 2);
        chip.write(0, 0x28, code);
    }
    let buffer = chip.render(1024);
    let peak = buffer
        .iter()
        .map(|frame| frame[0].abs())
        .fold(0.0f32, f32::max);
    assert!(peak < 0.01, "silent after reset, peak {peak}");
}
