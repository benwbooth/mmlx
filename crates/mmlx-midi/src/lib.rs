//! `mmlx-midi`: SMF export/import for mmlx event streams.
//!
//! Core has no 16-voice / 7-bit limits; export quantizes down (velocity
//! `0..1 → 0..127`). `events_to_smf_multi` writes one track per instrument
//! (channels rotate 0..15); import rebuilds a `ser!` of atoms+rests.

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use mmlx_core::{MusicalEventType, Note, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;

/// GM program guess for our patch names (lead-ish voices).
fn patch_program(patch: &str) -> u8 {
    match patch {
        "sine" => 80,
        "square" => 81,
        "triangle" => 80,
        "sawtooth" => 82,
        "noise" => 127,
        _ => 81,
    }
}

/// Export events to Standard MIDI File bytes at `tempo_bpm`.
pub fn events_to_smf(
    events: &[TimedMusicalEvent],
    ticks_per_quarter: u16,
    tempo_bpm: f32,
) -> Vec<u8> {
    let indexed = index_pitches(events);
    let track = build_track(events, &indexed, 0, ticks_per_quarter, tempo_bpm, true);
    write_smf(Format::SingleTrack, ticks_per_quarter, vec![track])
}

/// Export one track per instrument (sorted by name, channels rotate 0..15).
/// Track 0 carries the tempo map; true multi-port cabling is future work.
pub fn events_to_smf_multi(
    events: &[TimedMusicalEvent],
    ticks_per_quarter: u16,
    tempo_bpm: f32,
) -> Vec<u8> {
    let indexed = index_pitches(events);
    let mut by_instrument: HashMap<&str, Vec<&TimedMusicalEvent>> = HashMap::new();
    for event in events {
        // Only sounding voices get tracks; params/rests/comments ride no MIDI.
        match &event.event {
            MusicalEventType::NoteOn { .. } | MusicalEventType::NoteOff { .. } => {
                by_instrument
                    .entry(event.instrument_name.as_str())
                    .or_default()
                    .push(event);
            }
            _ => {}
        }
    }
    let mut names: Vec<&str> = by_instrument.keys().copied().collect();
    names.sort();
    let mut tracks = vec![tempo_track(tempo_bpm)];
    for (channel, name) in names.iter().enumerate() {
        let owned: Vec<TimedMusicalEvent> = by_instrument[name]
            .iter()
            .map(|event| (*event).clone())
            .collect();
        tracks.push(build_track(
            &owned,
            &indexed,
            (channel % 16) as u8,
            ticks_per_quarter,
            tempo_bpm,
            false,
        ));
    }
    write_smf(Format::Parallel, ticks_per_quarter, tracks)
}

/// Import an SMF back into a `ser!`-style `Note`: atoms for notes (with
/// `velocity` when it differs from 100), rests for gaps.
pub fn smf_to_note(bytes: &[u8]) -> Result<Note, String> {
    let smf = Smf::parse(bytes).map_err(|err| format!("SMF parse: {err:?}"))?;
    let ticks_per_quarter = match smf.header.timing {
        Timing::Metrical(ticks) => u16::from(ticks) as f32,
        Timing::Timecode(_, _) => return Err("timecode timing unsupported".to_string()),
    };
    let mut tempo_bpm = 120.0;
    // (start_tick, dur_ticks, pitch, velocity)
    let mut notes: Vec<(u32, u32, u8, u8)> = Vec::new();
    for track in &smf.tracks {
        let mut tick = 0u32;
        let mut open: HashMap<(u8, u8), (u32, u8)> = HashMap::new();
        for event in track {
            tick += event.delta.as_int();
            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(micros)) => {
                    tempo_bpm = 60_000_000.0 / u32::from(micros) as f32;
                }
                TrackEventKind::Midi { channel, message } => {
                    let channel = u8::from(channel);
                    match message {
                        MidiMessage::NoteOn { key, vel } if u8::from(vel) > 0 => {
                            open.insert((channel, u8::from(key)), (tick, u8::from(vel)));
                        }
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            if let Some((start, velocity)) = open.remove(&(channel, u8::from(key)))
                            {
                                notes.push((
                                    start,
                                    tick.saturating_sub(start),
                                    u8::from(key),
                                    velocity,
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    notes.sort();
    let to_whole = |ticks: u32| ticks as f32 / ticks_per_quarter / 4.0;
    let _ = tempo_bpm;
    let mut items = Vec::new();
    let mut cursor = 0u32;
    for (start, duration, pitch, velocity) in notes {
        if start > cursor {
            items.push(Note::Rest {
                duration: to_whole(start - cursor),
                parameters: Vec::new(),
            });
        }
        let mut parameters = Vec::new();
        if velocity != 100 {
            parameters.push(("velocity".to_string(), ParamValue::Number(velocity as f32)));
        }
        items.push(Note::Atom {
            midi: pitch,
            duration: to_whole(duration.max(1)),
            parameters,
        });
        cursor = start + duration;
    }
    Ok(Note::Serial(items))
}

fn tempo_track(tempo_bpm: f32) -> Vec<TrackEvent<'static>> {
    let micros = (60_000_000.0 / tempo_bpm).round() as u32;
    vec![
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(micros))),
        },
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]
}

fn write_smf(format: Format, ticks_per_quarter: u16, tracks: Vec<Vec<TrackEvent>>) -> Vec<u8> {
    let smf = Smf {
        header: Header::new(format, Timing::Metrical(u15::from(ticks_per_quarter))),
        tracks,
    };
    let mut bytes = Vec::new();
    smf.write(&mut bytes).expect("SMF write");
    bytes
}

/// Map NoteOff id -> pitch from its paired NoteOn (O(n) build).
fn index_pitches(events: &[TimedMusicalEvent]) -> HashMap<u64, u8> {
    let mut map = HashMap::new();
    for event in events {
        if let MusicalEventType::NoteOn {
            note_id,
            pitch_midi,
            ..
        } = &event.event
        {
            map.insert(*note_id, *pitch_midi);
        }
    }
    map
}

fn build_track(
    events: &[TimedMusicalEvent],
    pitches: &HashMap<u64, u8>,
    channel: u8,
    ticks_per_quarter: u16,
    tempo_bpm: f32,
    with_tempo: bool,
) -> Vec<TrackEvent<'static>> {
    let to_ticks = |seconds: f32| -> u32 {
        (seconds * ticks_per_quarter as f32 * tempo_bpm / 60.0)
            .round()
            .max(0.0) as u32
    };

    // (abs_tick, order_key, kind): NoteOffs sort before NoteOns at same tick.
    enum Out {
        Off(u8),
        On(u8, u8),
        Program(u8),
    }
    let mut ordered: Vec<(u32, u8, Out)> = Vec::new();
    let mut program_sent = false;
    for event in events {
        let tick = to_ticks(event.time_seconds);
        match &event.event {
            MusicalEventType::NoteOn {
                pitch_midi,
                velocity,
                parameters,
                ..
            } => {
                if !program_sent {
                    let patch = parameters
                        .get("patch")
                        .and_then(|value| value.as_string())
                        .map(String::as_str)
                        .unwrap_or("square");
                    ordered.push((tick, 0, Out::Program(patch_program(patch))));
                    program_sent = true;
                }
                let velocity = (velocity * 127.0).round().clamp(0.0, 127.0) as u8;
                ordered.push((tick, 2, Out::On(*pitch_midi, velocity)));
            }
            MusicalEventType::NoteOff { note_id } => {
                if let Some(pitch) = pitches.get(note_id) {
                    ordered.push((tick, 1, Out::Off(*pitch)));
                }
            }
            _ => {}
        }
    }
    ordered.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let mut track: Vec<TrackEvent> = Vec::new();
    if with_tempo {
        let micros = (60_000_000.0 / tempo_bpm).round() as u32;
        track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(micros))),
        });
    }
    let mut last_tick = 0u32;
    for (tick, _, out) in ordered {
        let delta = u28::from(tick.saturating_sub(last_tick));
        last_tick = tick;
        let message = match out {
            Out::On(key, vel) => MidiMessage::NoteOn {
                key: u7::from(key.min(127)),
                vel: u7::from(vel.min(127)),
            },
            Out::Off(key) => MidiMessage::NoteOff {
                key: u7::from(key.min(127)),
                vel: u7::from(0),
            },
            Out::Program(program) => MidiMessage::ProgramChange {
                program: u7::from(program.min(127)),
            },
        };
        track.push(TrackEvent {
            delta,
            kind: TrackEventKind::Midi {
                channel: u4::from(channel),
                message,
            },
        });
    }
    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    track
}
