//! GME reference render: plays the source VGM through Game Music Emu's
//! own YM2612/SN76489 cores for A/B against our voices. Needs libgme at
//! runtime (`MMLX_GME_LIB`, else the `libgme.so.0` soname).
//!
//! Usage: `MMLX_GME_LIB=/path/to/libgme.so.0 cargo run -p mmlx-vgm --example gmeref -- <song.vgm> <seconds> <out_prefix>`
//! Writes `<out_prefix>_ref.wav` plus the same stats fmstats prints.
use mmlx_track::render_gme_muted;

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 || args.len() > 5 {
        eprintln!("usage: gmeref <song.vgm> <seconds> <out_prefix> [mute_mask]");
        std::process::exit(2);
    }
    let data = std::fs::read(&args[1]).expect("read VGM");
    let seconds: f32 = args[2].parse().expect("seconds");
    let mute_mask: i32 = args
        .get(4)
        .map(|mask| {
            let mask = mask.trim();
            if let Some(hex) = mask.strip_prefix("0x") {
                i32::from_str_radix(hex, 16).expect("mute mask")
            } else {
                mask.parse().expect("mute mask")
            }
        })
        .unwrap_or(0);
    for (i, name) in mmlx_track::gme_voice_names("vgm", &data)
        .expect("voice names")
        .iter()
        .enumerate()
    {
        println!("voice {i}: {name}");
    }
    let out = render_gme_muted("vgm", &data, 0, 44100, seconds, mute_mask).expect("gme render");
    stats("ref ", &out, 44100);
    write_wav(&format!("{}_ref.wav", args[3]), &out, 44100);
    println!("wrote {}_ref.wav", args[3]);
}
