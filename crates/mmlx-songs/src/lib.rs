//! mmlx-songs: example compositions exercising the full v0.1 DSL.
//!
//! `all_features()` is the conformance song: every currently implemented
//! language feature appears at least once. Keep it in sync with MMLX_SPEC.md.

#[macro_use]
extern crate mmlx_core;

use mmlx_core::env;
use mmlx_core::prelude::*;

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
