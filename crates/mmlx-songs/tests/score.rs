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

/// Inline param diffs immediately before legato! must reach the inner
/// note (regression: op4_tl/tied arriving stale cost us compare #403).
#[test]
fn inline_param_diffs_reach_legato_inner() {
    use std::collections::HashMap;
    let voice: Note = param!(instrument = "ym", ym_channel = 0, op4_tl = 9);
    let ch = ser!(track!(
        voice c4q rx param!(op4_tl = 17, tied = 0) legato!(d4q, 4)
    ));
    let ons: Vec<HashMap<String, mmlx_core::ParamValue>> = ch
        .event_stream(0.0)
        .filter_map(|ev| match ev.event {
            mmlx_core::MusicalEventType::NoteOn { parameters, .. } => Some(parameters),
            _ => None,
        })
        .collect();
    assert_eq!(ons.len(), 2);
    let get = |params: &HashMap<String, mmlx_core::ParamValue>, key: &str| match params.get(key) {
        Some(mmlx_core::ParamValue::Number(value)) => *value,
        other => panic!("{key} missing: {other:?}"),
    };
    assert_eq!(get(&ons[0], "op4_tl"), 9.0);
    assert_eq!(get(&ons[1], "op4_tl"), 17.0, "inline diff applies");
    assert_eq!(get(&ons[1], "tied"), 0.0, "inline tied applies");
}

/// Repeated toggles of the same keys (voice TL wobble runs like ch2's
/// 9/17/21 tremolo) must each take effect.
#[test]
fn repeated_param_toggles_each_apply() {
    use std::collections::HashMap;
    let voice: Note = param!(instrument = "ym", ym_channel = 0, op4_tl = 9);
    let ch = ser!(track!(
        voice c4q rx param!(op4_tl = 9, tied = 1) d4q rx
        param!(op4_tl = 17, tied = 0) legato!(e4q, 4)
    ));
    let ons: Vec<HashMap<String, mmlx_core::ParamValue>> = ch
        .event_stream(0.0)
        .filter_map(|ev| match ev.event {
            mmlx_core::MusicalEventType::NoteOn { parameters, .. } => Some(parameters),
            _ => None,
        })
        .collect();
    assert_eq!(ons.len(), 3);
    let get = |params: &HashMap<String, mmlx_core::ParamValue>, key: &str| match params.get(key) {
        Some(mmlx_core::ParamValue::Number(value)) => *value,
        other => panic!("{key} missing: {other:?}"),
    };
    assert_eq!(get(&ons[2], "op4_tl"), 17.0, "second toggle applies");
    assert_eq!(get(&ons[2], "tied"), 0.0, "second tied applies");
}

/// Single-pair `param!(k=v)` followed by a multi-pair group touching the
/// same key: the later write must win in the following `legato!` inner
/// note. Regression for the bake-context bug where multi-pair groups
/// (nested `Serial`s) never updated `current_attrs`, so the legato inner
/// baked the stale first value (9) instead of the live one (17).
#[test]
fn multi_pair_group_after_single_updates_bake() {
    use std::collections::HashMap;
    let voice: Note = param!(instrument = "ym", ym_channel = 2, op4_tl = 21);
    let lane = ser!(track!(
        voice param!(op4_tl = 9) d3i param!(op4_tl = 17, tied = 0) legato!(a3i, 4)
    ));
    let ons: Vec<(u8, HashMap<String, mmlx_core::ParamValue>)> = lane
        .event_stream(0.0)
        .filter_map(|ev| match ev.event {
            mmlx_core::MusicalEventType::NoteOn {
                pitch_midi,
                parameters,
                ..
            } => Some((pitch_midi, parameters)),
            _ => None,
        })
        .collect();
    assert_eq!(ons.len(), 2);
    let get = |params: &HashMap<String, mmlx_core::ParamValue>, key: &str| match params.get(key) {
        Some(mmlx_core::ParamValue::Number(value)) => *value,
        other => panic!("{key} missing: {other:?}"),
    };
    assert_eq!(
        get(&ons[1].1, "op4_tl"),
        17.0,
        "later multi-pair write wins"
    );
    assert_eq!(get(&ons[1].1, "tied"), 0.0, "later multi-pair tied wins");
}

/// Verbatim loop-bar-2 harmony2 lane (compare #403): the trailing legato
/// a3i must carry the immediately preceding op4_tl=17/tied=0 diff, not the
/// stale op4_tl=9 from earlier in the bar.
#[test]
fn verbatim_ch2_bar2_legato_diff_applies() {
    let voice_harmony2_7: Note = param!(
        instrument = "ym",
        op1_ar = 25,
        op1_dr = 28,
        op1_dt = 6,
        op1_mult = 6,
        op1_rr = 1,
        op1_sl = 3,
        op1_sr = 5,
        op1_ssg = 0,
        op1_tl = 20,
        op2_ar = 24,
        op2_dr = 1,
        op2_dt = 5,
        op2_mult = 4,
        op2_rr = 4,
        op2_sl = 15,
        op2_sr = 1,
        op2_ssg = 0,
        op2_tl = 20,
        op3_ar = 25,
        op3_dr = 27,
        op3_dt = 1,
        op3_mult = 1,
        op3_rr = 4,
        op3_sl = 2,
        op3_sr = 1,
        op3_ssg = 0,
        op3_tl = 14,
        op4_ar = 20,
        op4_dr = 28,
        op4_dt = 3,
        op4_mult = 2,
        op4_rr = 6,
        op4_sl = 3,
        op4_sr = 5,
        op4_ssg = 0,
        op4_tl = 21,
        ym_algo = 3,
        ym_channel = 2,
        ym_feedback = 3
    );
    let lane = ser!(
        track!(voice_harmony2_7                  rxd                 param!(op4_tl=9, tied=1) c3e x                                                                                  rx                 d3i o                                           ro param!(tied=0) d3i                                   rx       param!(op4_tl=17) d3i                                  rx param!(op4_tl=21) d3i rq                                                                                                                            param!(op4_tl=9)             d3i                                        rxd                          param!(tied=1)                       g3i                                                         rx                                       a3i                                                                                                   rx      param!(op4_tl=17, tied=0) legato!(a3i, 4))
    );
    let ons: Vec<(u8, f32, f32)> = lane
        .event_stream(0.0)
        .filter_map(|ev| match ev.event {
            mmlx_core::MusicalEventType::NoteOn {
                pitch_midi,
                parameters,
                ..
            } => {
                let num = |key: &str| match parameters.get(key) {
                    Some(mmlx_core::ParamValue::Number(value)) => *value,
                    _ => f32::NAN,
                };
                Some((pitch_midi, num("op4_tl"), num("tied")))
            }
            _ => None,
        })
        .collect();
    let last = ons.last().expect("lane sounds notes");
    assert_eq!(last.0, 57);
    assert_eq!(last.1, 17.0, "verbatim lane applies final diff");
    assert_eq!(last.2, 0.0, "verbatim lane applies final tied");
}
