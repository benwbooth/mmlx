//! Queue test (headless-safe: no audio device needed).

#[macro_use]
extern crate mmlx_core;

use mmlx_audio::{new_queue, new_synth_time, queue_note};
use mmlx_core::prelude::*;

#[test]
fn queued_note_produces_ordered_events() {
    let queue = new_queue();
    let time = new_synth_time();
    queue_note(ser!([c4q, d4q]), &queue, &time);
    let guard = queue.lock().unwrap();
    assert!(!guard.is_empty());
    assert!(guard
        .iter()
        .any(|ev| matches!(ev.event, mmlx_core::MusicalEventType::NoteOn { .. })));
}
