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
