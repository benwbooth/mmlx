//! Exact YM2612: program a note, verify pitch and idleness.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use mmlx_ym::{gain_to_tl, midi_to_fnum, Ym2612, YM2612_CLOCK_NTSC};
use std::collections::HashMap;

/// Key ch0, algo 7, single carrier on row 3 (`tl`), everything else fixed.
fn key_carrier(chip: &mut Ym2612, tl: u8) {
    chip.write(0, 0xB0, 0x07);
    chip.write(0, 0xB4, 0xC0);
    for row in [0u8, 1, 2, 3] {
        chip.write(0, 0x30 + row * 4, 1);
        chip.write(0, 0x40 + row * 4, if row == 3 { tl } else { 127 });
        chip.write(0, 0x50 + row * 4, 31);
        chip.write(0, 0x60 + row * 4, 5);
        chip.write(0, 0x70 + row * 4, 0);
        chip.write(0, 0x80 + row * 4, 0x28);
        chip.write(0, 0x90 + row * 4, 0);
    }
    chip.write(0, 0xA4, 0x24);
    chip.write(0, 0xA0, 0x3A);
    chip.write(0, 0x28, 0xF0);
}

fn sustained(chip: &mut Ym2612) -> Vec<[f32; 2]> {
    chip.render(44100)[44100 / 2..].to_vec()
}

#[test]
fn tl_attenuates_monotonically_and_stays_bipolar() {
    // Regression: DO_OUTPUT_INT once mixed in unsigned arithmetic, so its
    // >> 14 was logical — every negative half-wave wrapped near full
    // scale. TL looked dead and FM sounded like hash. A pure carrier must
    // fall monotonically with TL, swing both rails, and silence at 127.
    let mut prev = f32::INFINITY;
    for tl in [0u8, 20, 40, 64] {
        let mut chip = Ym2612::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
        key_carrier(&mut chip, tl);
        let sus = sustained(&mut chip);
        let rms = (sus.iter().map(|f| (f[0] as f64).powi(2)).sum::<f64>() / sus.len() as f64).sqrt()
            as f32;
        let peak = sus.iter().map(|f| f[0].abs()).fold(0.0f32, f32::max);
        let trough = sus.iter().map(|f| f[0]).fold(0.0f32, f32::min);
        assert!(rms < prev, "TL={tl}: rms {rms:.4} not below {prev:.4}");
        assert!(peak > 0.0 && trough < 0.0, "TL={tl}: output not bipolar");
        assert!(peak < 0.35, "TL={tl}: peak {peak:.3} looks wrapped");
        prev = rms;
    }
    let mut chip = Ym2612::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
    key_carrier(&mut chip, 127);
    let sus = sustained(&mut chip);
    let rms =
        (sus.iter().map(|f| (f[0] as f64).powi(2)).sum::<f64>() / sus.len() as f64).sqrt() as f32;
    assert!(rms < 1e-4, "TL=127 should silence, rms={rms}");
}

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
fn fnum_roundtrip_all_midis() {
    // Every MIDI note in hardware range must come back at its
    // equal-tempered frequency (catches block-boundary octave slips that
    // read as atonal garbage). Above ~midi 116 the 11-bit FNUM saturates,
    // a hardware limit, not a bug: assert exact saturation instead.
    for midi in 0..=116u8 {
        let (block, fnum) = midi_to_fnum(midi, YM2612_CLOCK_NTSC);
        assert!(fnum < 0x7FF, "midi {midi} should not saturate");
        let got = mmlx_ym::fnum_to_freq(block, fnum, YM2612_CLOCK_NTSC);
        let want = 440.0 * 2.0f32.powf((midi as f32 - 69.0) / 12.0);
        let rel = ((got - want) / want).abs();
        assert!(
            rel < 0.01,
            "midi {midi}: block {block} fnum {fnum:#x} -> {got}Hz, want {want}Hz"
        );
    }
    assert_eq!(midi_to_fnum(127, YM2612_CLOCK_NTSC), (7, 0x7FF));
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

fn voice_params(tl4: f32) -> HashMap<String, ParamValue> {
    let mut parameters = HashMap::from([
        ("ym_algo".to_string(), ParamValue::Number(7.0)),
        ("ym_feedback".to_string(), ParamValue::Number(0.0)),
        ("ym_channel".to_string(), ParamValue::Number(0.0)),
    ]);
    for op in 1..=4 {
        let tl = if op == 4 { tl4 } else { 127.0 };
        for (key, value) in [
            ("ar", 31.0),
            ("dr", 10.0),
            ("sr", 0.0),
            ("sl", 15.0),
            ("rr", 8.0),
            ("tl", tl),
            ("mult", 1.0),
        ] {
            parameters.insert(format!("op{op}_{key}"), ParamValue::Number(value));
        }
    }
    parameters
}

fn note_on(
    id: u64,
    midi: u8,
    time: f32,
    parameters: HashMap<String, ParamValue>,
) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: time,
        real_duration: 0.5,
        event: MusicalEventType::NoteOn {
            note_id: id,
            pitch_midi: midi,
            velocity: 1.0,
            parameters,
            attack_envelope: None,
            sustain_envelope: None,
            release_envelope: None,
            other_envelopes: Vec::new(),
        },
        instrument_name: "ym".to_string(),
    }
}

fn note_off(id: u64, time: f32) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: time,
        real_duration: 0.0,
        event: MusicalEventType::NoteOff { note_id: id },
        instrument_name: "ym".to_string(),
    }
}

fn rms_of(samples: &[[f32; 2]]) -> f32 {
    (samples.iter().map(|f| (f[0] as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
        as f32
}

fn crossings_hz(samples: &[[f32; 2]]) -> f32 {
    let mut crossings = 0u32;
    for w in samples.windows(2) {
        if (w[0][0] < 0.0) != (w[1][0] < 0.0) {
            crossings += 1;
        }
    }
    crossings as f32 / (samples.len() as f32 / 44100.0) / 2.0
}

#[test]
fn legato_glides_abutting_same_voice_without_reattack() {
    // Abutting same-program notes (pitch slides, ties) must glide: the
    // pitch retunes but the envelope keeps decaying. Re-keying restarts
    // the attack, which machine-guns slides into blips.
    use mmlx_ym::Ym2612Voice;
    let prog = voice_params(0.0);
    let mut voice = Ym2612Voice::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
    voice.process_event(&note_on(1, 69, 0.0, prog.clone()), 44100);
    let mut first = voice.generate_samples((44100.0 * 0.5) as usize, 44100);
    voice.process_event(&note_off(1, 0.5), 44100);
    voice.process_event(&note_on(2, 76, 0.5, prog), 44100);
    first.extend(voice.generate_samples((44100.0 * 0.5) as usize, 44100));
    let mid = rms_of(&first[13230..19845]);
    let late = rms_of(&first[30870..39690]);
    assert!(
        late < mid * 0.7,
        "envelope keeps decaying, mid={mid:.4} late={late:.4}"
    );
    assert!(
        (crossings_hz(&first[22932..26460]) - 659.0).abs() < 40.0,
        "retuned to E5"
    );
}

#[test]
fn steal_rekeys_on_program_change() {
    // Same tick, different program (voice steal): must fully re-key, or
    // the new note inherits the old attenuation and stays silent.
    use mmlx_ym::Ym2612Voice;
    let mut voice = Ym2612Voice::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
    voice.process_event(&note_on(1, 69, 0.0, voice_params(127.0)), 44100);
    let _ = voice.generate_samples((44100.0 * 0.3) as usize, 44100);
    voice.process_event(&note_off(1, 0.3), 44100);
    voice.process_event(&note_on(2, 69, 0.3, voice_params(0.0)), 44100);
    let second = voice.generate_samples((44100.0 * 0.5) as usize, 44100);
    assert!(rms_of(&second[11025..]) > 0.05, "stolen voice sounds");
}

fn ssg_params(ssg: f32) -> HashMap<String, ParamValue> {
    let mut parameters = HashMap::from([
        ("ym_algo".to_string(), ParamValue::Number(7.0)),
        ("ym_feedback".to_string(), ParamValue::Number(0.0)),
        ("ym_channel".to_string(), ParamValue::Number(0.0)),
    ]);
    for op in 1..=4 {
        let (tl, ssg) = if op == 4 { (20.0, ssg) } else { (127.0, 0.0) };
        for (key, value) in [
            ("ar", 31.0),
            ("dr", 31.0),
            ("sr", 0.0),
            ("sl", 15.0),
            ("rr", 8.0),
            ("tl", tl),
            ("mult", 1.0),
            ("ssg", ssg),
        ] {
            parameters.insert(format!("op{op}_{key}"), ParamValue::Number(value));
        }
    }
    parameters
}

#[test]
fn ssg_envelope_holds_where_plain_decay_dies() {
    // SSG-EG attack-hold shape with a silent sustain target: plain decay
    // must collapse while the SSG loop keeps sounding (and stays bounded).
    // (Shape 0 ends low like a normal decay; only hold/alternate shapes
    // sustain — the song's shape-0 notes are covered by lane A/B instead.)
    use mmlx_ym::Ym2612Voice;
    let render = |ssg: f32| {
        let mut voice = Ym2612Voice::new(YM2612_CLOCK_NTSC, 44100).expect("chip");
        voice.process_event(&note_on(1, 69, 0.0, ssg_params(ssg)), 44100);
        voice.generate_samples(44100, 44100)
    };
    let plain = render(0.0);
    let shaped = render(12.0);
    let tail = |buffer: &[[f32; 2]]| rms_of(&buffer[buffer.len() / 2..]);
    let peak = shaped
        .iter()
        .map(|f| f[0].abs().max(f[1].abs()))
        .fold(0.0f32, f32::max);
    assert!(tail(&shaped) > 5.0 * tail(&plain).max(1e-6), "ssg sustains");
    assert!(peak < 0.5, "ssg stays bounded, peak={peak}");
}
