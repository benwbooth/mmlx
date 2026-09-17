//! Compare a decompiled song against its VGM ground truth.
//!
//! Usage: `cargo run -p mmlx-vgm --example compare -- <song.vgm> [out.wav]`
//! Compares the committed transcription's event streams against the
//! snapped tracker model (times/pitches/params), then renders the loop
//! body through the exact YM2612 + PSG voices, optionally writing a WAV.
//!
//! NOTE: this example is wired to `alisia_stage1`; generalize by feature
//! or argv when more transcriptions land.

use mmlx_core::{Instrument, MusicalEventType};
use mmlx_vgm::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: compare <song.vgm> [out.wav]");
        std::process::exit(2);
    }
    let data = std::fs::read(&args[1]).expect("read VGM");
    let header = parse_header(&data).expect("header");
    let commands = parse_commands(&data, &header).expect("commands");
    let end = header.total_samples as u64;
    let fm = track_fm(&commands, header.ym2612_clock, end);
    let psg = track_psg(&commands, end);
    let all: Vec<TrackNote> = fm.iter().chain(psg.iter()).cloned().collect();
    let tick = detect_tick(&all);
    let loop_sample = header.loop_offset.and_then(|offset| {
        commands
            .iter()
            .find(|(command_offset, _, _)| *command_offset >= offset)
            .map(|(_, sample, _)| *sample)
    });
    println!("tick {tick}, loop sample {loop_sample:?}");

    // Reference model, snapped exactly like the generator snaps.
    // Compared in TICK domain: float seconds drift ~0.5us/tick, which would
    // false-mismatch millisecond rounding over a 40s loop.
    let tick_secs = tick as f32 / 44100.0;
    let to_ticks = |seconds: f32| (seconds / tick_secs).round() as i64;
    let mut reference: Vec<(i64, i64, u8, String, Vec<(String, f32)>)> = Vec::new();
    for channel in 0..6u8 {
        let mine: Vec<TrackNote> = fm
            .iter()
            .filter(|note| note.voice == channel)
            .cloned()
            .collect();
        let (intro, looping) = split_loop(mine, loop_sample);
        for (notes, tag) in [(intro, "intro"), (looping, "loop")] {
            for note in snap_voice(&notes, tick) {
                let start = to_ticks(note.start as f32 / 44100.0);
                let duration = to_ticks(note.duration as f32 / 44100.0);
                let mut params: Vec<(String, f32)> = note
                    .params
                    .iter()
                    .cloned()
                    .chain([("ym_channel".to_string(), channel as f32)])
                    .collect();
                params.sort_by(|a, b| a.0.cmp(&b.0));
                reference.push((start, duration, note.midi, format!("ym:{tag}"), params));
            }
        }
    }
    for (voice, tag_voice) in [
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
        let (intro, looping) = split_loop(mine, loop_sample);
        // Must match the sn_channel pin in decompile's voice mapping.
        let channel = match tag_voice {
            "psg0" => 0.0,
            "psg1" => 1.0,
            "psg2" => 2.0,
            _ => 3.0,
        };
        for (notes, tag) in [(intro, "intro"), (looping, "loop")] {
            for note in snap_voice(&notes, tick) {
                // Sorted to match the song side's key order; carries the
                // note's own params too (sn_noise_mode on drum notes).
                let mut params = vec![
                    ("sn_channel".to_string(), channel),
                    (
                        "velocity".to_string(),
                        (note.velocity * 1000.0).round() / 1000.0,
                    ),
                ];
                params.extend(
                    note.params
                        .iter()
                        .map(|(key, value)| (key.clone(), (value * 1000.0).round() / 1000.0)),
                );
                params.sort_by(|a, b| a.0.cmp(&b.0));
                reference.push((
                    to_ticks(note.start as f32 / 44100.0),
                    to_ticks(note.duration as f32 / 44100.0),
                    note.midi,
                    format!("psg:{tag_voice}:{tag}"),
                    params,
                ));
            }
        }
    }

    // Song streams, split the same way: first pull is the intro, the
    // second the loop body (later pulls repeat it).
    let mut full = mmlx_songs::vgm::alisia_stage1::alisia_stage1();
    let intro_song = full.next().expect("intro body");
    let loop_song = full.next().expect("loop body");
    let mut song: Vec<(i64, i64, u8, String, Vec<(String, f32)>)> = Vec::new();
    for (note, tag) in [(intro_song, "intro"), (loop_song, "loop")] {
        for event in note.event_stream(0.0) {
            if let MusicalEventType::NoteOn {
                pitch_midi,
                velocity,
                parameters,
                ..
            } = &event.event
            {
                let instrument = event.instrument_name.clone();
                let mut params: Vec<(String, f32)> = parameters
                    .iter()
                    .filter_map(|(key, value)| match value {
                        mmlx_core::ParamValue::Number(number) => {
                            Some((key.clone(), (number * 1000.0).round() / 1000.0))
                        }
                        mmlx_core::ParamValue::String(_) => None,
                        _ => None,
                    })
                    .filter(|(key, _)| {
                        key != "tempo"
                            && key != "velocity"
                            && key != "time_note"
                            && key != "time_beat"
                    })
                    .collect();
                // FM loudness lives in TL params; velocity only matters for PSG.
                if instrument == "psg" {
                    params.push(("velocity".to_string(), (velocity * 1000.0).round() / 1000.0));
                }
                params.sort_by(|a, b| a.0.cmp(&b.0));
                song.push((
                    to_ticks(event.time_seconds),
                    to_ticks(event.real_duration),
                    *pitch_midi,
                    format!("{instrument}:{tag}"),
                    params,
                ));
            }
        }
    }

    // Compare per section-tag-independent multisets keyed by voice family.
    let key = |(start, duration, midi, tag, params): (i64, i64, u8, String, Vec<(String, f32)>)| {
        let family = tag.split(':').next().unwrap_or("").to_string();
        let section = if tag.ends_with(":intro") {
            "intro"
        } else {
            "loop"
        }
        .to_string();
        (
            start,
            duration,
            midi,
            family,
            section,
            params
                .into_iter()
                .map(|(key, value)| (key, (value * 1000.0).round() as i64))
                .collect::<Vec<_>>(),
        )
    };
    let mut reference_keys: Vec<_> = reference.into_iter().map(key).collect();
    let mut song_keys: Vec<_> = song.into_iter().map(key).collect();
    reference_keys.sort();
    song_keys.sort();
    println!(
        "reference notes: {}, song notes: {}",
        reference_keys.len(),
        song_keys.len()
    );
    let mut mismatches = 0;
    for (index, (reference, actual)) in reference_keys.iter().zip(song_keys.iter()).enumerate() {
        if reference != actual {
            mismatches += 1;
            if mismatches <= 8 {
                println!("MISMATCH #{index}:\n  ref  {reference:?}\n  song {actual:?}");
            }
        }
    }
    if reference_keys.len() != song_keys.len() {
        println!(
            "COUNT MISMATCH: ref {} vs song {}",
            reference_keys.len(),
            song_keys.len()
        );
        mismatches += 1;
    }
    if mismatches == 0 {
        println!("COMPARE EXACT PASS");
    } else {
        println!("COMPARE FAILED with {mismatches} mismatches");
        std::process::exit(1);
    }

    // Render the loop body through the exact voices for listening.
    let loop_events: Vec<_> = mmlx_songs::vgm::alisia_stage1::alisia_stage1()
        .into_iter()
        .nth(1)
        .expect("loop body")
        .event_stream(0.0)
        .collect();
    let end = loop_events
        .iter()
        .map(|event| event.time_seconds + event.real_duration)
        .fold(0.0f32, f32::max)
        .min(30.0);
    let mut ym = mmlx_ym::Ym2612Voice::new(mmlx_ym::YM2612_CLOCK_NTSC, 44100).expect("ym");
    let mut psg_voice = mmlx_chip::PsgVoice::new();
    let mut out: Vec<[f32; 2]> = Vec::new();
    let mut pending = &loop_events[..];
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
        out.extend(
            a.into_iter()
                .zip(b)
                .map(|(a, b)| [a[0] + b[0], a[1] + b[1]]),
        );
        now += frames as f32 / 44100.0;
    }
    let peak = out
        .iter()
        .map(|frame| frame[0].abs().max(frame[1].abs()))
        .fold(0.0f32, f32::max);
    println!(
        "rendered {:.1}s loop, peak {peak:.3}",
        out.len() as f32 / 44100.0
    );
    if args.len() >= 3 {
        write_wav(&args[2], &out, 44100);
        println!("wrote {}", args[2]);
    }
}

fn write_wav(path: &str, samples: &[[f32; 2]], rate: u32) {
    let mut bytes = Vec::new();
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
        for channel in frame {
            bytes.extend_from_slice(&((channel.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        }
    }
    std::fs::write(path, bytes).expect("write wav");
}
