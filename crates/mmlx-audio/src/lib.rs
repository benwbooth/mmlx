// mmlx-audio: playback queue + optional live audio backend.
//
// The queue types and `queue_note` are always available (headless-safe).
// The cpal backend (`setup_audio`) needs system audio headers and lives
// behind the `audio` feature.

use mmlx_core::Instrument;
use mmlx_core::{note_stream_to_event_stream, Note, TimedMusicalEvent};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

pub type EventQueue = Arc<Mutex<VecDeque<TimedMusicalEvent>>>;
pub type SynthTime = Arc<Mutex<f32>>;
pub type InstrumentsMap = Arc<Mutex<HashMap<String, Arc<Mutex<dyn Instrument>>>>>;

pub fn new_queue() -> EventQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub fn new_synth_time() -> SynthTime {
    Arc::new(Mutex::new(0.0f32))
}

/// Converts a Note into TimedMusicalEvents and queues them for playback.
pub fn queue_note(note: Note, queue: &EventQueue, time: &SynthTime) {
    let start_time = { *time.lock().unwrap() };
    let mut event_iter = note_stream_to_event_stream(Box::new(std::iter::once(note)), start_time);

    let mut events_to_queue = Vec::new();
    while let Some(evt) = event_iter.next() {
        log::debug!("Queueing Event: time={:.4}s", evt.time_seconds);
        events_to_queue.push(evt);
    }

    if !events_to_queue.is_empty() {
        let mut q_guard = queue.lock().unwrap();
        for evt in events_to_queue {
            q_guard.push_back(evt);
        }
    }
    log::info!("Queued note starting at {:.4}s", start_time);
}

#[cfg(feature = "audio")]
pub mod backend;
