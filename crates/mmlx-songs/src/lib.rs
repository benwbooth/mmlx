//! mmlx-songs: example compositions exercising the full v0.1 DSL.
//!
//! `all_features()` is the conformance song: every currently implemented
//! language feature appears at least once. Keep it in sync with MMLX_SPEC.md.

#[macro_use]
extern crate mmlx_core;

use mmlx_core::env;
use mmlx_core::prelude::*;

pub mod vgm;

/// Compiled-in song registry for instant play (no JIT): maps the names of
/// the `fn() -> Note` songs to their constructors. This is the lotw-style
/// fast path — pressing play on a saved song starts in milliseconds;
/// only unsaved edits go through the evcxr JIT in mmlx-server.
pub fn song_by_name(name: &str) -> Option<fn() -> Note> {
    match name {
        "all_features" => Some(all_features),
        _ => None,
    }
}

/// Compiled-in generator songs: full performances (intro once, loop body
/// forever). The server pages one body per cycle with bounded memory
/// instead of replaying one collected stream.
pub fn song_stream_by_name(name: &str) -> Option<fn() -> NoteIterator> {
    match name {
        "alisia_stage1" => Some(|| Box::new(vgm::alisia_stage1::alisia_stage1())),
        _ => None,
    }
}

/// Exercise every currently implemented feature in one composition.
///
/// Covers: pitched atoms (plain/sharp/flat, dotted), rests, previous-pitch,
/// implicit durations, ties, `ser`/`par`/`parmin`/`forkseq`/`forkpar`,
/// `repeat`, `comment`, `param` (single/multi/unset, prefix shortening),
/// per-note attrs, key/tempo/transpose/gate, note-macro ornaments, and all
/// four envelope interpolations bound to several targets including pitch.
pub fn all_features() -> Note {
    let attack = cosenv!(0p0, 10p0.9);
    let release = cosenv!(0p0.9, 100p0);

    ser!([
        comment!("mmlx all-features conformance song"),
        // Global setup: multi-pair param + prefix shortening (time_b→time_beat).
        param!(tempo = 120, time_b = 4, time_n = 4, patch = "square"),
        param!(key = "Cmaj", gate = 0.9, volume = 0.8),
        // 1. Simple melody: atoms, dotted rhythm, rest.
        ser!([c4q, d4q, e4qd, rq, g4h]),
        // 2. Implicit durations + previous pitch + ties.
        //    c4q sets duration=q and pitch=C4; d4 inherits q; p repeats D4;
        //    e4q+q ties to a half note.
        ser!([c4q, d4, p, e4q, q, rq]),
        // 3. Accidentals: sharp, flat, double variants via generated consts.
        ser!([cs4q, df4q, fss4e, gff4e, bn4q]),
        // 4. Chords: parallel (max) + parallel-min (min advance).
        par!([c4h, e4h, g4h]),
        parmin!([c4h, e4q]),
        // 5. Forks: play alongside following material, advancing 0.
        ser!([c4q, forkseq!([e4q, g4q]), d4q]),
        ser!([c4q, forkpar!([e4q, g4q]), d4q]),
        // 6. Repeat marker: original + 2 extra hits.
        ser!([a4e, repeat!(2)]),
        // 7. Per-note attrs + unset; param scope + unset.
        ser!([
            param!(duty = 0.25),
            c5q!(velocity = 110),
            d5q!(duty =),
            param!(duty =),
            e5q,
        ]),
        // 8. Transpose + key signature + gate overlap.
        ser!([
            param!(transpose = 2, gate = 1.1),
            c4q,
            d4q,
            param!(transpose =, gate =),
        ]),
        // 9. Ornament macro: arpeggio fragment transposed to each trigger.
        ser!([
            param!(macro = ser!([c4i, e4i, g4i, c5i])),
            c4h,
            g4q,
            param!(macro =),
            c5h,
        ]),
        // 10. Envelopes: all four interpolations, several targets.
        //     Attack/release ADSR pair on a whole note (volume shaping).
        ser!([c4w!(
            attack_envelope = attack,
            release_envelope = release,
            release_duration = 0.15
        ),]),
        // Block-level volume swell + fine pitch blip + PWM sweep + stepped octave.
        ser!([
            param!(volume = env!(q0, e1, h0.5, q0)),
            param!(pitch = linenv!(q0, e2, q0)),
            param!(duty = cubenv!(q0.2, q0.8)),
            param!(note = expenv!(q0.01, q1)),
            c4h,
            param!(volume =, pitch =, duty =, note =),
        ]),
    ])
}

/// Infinite generator: stable `genawaiter` streaming (never materialize).
pub fn infinite_arp() -> impl Iterator<Item = Note> + Send + Sync + 'static {
    use genawaiter::sync::gen;
    use genawaiter::yield_;
    gen!({
        loop {
            yield_!(c4q);
            yield_!(e4q);
            yield_!(g4q);
            yield_!(c5q);
        }
    })
    .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mmlx_core::env;
    use mmlx_core::{note_stream_to_event_stream, MusicalEventType};

    fn collect_events(song: Note) -> Vec<mmlx_core::TimedMusicalEvent> {
        song.event_stream(0.0).collect()
    }

    #[test]
    fn song_builds_and_streams() {
        let events = collect_events(all_features());
        // Sanity: we get NoteOns, NoteOffs, rests, params and the comment.
        let mut on = 0;
        let mut off = 0;
        let mut rest_count = 0;
        let mut param_count = 0;
        let mut comment_count = 0;
        for ev in &events {
            match &ev.event {
                MusicalEventType::NoteOn { .. } => on += 1,
                MusicalEventType::NoteOff { .. } => off += 1,
                MusicalEventType::Rest { .. } => rest_count += 1,
                MusicalEventType::SetParameter { .. } => param_count += 1,
                MusicalEventType::Comment(_) => comment_count += 1,
                // Internal advance-accounting marker; never escapes into
                // final streams (par heaps consume it).
                MusicalEventType::BranchEnd => {}
            }
        }
        assert!(on > 20, "expected many notes, got {on}");
        // NOTE: `parmin!` truncates at the shortest branch, dropping NoteOffs
        // at/past the cutoff (inherited CP437 behavior) — so ons may exceed
        // offs by the number of truncated voices (here: 2).
        assert!(on >= off, "no orphan NoteOffs");
        assert!(on - off <= 2, "only parmin truncation drops NoteOffs");
        assert!(rest_count >= 2, "rests present");
        assert!(param_count >= 5, "params present");
        assert_eq!(comment_count, 1);
        // Forks (`forkseq`/`forkpar`) and macro overflow intentionally emit
        // events out of global order (a fork looks ahead, then the following
        // sibling goes back to the fork point) — so assert bounds instead.
        let mut min_t = f32::MAX;
        let mut max_t = 0.0f32;
        for ev in &events {
            assert!(ev.time_seconds >= -1e-6, "no negative timestamps");
            min_t = min_t.min(ev.time_seconds);
            max_t = max_t.max(ev.time_seconds);
        }
        assert!((min_t - 0.0).abs() < 1e-6, "stream starts at 0");
        assert!(max_t > 15.0, "full song spans many seconds, got {max_t}");
    }

    #[test]
    fn infinite_stream_takes_lazily() {
        let it = infinite_arp();
        let stream = note_stream_to_event_stream(Box::new(it), 0.0);
        let first: Vec<_> = stream.take(8).collect();
        assert_eq!(first.len(), 8);
    }

    #[test]
    fn full_song_streams_intro_then_loop_forever() {
        // One function, whole performance: the first pull differs from
        // every later pull (intro once, loop forever), loop pulls are
        // event-identical, and every pull sounds notes. The VGM compare
        // harness pins exact ground truth; this pins the shape.
        let bodies = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                let ctor = song_stream_by_name("alisia_stage1")
                    .expect("stream registry misses alisia_stage1");
                let mut full = ctor();
                std::iter::from_fn(|| full.next())
                    .take(4)
                    .map(|body| stream_events(&body))
                    .collect::<Vec<_>>()
            })
            .expect("spawn")
            .join()
            .expect("collect");
        assert_eq!(bodies.len(), 4);
        let (intro, loops) = bodies.split_first().expect("intro body");
        assert_eq!(loops.len(), 3);
        assert!(!intro.is_empty(), "intro sounds");
        assert!(!loops[0].is_empty(), "loop sounds");
        assert_ne!(*intro, loops[0], "intro differs from loop");
        for body in loops {
            assert_eq!(*body, loops[0], "loop repeats exactly");
        }
    }

    #[test]
    fn registry_resolves_compiled_songs() {
        // Instant-play path: every registry entry must build a Note.
        // The Alisia tree is huge; collect on a big stack like the
        // server's main thread does (default test threads are 2 MiB).
        for name in ["all_features"] {
            let events = std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || {
                    let ctor =
                        song_by_name(name).unwrap_or_else(|| panic!("registry misses {name}"));
                    collect_events(ctor())
                })
                .expect("spawn")
                .join()
                .expect("collect");
            assert!(
                events
                    .iter()
                    .any(|ev| matches!(ev.event, MusicalEventType::NoteOn { .. })),
                "{name} streams notes"
            );
        }
        assert!(song_by_name("no_such_song").is_none());
        assert!(song_stream_by_name("no_such_song").is_none());
        assert!(song_stream_by_name("alisia_stage1").is_some());
    }

    // Event streams with note IDs zeroed (IDs come from a global counter)
    // and params key-sorted (HashMap iteration order is random per map).
    fn stream_events(song: &Note) -> Vec<(u32, u32, String, String)> {
        song.event_stream(0.0)
            .map(|event| {
                let time = (event.time_seconds * 1000.0).round() as u32;
                let dur = (event.real_duration * 1000.0).round() as u32;
                let mut ev = event.event.clone();
                match &mut ev {
                    MusicalEventType::NoteOn { note_id, .. } => *note_id = 0,
                    MusicalEventType::NoteOff { note_id } => *note_id = 0,
                    _ => {}
                }
                let mut shown = format!("{ev:?}");
                if let MusicalEventType::NoteOn { parameters, .. } = &ev {
                    let mut params: Vec<String> = parameters
                        .iter()
                        .map(|(key, value)| format!("{key}={value:?}"))
                        .collect();
                    params.sort();
                    shown = format!("NoteOn({})", params.join(","));
                }
                (time, dur, shown, event.instrument_name)
            })
            .collect()
    }

    #[test]
    fn frontend_forms_are_equivalent() {
        // Bracket, paren-comma, and whitespace bodies expand identically.
        let old = ser!([
            param!(tempo = 120),
            c4q,
            d4q,
            rq,
            e4e,
            o,
            f4q!(velocity = 0.5),
            repeat!(1)
        ]);
        let new_parens = ser!(
            param!(tempo = 120),
            c4q,
            d4q,
            rq,
            e4e,
            o,
            f4q!(velocity = 0.5),
            repeat!(1)
        );
        let new_ws = ser!(param!(tempo = 120) c4q d4q rq e4e o f4q!(velocity = 0.5) repeat!(1));
        assert_eq!(stream_events(&old), stream_events(&new_parens));
        assert_eq!(stream_events(&old), stream_events(&new_ws));
        // Nesting mixes old and new forms freely.
        let old_par = par!([ser!([c4q, e4q]), ser!([g4q])]);
        let new_par = par!(ser!(c4q e4q) ser!(g4q));
        assert_eq!(stream_events(&old_par), stream_events(&new_par));
    }

    #[test]
    fn nested_ser_shares_ambient_params() {
        // Voice-program variables splice as nested ser! blocks; their
        // params must leak forward to following siblings (event-time
        // ambient threading, matching generate_events_recursive).
        use mmlx_core::ParamValue;
        fn tl_of(song: &Note, midi: u8) -> f32 {
            song.event_stream(0.0)
                .filter_map(|event| match &event.event {
                    MusicalEventType::NoteOn {
                        pitch_midi,
                        parameters,
                        ..
                    } if *pitch_midi == midi => match parameters.get("op4_tl") {
                        Some(ParamValue::Number(tl)) => Some(*tl),
                        _ => None,
                    },
                    _ => None,
                })
                .next()
                .unwrap_or(-1.0)
        }
        let nested = ser!([ser!([param!(op4_tl = 9)]), c4q]);
        let flat = ser!([param!(op4_tl = 9), c4q]);
        assert_eq!(tl_of(&nested, 60), 9.0);
        assert_eq!(tl_of(&nested, 60), tl_of(&flat, 60));
        // A bare multi-pair `param!(...)` (the single-use inline form)
        // splices identically too.
        let multi = ser!([param!(op4_tl = 9, op1_tl = 8), c4q]);
        assert_eq!(tl_of(&multi, 60), 9.0);
        // And programs can change mid-lane through nested blocks.
        let swap = ser!([
            ser!([param!(op4_tl = 9)]),
            c4q,
            ser!([param!(op4_tl = 22)]),
            d4q,
        ]);
        assert_eq!(tl_of(&swap, 60), 9.0);
        assert_eq!(tl_of(&swap, 62), 22.0);
    }

    #[test]
    fn bindings_splice_bare() {
        // Blocks borrow items and clone inside, so `let`-bound programs
        // splice bare with no moves and no explicit `.clone()`.
        use mmlx_core::{MusicalEventType, ParamValue};
        let voice: Note = ser!(param!(op4_tl = 9));
        let song = ser!(voice c4q voice d4q);
        let tls: Vec<f32> = song
            .event_stream(0.0)
            .filter_map(|event| match &event.event {
                MusicalEventType::NoteOn { parameters, .. } => match parameters.get("op4_tl") {
                    Some(ParamValue::Number(tl)) => Some(*tl),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(tls, vec![9.0, 9.0]);
    }

    #[test]
    fn legato_matches_tie() {
        // Tied-over-barline sustain: one NoteOn with the full duration,
        // grid advance covered by rests. Note events identical to a tie
        // (the remainder rest starts before the NoteOff — inherent overlap).
        let notes_only = |song: &Note| {
            stream_events(song)
                .into_iter()
                .filter(|(_, _, shown, _)| {
                    shown.starts_with("NoteOn(") || shown.starts_with("NoteOff")
                })
                .collect::<Vec<_>>()
        };
        let tied = ser!([c4q, q, rq]);
        let sustained = ser!([legato!(ser!(c4q, q), 32), rq]);
        assert_eq!(notes_only(&tied), notes_only(&sustained));
        // Advance accounting: full-duration advance behaves like a plain note.
        let full = ser!([legato!(c4e, 16), rqd]);
        let plain = ser!([c4e, rqd]);
        assert_eq!(stream_events(&full), stream_events(&plain));
        // Par-head tempo bakes into atoms but never reaches event ambient;
        // the sustain must read tempo exactly like its inner note does
        // (note events identical; the remainder rest starts early by design).
        let par_tied = par!([param!(tempo = 116.955444), ser!([c4q, q, rq])]);
        let par_sus = par!([
            param!(tempo = 116.955444),
            ser!([legato!(ser!(c4q, q), 32), rq]),
        ]);
        assert_eq!(notes_only(&par_tied), notes_only(&par_sus));
        // Grid exactness: material after the sustain lands on the advance,
        // read with the song's tempo (not the 60bpm default). 64 ticks =
        // 1026ms at this tempo; ambient-only tempo would give 1513ms.
        let grid = ser!([
            param!(tempo = 116.955444),
            legato!(ser!(c4q, q), 32),
            rq,
            c4e,
        ]);
        let onsets: Vec<u32> = stream_events(&grid)
            .into_iter()
            .filter(|(_, _, shown, _)| shown.starts_with("NoteOn("))
            .map(|(time, _, _, _)| time)
            .collect();
        assert_eq!(onsets, vec![0, 1026]);
    }

    #[test]
    fn envelopes_evaluate() {
        let probe = env!(q0, e1, h0.5, q0);
        let mmlx_core::Note::Envelope(envelope) = probe else {
            panic!("env! must produce an Envelope note")
        };
        // Mid-attack value strictly between endpoints (linear).
        let val = envelope.evaluate(0.05, 120.0, 4, Some(2.0));
        assert!(val > 0.0 && val < 1.0, "interpolated, got {val}");
        for variant in [
            linenv!(q0, q1),
            cosenv!(q0, q1),
            expenv!(q0.01, q1),
            cubenv!(q0, q1),
        ] {
            match variant {
                mmlx_core::Note::Envelope(inner) => assert_eq!(inner.points.len(), 2),
                other => panic!("interpolation variant must produce Envelope, got {other:?}"),
            }
        }
    }
}
