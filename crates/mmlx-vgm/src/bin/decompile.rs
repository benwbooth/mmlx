//! Decompile a VGM file to an mmlx song module.
//!
//! Usage: `cargo run -p mmlx-vgm --bin decompile -- <song.vgm> <name> <out.rs> [meter]`
//! The VGM stays out of the repo; only the transcription is committed.
//! Meter like `4/4` sets the bar grid (default `4/4`).

use mmlx_vgm::*;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 && args.len() != 5 {
        eprintln!("usage: decompile <song.vgm> <name> <out.rs> [meter, e.g. 4/4]");
        std::process::exit(2);
    }
    let data = std::fs::read(&args[1]).expect("read VGM");
    let header = parse_header(&data).expect("parse header");
    println!(
        "version {:08x}, {} samples ({:.1}s), loop {:?}, ym2612 clock {}",
        header.version,
        header.total_samples,
        header.total_samples as f32 / 44100.0,
        header.loop_offset,
        header.ym2612_clock
    );
    let commands = parse_commands(&data, &header).expect("parse commands");

    // Tick grid: most common note duration (drivers sequence on a grid;
    // command waits include sub-tick engine updates, so waits mislead).
    let end = header.total_samples as u64;
    let fm = track_fm(&commands, header.ym2612_clock, end);
    let psg = track_psg(&commands, end);
    let all: Vec<TrackNote> = fm.iter().chain(psg.iter()).cloned().collect();
    let tick = detect_tick(&all);
    let tempo = tempo_for_tick(tick);
    println!("tick grid: {tick} samples, tempo {tempo}");
    println!("FM notes: {}, PSG notes: {}", fm.len(), psg.len());
    let mut durations: HashMap<u64, usize> = HashMap::new();
    for note in fm.iter().chain(psg.iter()) {
        *durations.entry(note.duration).or_default() += 1;
    }
    let mut ranked: Vec<(u64, usize)> = durations.into_iter().collect();
    ranked.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    println!(
        "top durations (samples): {:?}",
        &ranked[..ranked.len().min(10)]
    );
    println!(
        "approx timbres: {}",
        fm.iter().filter(|note| note.approx).count()
    );

    // Split at the loop sample (first command at/after the loop offset).
    let loop_sample = header.loop_offset.and_then(|offset| {
        commands
            .iter()
            .find(|(command_offset, _, _)| *command_offset >= offset)
            .map(|(_, sample, _)| *sample)
    });
    println!("loop sample: {loop_sample:?}");

    // Voices: FM channels 0-5 pinned, PSG tones + noise.
    // Collect full (pre-split) lanes for role stats first: roles must be
    // stable across intro and loop, so they come from combined stats.
    let mut fm_lanes: Vec<(u8, Vec<TrackNote>)> = Vec::new();
    for channel in 0..6u8 {
        let mine: Vec<TrackNote> = fm
            .iter()
            .filter(|note| note.voice == channel)
            .cloned()
            .collect();
        if !mine.is_empty() {
            fm_lanes.push((channel, mine));
        }
    }
    let mut psg_lanes: Vec<(u8, &str, Vec<TrackNote>)> = Vec::new();
    for (voice, name) in [
        (10u8, "psg0"),
        (11, "psg1"),
        (12, "psg2"),
        (13, "psg_noise"),
    ] {
        let mine: Vec<TrackNote> = psg
            .iter()
            .filter(|note| note.voice == voice)
            .cloned()
            .collect();
        if !mine.is_empty() {
            psg_lanes.push((voice, name, mine));
        }
    }
    let roles = assign_roles(&fm_lanes, &psg_lanes);
    let mut voice_names: Vec<String> = fm_lanes
        .iter()
        .map(|(channel, _)| format!("ym{channel}"))
        .chain(psg_lanes.iter().map(|(_, name, _)| name.to_string()))
        .collect();
    voice_names.sort();
    voice_names.dedup();
    for voice in &voice_names {
        println!("voice {voice} -> role {}", roles[voice.as_str()]);
    }
    let mut voices: Vec<(String, String, Vec<TrackNote>, Vec<TrackNote>)> = Vec::new();
    for (channel, mine) in &fm_lanes {
        let (intro, looping) = split_loop(mine.clone(), loop_sample);
        let voice = format!("ym{channel}");
        voices.push((voice.clone(), roles[&voice].clone(), intro, looping));
    }
    for (_, name, mine) in &psg_lanes {
        let (intro, looping) = split_loop(mine.clone(), loop_sample);
        voices.push((name.to_string(), roles[*name].clone(), intro, looping));
    }
    let name = &args[2];
    let meter = args.get(4).map(String::as_str).unwrap_or("4/4");
    let bar_ticks = parse_meter(meter);
    let src = emit_song(
        name,
        &voices_with_instruments(voices),
        tick,
        tempo,
        bar_ticks,
    );
    std::fs::write(&args[3], &src).expect("write song");
    println!("wrote {} ({} bytes)", args[3], src.len());
}

/// Parse a meter like `4/4` into grid ticks per bar (a whole note is 128
/// ticks). Unknown shapes fall back to 4/4.
fn parse_meter(meter: &str) -> u64 {
    if let Some((num, den)) = meter.split_once('/') {
        if let (Ok(num), Ok(den)) = (num.parse::<u64>(), den.parse::<u64>()) {
            if den > 0 {
                return 128 * num / den;
            }
        }
    }
    128
}

/// Mean MIDI pitch and note count of a lane (combined intro+loop).
fn lane_stats(notes: &[TrackNote]) -> (usize, f32) {
    if notes.is_empty() {
        return (0, 0.0);
    }
    let sum: u64 = notes.iter().map(|note| note.midi as u64).sum();
    (notes.len(), sum as f32 / notes.len() as f32)
}

/// Thoughtful role names from combined lane stats (deterministic; the
/// musician renames as parts become clear):
/// - FM by mean pitch ascending: bass, harmony, harmony2, harmony3,
///   lead2, lead (ties break by channel; fewer lanes take a prefix of
///   this list from the bass end, keeping bass/lead stable).
/// - sparsest PSG lane: melody; the other two by mean ascending:
///   arp, arp2. Noise is always drums.
fn assign_roles(
    fm_lanes: &[(u8, Vec<TrackNote>)],
    psg_lanes: &[(u8, &str, Vec<TrackNote>)],
) -> HashMap<String, String> {
    let mut roles = HashMap::new();
    let fm_names = ["bass", "harmony", "harmony2", "harmony3", "lead2", "lead"];
    let mut by_pitch: Vec<(f32, u8, String)> = fm_lanes
        .iter()
        .map(|(channel, notes)| {
            let (_, mean) = lane_stats(notes);
            (mean, *channel, format!("ym{channel}"))
        })
        .collect();
    by_pitch.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
    // Anchor from the bass end so bass/lead stay stable when lanes drop;
    // with all six present this is bass..lead in pitch order.
    let offset = 6 - by_pitch.len().min(6);
    for (rank, (_, _, voice)) in by_pitch.iter().enumerate() {
        roles.insert(voice.clone(), fm_names[offset + rank].to_string());
    }
    // PSG: sparsest is the counter-line melody; the rest arpeggiate,
    // lower mean first.
    let mut tones: Vec<(usize, f32, String)> = Vec::new();
    for (_, name, notes) in psg_lanes {
        if *name == "psg_noise" {
            roles.insert(name.to_string(), "drums".to_string());
            continue;
        }
        let (count, mean) = lane_stats(notes);
        tones.push((count, mean, name.to_string()));
    }
    tones.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap()));
    if let Some((_, _, voice)) = tones.first() {
        roles.insert(voice.clone(), "melody".to_string());
    }
    let mut rest: Vec<(f32, String)> = tones
        .iter()
        .skip(1)
        .map(|(_, mean, voice)| (*mean, voice.clone()))
        .collect();
    rest.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for (index, (_, voice)) in rest.iter().enumerate() {
        let role = if index == 0 { "arp" } else { "arp2" };
        roles.insert(voice.clone(), role.to_string());
    }
    roles
}

/// Attach the playback instrument per voice lane, keeping the role.
fn voices_with_instruments(
    voices: Vec<(String, String, Vec<TrackNote>, Vec<TrackNote>)>,
) -> Vec<(String, String, Vec<TrackNote>, Vec<TrackNote>)> {
    voices
        .into_iter()
        .map(|(voice, role, mut intro, mut looping)| {
            let instrument = if voice.starts_with("ym") {
                // Pin the FM channel for deterministic chip allocation.
                let channel: u8 = voice[2..].parse().unwrap_or(0);
                for note in intro.iter_mut().chain(looping.iter_mut()) {
                    note.params.push(("ym_channel".to_string(), channel as f32));
                }
                "ym".to_string()
            } else {
                // Pin the PSG lane the same way (psg0-2 tone channels,
                // psg_noise shares the noise slot). Besides deterministic
                // allocation, the pin is the highlight lane key.
                let channel: u8 = match voice.as_str() {
                    "psg0" => 0,
                    "psg1" => 1,
                    "psg2" => 2,
                    _ => 3,
                };
                for note in intro.iter_mut().chain(looping.iter_mut()) {
                    note.params.push(("sn_channel".to_string(), channel as f32));
                }
                "psg".to_string()
            };
            (instrument, role, intro, looping)
        })
        .collect()
}
