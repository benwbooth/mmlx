//! `mmlx-server`: line-protocol playback engine for editors.
//!
//! stdin commands (one per line):
//!   load <path>      execute a Rust song file's items in the eval context
//!   play <expr>      evaluate `<expr>` as a Note and play it from the start
//!   stop             pause, keeping position
//!   reset            stop and rewind to the start
//!   loop on|off      toggle looping at the end of the stream
//!   preview <expr>   evaluate `<expr>` (queued to audio when present)
//! stdout events:
//!   ok <msg> | err <msg>
//!   pos <tick> <ordinal>   `<ordinal>` = NoteOns with time <= now (1-based
//!     count of the currently sounding note; -1 when stopped at the start)
//!   ended                emitted once when a non-looping play finishes
//!
//! The clock is virtual (wall-clock rate), so position reporting works
//! headless. With the `audio` feature the evaluated notes also sound via
//! the cpal backend.

use anyhow::Result;
use mmlx_core::{MusicalEventType, TimedMusicalEvent};
use mmlx_repl::{EvalOutcome, ReplEnv};
use std::sync::mpsc::Sender;
use std::time::Instant;

pub struct Player {
    repl: ReplEnv,
    out: Sender<String>,
    events: Vec<TimedMusicalEvent>,
    noteon_times: Vec<f32>,
    playing: bool,
    looping: bool,
    offset: f32,
    started: Option<Instant>,
    tick: u64,
    last_ordinal: i64,
}

impl Player {
    pub fn new(
        queue: mmlx_audio::EventQueue,
        time: mmlx_audio::SynthTime,
        out: Sender<String>,
    ) -> Result<Self> {
        Ok(Player {
            repl: ReplEnv::new(queue, time)?,
            out,
            events: Vec::new(),
            noteon_times: Vec::new(),
            playing: false,
            looping: true,
            offset: 0.0,
            started: None,
            tick: 0,
            last_ordinal: -1,
        })
    }

    fn say(&self, message: String) {
        let _ = self.out.send(message);
    }

    fn now(&self) -> f32 {
        self.offset
            + self
                .started
                .map(|started| started.elapsed().as_secs_f32())
                .unwrap_or(0.0)
    }

    fn end(&self) -> f32 {
        self.events
            .iter()
            .map(|event| event.time_seconds + event.real_duration)
            .fold(0.0f32, f32::max)
    }

    fn ordinal_at(&self, now: f32) -> i64 {
        self.noteon_times
            .iter()
            .filter(|time| **time <= now)
            .count() as i64
    }

    pub fn handle_line(&mut self, line: &str) {
        let mut parts = line.splitn(2, char::is_whitespace);
        let command = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("").trim();
        match command {
            "load" => match std::fs::read_to_string(rest) {
                Ok(code) => match self.repl.execute_code(&code) {
                    Ok(_) => self.say("ok loaded".to_string()),
                    Err(err) => self.say(format!("err {err:?}")),
                },
                Err(err) => self.say(format!("err cannot read {rest}: {err}")),
            },
            "play" => match self.repl.evaluate_line(rest) {
                Ok(EvalOutcome::Note(note)) => {
                    self.events = note.event_stream(0.0).collect();
                    self.noteon_times = self
                        .events
                        .iter()
                        .filter(|event| matches!(event.event, MusicalEventType::NoteOn { .. }))
                        .map(|event| event.time_seconds)
                        .collect();
                    self.offset = 0.0;
                    self.started = Some(Instant::now());
                    self.playing = true;
                    self.last_ordinal = -1;
                    self.say(format!("ok playing {}", self.events.len()));
                }
                Ok(EvalOutcome::IterDone(_)) => self.say(
                    "err infinite iterators are not playable, bound with .take(n)".to_string(),
                ),
                Ok(_) => self.say("err expression is not a Note".to_string()),
                Err(err) => self.say(format!("err {err:?}")),
            },
            "preview" => match self.repl.evaluate_line(rest) {
                Ok(EvalOutcome::Note(note)) => {
                    let count = note.event_stream(0.0).count();
                    self.say(format!("ok preview {count}"));
                }
                Ok(_) => self.say("err expression is not a Note".to_string()),
                Err(err) => self.say(format!("err {err:?}")),
            },
            "stop" => {
                if self.playing {
                    self.offset = self.now();
                }
                self.playing = false;
                self.started = None;
                self.say("ok stopped".to_string());
            }
            "reset" => {
                self.playing = false;
                self.started = None;
                self.offset = 0.0;
                self.last_ordinal = -1;
                self.say("ok reset".to_string());
                self.say("pos 0 -1".to_string());
            }
            "loop" => {
                self.looping = rest != "off";
                self.say(format!(
                    "ok loop {}",
                    if self.looping { "on" } else { "off" }
                ));
            }
            "" => {}
            other => self.say(format!("err unknown command {other}")),
        }
    }

    /// Advance the virtual clock; call at ~60Hz. Emits `pos`/`ended`.
    pub fn poll(&mut self) {
        if !self.playing {
            return;
        }
        let now = self.now();
        let end = self.end();
        if !self.events.is_empty() && now >= end {
            if self.looping {
                self.offset -= end;
                self.started = Some(Instant::now());
                self.last_ordinal = -1;
                return;
            }
            self.playing = false;
            self.started = None;
            self.offset = 0.0;
            self.say("ended".to_string());
            return;
        }
        let ordinal = self.ordinal_at(now);
        if ordinal != self.last_ordinal {
            self.last_ordinal = ordinal;
            self.say(format!("pos {} {ordinal}", self.tick));
        }
        self.tick += 1;
    }
}
