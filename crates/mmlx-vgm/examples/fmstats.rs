//! FM/PSG stem audit: renders the Alisia loop body through the exact
//! voices, writes per-stem WAVs, and prints objectively checkable stats
//! (peak/RMS/DC/clip/NaN/discontinuity). Deliberately NOT gated on
//! compare-exactness (compare exits before its render block on mismatch).
//!
//! Usage: `cargo run -p mmlx-vgm --example fmstats -- <out_prefix> [body]`
//! Writes `<out_prefix>_ym.wav`, `<out_prefix>_psg.wav`,
//! `<out_prefix>_mix.wav`. Body 0 = intro, 1 = loop (default).
use mmlx_core::Instrument;
use mmlx_core::TimedMusicalEvent;

fn stats(name: &str, samples: &[[f32; 2]], rate: u32) {
    let n = samples.len().max(1) as f32;
    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    let mut sum = 0.0f64;
    let mut clipped = 0u64;
    let mut railed = 0u64;
    let mut bad = 0u64;
    let mut max_disc = 0.0f32;
    let mut max_at = 0usize;
    let mut prev = [0.0f32, 0.0f32];
    let mut crossings = 0u64;
    for (i, frame) in samples.iter().enumerate() {
        for ch in 0..2 {
            let x = frame[ch];
            if !x.is_finite() {
                bad += 1;
                continue;
            }
            peak = peak.max(x.abs());
            sum_sq += (x as f64) * (x as f64);
            sum += x as f64;
            if x.abs() > 1.0 {
                clipped += 1;
            }
            if x.abs() >= 0.4999 {
                railed += 1;
            }
            let disc = (x - prev[ch]).abs();
            if disc > max_disc {
                max_disc = disc;
                max_at = i;
            }
            if i > 0 && x.signum() != prev[ch].signum() && x != 0.0 && prev[ch] != 0.0 {
                crossings += 1;
            }
            prev[ch] = x;
        }
    }
    let total = (samples.len() * 2) as f64;
    println!(
        "{name}: n={} peak={peak:.3} rms={:.4} dc={:+.5} clip%={:.2} rail%={:.2} naninf={bad} maxdisc={max_disc:.3}@{:.3}s zcr={:.4}",
        samples.len(),
        (sum_sq / n as f64).sqrt(),
        sum / total,
        100.0 * clipped as f64 / total,
        100.0 * railed as f64 / total,
        max_at as f32 / rate as f32,
        crossings as f64 / total,
    );
}

fn write_wav(path: &str, samples: &[[f32; 2]], rate: u32) {
    let mut bytes = Vec::with_capacity(44 + samples.len() * 4);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&((36 + samples.len() * 4) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(samples.len() as u32 * 4).to_le_bytes());
    for frame in samples {
        for ch in 0..2 {
            let clamped = frame[ch].clamp(-1.0, 1.0);
            bytes.extend_from_slice(&((clamped * 32767.0) as i16).to_le_bytes());
        }
    }
    std::fs::write(path, bytes).expect("write wav");
}

fn render(events: &[TimedMusicalEvent], end: f32, ym_only: bool, psg_only: bool) -> Vec<[f32; 2]> {
    let mut ym = mmlx_ym::Ym2612Voice::new(mmlx_ym::YM2612_CLOCK_NTSC, 44100).expect("ym");
    let mut psg_voice = mmlx_chip::PsgVoice::new();
    let mut out: Vec<[f32; 2]> = Vec::new();
    let mut pending = events;
    let mut now = 0.0f32;
    let step = 512;
    while now < end {
        while let Some((first, rest)) = pending.split_first() {
            if first.time_seconds > now {
                break;
            }
            match first.instrument_name.as_str() {
                "ym" => ym.process_event(first, 44100),
                _ => psg_voice.process_event(first, 44100),
            }
            pending = rest;
        }
        let frames = step.min(((end - now) * 44100.0) as usize);
        if frames == 0 {
            break;
        }
        let a = ym.generate_samples(frames, 44100);
        let b = psg_voice.generate_samples(frames, 44100);
        out.extend(a.into_iter().zip(b).map(|(a, b)| {
            let l = if psg_only { 0.0 } else { a[0] } + if ym_only { 0.0 } else { b[0] };
            let r = if psg_only { 0.0 } else { a[1] } + if ym_only { 0.0 } else { b[1] };
            [l, r]
        }));
        now += frames as f32 / 44100.0;
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("usage: fmstats <out_prefix> [body: 0=intro, 1=loop]");
        std::process::exit(2);
    }
    let body: usize = args.get(2).and_then(|b| b.parse().ok()).unwrap_or(1);
    let loop_events: Vec<_> = mmlx_songs::vgm::alisia_stage1::alisia_stage1()
        .into_iter()
        .nth(body)
        .expect("song body")
        .event_stream(0.0)
        .collect();
    // The stream emits lane-major runs (43% out of order); the live server
    // sorts per body before queueing, so audit renders must sort too —
    // otherwise the break-on-future drain drops whole spans to silence.
    let mut loop_events = loop_events;
    loop_events.sort_by(|a, b| {
        a.time_seconds
            .partial_cmp(&b.time_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let end = loop_events
        .iter()
        .map(|event| event.time_seconds + event.real_duration)
        .fold(0.0f32, f32::max)
        .min(30.0);
    println!("loop span: {end:.1}s, {} events", loop_events.len());
    // Per-stem renders share the event clock: render each from scratch so
    // voice state (envelopes, phases) matches the mix exactly.
    let ym = render(&loop_events, end, true, false);
    let psg = render(&loop_events, end, false, true);
    let mix: Vec<[f32; 2]> = ym
        .iter()
        .zip(psg.iter())
        .map(|(a, b)| [a[0] + b[0], a[1] + b[1]])
        .collect();
    stats("ym ", &ym, 44100);
    stats("psg", &psg, 44100);
    stats("mix", &mix, 44100);
    write_wav(&format!("{}_ym.wav", args[1]), &ym, 44100);
    write_wav(&format!("{}_psg.wav", args[1]), &psg, 44100);
    write_wav(&format!("{}_mix.wav", args[1]), &mix, 44100);
    println!("wrote {}_*.wav", args[1]);
}
