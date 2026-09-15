//! Decompile a VGM file to an mmlx song module.
//!
//! Usage: `cargo run -p mmlx-vgm --example decompile -- <song.vgm> <name> <out.rs>`
//! The VGM stays out of the repo; only the transcription is committed.

use mmlx_vgm::*;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: decompile <song.vgm> <name> <out.rs>");
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
    let mut voices: Vec<(String, Vec<TrackNote>, Vec<TrackNote>)> = Vec::new();
    for channel in 0..6u8 {
        let mine: Vec<TrackNote> = fm
            .iter()
            .filter(|note| note.voice == channel)
            .cloned()
            .collect();
        if mine.is_empty() {
            continue;
        }
        let (intro, looping) = split_loop(mine, loop_sample);
        voices.push((format!("ym{channel}"), intro, looping));
    }
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
        if mine.is_empty() {
            continue;
        }
        let (intro, looping) = split_loop(mine, loop_sample);
        voices.push((name.to_string(), intro, looping));
    }
    let name = &args[2];
    let src = emit_song(name, &voices_with_instruments(voices), tick, tempo);
    std::fs::write(&args[3], &src).expect("write song");
    println!("wrote {} ({} bytes)", args[3], src.len());
}

/// Attach the playback instrument per voice lane.
fn voices_with_instruments(
    voices: Vec<(String, Vec<TrackNote>, Vec<TrackNote>)>,
) -> Vec<(String, Vec<TrackNote>, Vec<TrackNote>)> {
    voices
        .into_iter()
        .map(|(voice, mut intro, mut looping)| {
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
            (instrument, intro, looping)
        })
        .collect()
}
