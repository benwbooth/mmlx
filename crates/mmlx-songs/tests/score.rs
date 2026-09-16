//! Score-structure macros: `bar!`/`track!` build bit-identical `Note`
//! trees to plain `par!`/`ser!` (outlining is layout-only for codegen).

#[macro_use]
extern crate mmlx_core;

use mmlx_core::prelude::*;

#[test]
fn bar_track_match_par_ser() {
    let voice_lead: Note = param!(instrument = "ym", ym_channel = 0);
    let voice_bass: Note = param!(instrument = "ym", ym_channel = 1);
    let plain = ser!(par!(ser!(voice_lead c4q d4q), ser!(voice_bass c2h)));
    let scored = ser!(bar!(track!(voice_lead c4q d4q), track!(voice_bass c2h),));
    // Identical trees imply identical streams (event IDs aside: those come
    // from a global counter at stream time, in tree order).
    assert_eq!(plain, scored);
    assert_eq!(
        plain.clone().event_stream(0.0).count(),
        scored.clone().event_stream(0.0).count()
    );
}

#[test]
fn bar_without_voices_needs_no_args() {
    let plain = ser!(par!(ser!(rw)));
    let scored = ser!(bar!(track!(rw)));
    assert_eq!(plain, scored);
}

fn first_note_on(note: &Note) -> Option<(f32, u8)> {
    // NOTE: never bind `e`/`c1`/etc. here — the note-const prelude
    // squats those names (a bare ident pattern resolves to the const!).
    note.clone()
        .event_stream(0.0)
        .find_map(|ev| match ev.event {
            mmlx_core::MusicalEventType::NoteOn { pitch_midi, .. } => {
                Some((ev.time_seconds, pitch_midi))
            }
            _ => None,
        })
}

/// A voice-headed channel sounds its first note at stream position 0:
/// program setup (voice lets, velocity params) takes no musical time.
#[test]
fn voice_head_takes_no_time() {
    let voice: Note = param!(instrument = "ym", ym_channel = 1);
    let plain = ser!(par!(ser!(voice e0o f0o)));
    let scored = ser!(bar!(track!(voice e0o f0o)));
    assert_eq!(first_note_on(&plain), Some((0.0, 16)));
    assert_eq!(first_note_on(&scored), Some((0.0, 16)));
}

fn all_note_ons(note: &Note) -> Vec<(f32, u8)> {
    note.clone()
        .event_stream(0.0)
        .filter_map(|ev| match ev.event {
            mmlx_core::MusicalEventType::NoteOn { pitch_midi, .. } => {
                Some((ev.time_seconds, pitch_midi))
            }
            _ => None,
        })
        .collect()
}

/// Exact head of alisia intro bar 1 arp: voice + velocity + e3tdd must
/// sound E3 at t=0, then A3 after e3tdd+rxd (7+3 ticks at 116.955444).
#[test]
fn arp_head_timing_is_exact() {
    let voice_arp: Note = param!(instrument = "psg", sn_channel = 0);
    let ch = ser!(track!(voice_arp param!(velocity = 93.13) e3tdd rxd a3i));
    let ons = all_note_ons(&ch);
    assert_eq!(ons.len(), 2);
    assert_eq!(ons[0].1, 52);
    assert_eq!(ons[1].1, 57);
    assert!(
        ons[0].0.abs() < 1e-6,
        "e3tdd sounds at stream start, got {:?}",
        ons[0]
    );
}
