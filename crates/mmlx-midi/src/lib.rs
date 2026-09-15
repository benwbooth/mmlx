//! `mmlx-midi`: SMF export from mmlx event streams.
//!
//! Core has no 16-voice / 7-bit limits; this crate quantizes down:
//! one track, channel 0, velocity `0..1 → 0..127`. Multi-port export and
//! SMF import are future work (see MMLX_SPEC.md §14).

use midly::num::{u15, u24, u28, u4, u7};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use mmlx_core::{MusicalEventType, TimedMusicalEvent};

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
    let ticks = u15::from(ticks_per_quarter);
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
            MusicalEventType::NoteOff { .. } => {
                // NoteOff carries no pitch; resolve from its paired NoteOn by id.
                if let Some(on) = events.iter().find_map(|candidate| match &candidate.event {
                    MusicalEventType::NoteOn { pitch_midi, .. }
                        if note_id_of(&candidate.event) == note_id_of(&event.event) =>
                    {
                        Some(*pitch_midi)
                    }
                    _ => None,
                }) {
                    ordered.push((tick, 1, Out::Off(on)));
                }
            }
            _ => {}
        }
    }
    ordered.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

    let mut track: Vec<TrackEvent> = Vec::new();
    let micros = (60_000_000.0 / tempo_bpm).round() as u32;
    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(micros))),
    });
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
                channel: u4::from(0),
                message,
            },
        });
    }
    track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    let smf = Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(ticks)),
        tracks: vec![track],
    };
    let mut bytes = Vec::new();
    smf.write(&mut bytes).expect("SMF write");
    bytes
}

fn note_id_of(event: &MusicalEventType) -> Option<u64> {
    match event {
        MusicalEventType::NoteOn { note_id, .. } | MusicalEventType::NoteOff { note_id } => {
            Some(*note_id)
        }
        _ => None,
    }
}
