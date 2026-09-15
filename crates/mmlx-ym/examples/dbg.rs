fn main() {
    use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
    use std::collections::HashMap;
    let mut voice = mmlx_ym::Ym2612Voice::new(mmlx_ym::YM2612_CLOCK_NTSC, 44100).unwrap();
    let mut parameters = HashMap::new();
    parameters.insert("fm_routing".to_string(), mmlx_core::ParamValue::Number(3.0)); // parallel: op1 alone = cleanest
    parameters.insert("op1_level".to_string(), mmlx_core::ParamValue::Number(0.8));
    parameters.insert("op2_level".to_string(), mmlx_core::ParamValue::Number(0.0));
    parameters.insert("op3_level".to_string(), mmlx_core::ParamValue::Number(0.0));
    parameters.insert("op4_level".to_string(), mmlx_core::ParamValue::Number(0.0));
    voice.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.0,
            real_duration: 2.0,
            event: MusicalEventType::NoteOn {
                note_id: 1,
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
    let buf = voice.generate_samples(88200, 44100);
    let mid: Vec<f32> = buf[44100..88200].iter().map(|f| f[0]).collect();
    let energy: f32 = mid.iter().map(|x| x * x).sum();
    let mut best = (0usize, 0.0f32);
    for lag in [50, 100, 200, 25, 75, 150, 300, 10, 20, 400] {
        let c: f32 = mid
            .iter()
            .zip(mid[lag..].iter())
            .map(|(a, b)| a * b)
            .sum::<f32>()
            / energy;
        if c > best.1 {
            best = (lag, c);
        }
        eprintln!("lag {lag}: {c:.3}");
    }
    eprintln!("best lag={} (44100/{})", best.0, best.0);
}
