//! `mmlx-server`: line-protocol playback engine for editors.
//!
//! stdin commands (one per line):
//!   load <path>      execute a Rust song file's items in the eval context
//!   play <expr>      bare `name()` of a compiled-in song plays instantly
//!     (no JIT); any other expression evaluates it as a Note and plays it
//!     from the start
//!   reload [expr]    re-evaluate (default: current) and keep the playhead
//!   stop             pause, keeping position
//!   reset            stop and rewind to the start
//!   loop on|off      toggle looping at the end of the stream
//!   preview <expr>   evaluate `<expr>` (queued to audio when present)
//!   roll <expr>      evaluate `<expr>` and emit its piano roll as
//!     `rollrow <start> <midi> <dur> <instrument>` lines plus `rollend`
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
    repl: Option<ReplEnv>,
    queue: mmlx_audio::EventQueue,
    time: mmlx_audio::SynthTime,
    out: Sender<String>,
    events: Vec<TimedMusicalEvent>,
    noteon_times: Vec<f32>,
    playing: bool,
    looping: bool,
    offset: f32,
    started: Option<Instant>,
    tick: u64,
    last_ordinal: i64,
    last_expr: Option<String>,
}

impl Player {
    pub fn new(
        queue: mmlx_audio::EventQueue,
        time: mmlx_audio::SynthTime,
        out: Sender<String>,
    ) -> Result<Self> {
        // The evcxr context is built lazily (see `ensure_repl`): a cold
        // build takes a minute or more, and compiled-in songs don't need
        // it at all, so starting playback must not wait for it.
        Ok(Player {
            repl: None,
            queue,
            time,
            out,
            events: Vec::new(),
            noteon_times: Vec::new(),
            playing: false,
            looping: true,
            offset: 0.0,
            started: None,
            tick: 0,
            last_ordinal: -1,
            last_expr: None,
        })
    }

    /// Build the evcxr JIT context on first use.
    fn ensure_repl(&mut self) -> Result<&mut ReplEnv> {
        if self.repl.is_none() {
            self.say("ok compiling jit context…".to_string());
            let queue = self.queue.clone();
            let time = self.time.clone();
            let repl = ReplEnv::new(queue, time)?;
            self.repl = Some(repl);
        }
        Ok(self.repl.as_mut().expect("repl built above"))
    }

    /// Start playback of an evaluated `Note`, shared by `play` (both
    /// compiled-in and JIT paths) and `reload`.
    fn start_playing(&mut self, note: mmlx_core::Note, label: String) {
        // Queue for the audio backend (drains to cpal when live; the JIT
        // path queues inside `evaluate_line`, so do it here for parity —
        // otherwise compiled-in songs play silently).
        mmlx_audio::queue_note(note.clone(), &self.queue, &self.time);
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
        self.last_expr = Some(label);
        self.say(format!("ok playing {}", self.events.len()));
    }

    /// `name()` with nothing else: a bare song call. Plays the compiled-in
    /// version instantly when the registry has it (the lotw `rom` path).
    /// Returns true when handled (hit or miss handled by caller fallback).
    fn bare_name(expr: &str) -> Option<&str> {
        let expr = expr.trim();
        let call = expr.strip_suffix("()")?;
        if call.is_empty()
            || !call
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
        {
            return None;
        }
        // Only plain `name()` / `path::name()` calls qualify; anything with
        // arguments or operators goes to the JIT.
        if call.contains('(') || call.contains([' ', '\t', '+', '-', '*', '/', '|', ';']) {
            return None;
        }
        Some(call)
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
                Ok(code) => match self.ensure_repl() {
                    Ok(repl) => match repl.execute_code(&code) {
                        Ok(_) => self.say("ok loaded".to_string()),
                        Err(err) => self.say(format!("err {err:?}")),
                    },
                    Err(err) => self.say(format!("err {err:?}")),
                },
                Err(err) => self.say(format!("err cannot read {rest}: {err}")),
            },
            "play" => {
                // lotw `rom` path: a bare `name()` call for a compiled-in
                // song starts instantly with no JIT involved.
                if let Some(name) = Self::bare_name(rest) {
                    if let Some(song) = mmlx_songs::song_by_name(name) {
                        self.start_playing(song(), rest.to_string());
                        return;
                    }
                }
                let outcome = self.ensure_repl().map(|repl| repl.evaluate_line(rest));
                match outcome {
                    Ok(Ok(EvalOutcome::Note(note))) => {
                        self.start_playing(note, rest.to_string());
                    }
                    Ok(Ok(EvalOutcome::IterDone(_))) => self.say(
                        "err infinite iterators are not playable, bound with .take(n)".to_string(),
                    ),
                    Ok(Ok(_)) => self.say("err expression is not a Note".to_string()),
                    Ok(Err(err)) | Err(err) => self.say(format!("err {err:?}")),
                }
            }
            "preview" => {
                let outcome = self.ensure_repl().map(|repl| repl.evaluate_line(rest));
                match outcome {
                    Ok(Ok(EvalOutcome::Note(note))) => {
                        let count = note.event_stream(0.0).count();
                        self.say(format!("ok preview {count}"));
                    }
                    Ok(Ok(_)) => self.say("err expression is not a Note".to_string()),
                    Ok(Err(err)) | Err(err) => self.say(format!("err {err:?}")),
                }
            }
            // Re-evaluate the current expression (or `reload <expr>`) and keep
            // the playhead: live-editing without losing position.
            "reload" => {
                let expr = if rest.is_empty() {
                    match self.last_expr.clone() {
                        Some(expr) => expr,
                        None => {
                            self.say("err nothing to reload".to_string());
                            return;
                        }
                    }
                } else {
                    rest.to_string()
                };
                match self.ensure_repl().map(|repl| repl.evaluate_line(&expr)) {
                    Ok(Ok(EvalOutcome::Note(note))) => {
                        let position = self.now();
                        self.events = note.event_stream(0.0).collect();
                        self.noteon_times = self
                            .events
                            .iter()
                            .filter(|event| matches!(event.event, MusicalEventType::NoteOn { .. }))
                            .map(|event| event.time_seconds)
                            .collect();
                        self.offset = position.min(self.end());
                        if self.playing {
                            self.started = Some(Instant::now());
                        }
                        self.last_ordinal = -1;
                        self.last_expr = Some(expr);
                        self.say(format!("ok reloaded {}", self.events.len()));
                    }
                    Ok(Ok(_)) => self.say("err expression is not a Note".to_string()),
                    Ok(Err(err)) | Err(err) => self.say(format!("err {err:?}")),
                }
            }
            "roll" => {
                let outcome = self.ensure_repl().map(|repl| repl.evaluate_line(rest));
                match outcome {
                    Ok(Ok(EvalOutcome::Note(note))) => {
                        let events: Vec<_> = note.event_stream(0.0).collect();
                        for row in mmlx_algo::piano_roll(&events).lines() {
                            self.say(format!("rollrow {row}"));
                        }
                        self.say("rollend".to_string());
                    }
                    Ok(Ok(_)) => self.say("err expression is not a Note".to_string()),
                    Ok(Err(err)) | Err(err) => self.say(format!("err {err:?}")),
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_player() -> (Player, std::sync::mpsc::Receiver<String>) {
        let (out_tx, out_rx) = std::sync::mpsc::channel();
        let player = Player::new(
            mmlx_audio::new_queue(),
            mmlx_audio::new_synth_time(),
            out_tx,
        )
        .expect("player");
        (player, out_rx)
    }

    #[test]
    fn builtin_play_streams_highlight_and_queues_audio() {
        // No evcxr involved: must answer instantly and feed BOTH the
        // highlight stream and the audio backend queue.
        let (mut player, out_rx) = test_player();
        player.handle_line("play all_features()");
        assert!(!player.events.is_empty(), "highlight events collected");
        assert!(
            !player.queue.lock().unwrap().is_empty(),
            "audio queue must receive events or compiled-in songs play silently"
        );
        assert!(out_rx.try_recv().unwrap().starts_with("ok playing"));
    }

    #[test]
    fn bare_name_detects_builtin_calls() {
        assert_eq!(Player::bare_name("all_features()"), Some("all_features"));
        assert_eq!(
            Player::bare_name("  loop_alisia_stage1()  "),
            Some("loop_alisia_stage1")
        );
        // Anything else (arguments, operators, macros, empty) goes JIT.
        for expr in ["", "foo(1)", "ser!([c4q])", "a + b", "foo ()", "play x"] {
            assert_eq!(Player::bare_name(expr), None, "{expr:?}");
        }
        // Unknown names miss the registry (covered in mmlx-songs tests).
        assert!(mmlx_songs::song_by_name("no_such_song").is_none());
    }
}
