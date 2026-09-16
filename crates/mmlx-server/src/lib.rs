//! `mmlx-server`: line-protocol playback engine for editors.
//!
//! stdin commands (one per line):
//!   load <path>      execute a Rust song file's items in the eval context
//!   play <expr>      bare `name()` of a compiled-in song plays instantly
//!     (no JIT); generator songs page one body per loop wrap. Any other
//!     expression evaluates it as a Note and plays it from the start
//!   reload [expr]    re-evaluate (default: current) and keep the playhead
//!   stop             pause, keeping position (queued audio is dropped and
//!     voices silenced too, so sound really pauses)
//!   resume           continue paused playback without re-evaluating
//!   reset            stop, rewind, and silence everything
//!   loop on|off      toggle looping at the end of the stream
//!   preview <expr>   evaluate `<expr>` (queued to audio when present)
//!   roll <expr>      evaluate `<expr>` and emit its piano roll as
//!     `rollrow <start> <midi> <dur> <instrument>` lines plus `rollend`
//! stdout events:
//!   ok <msg> | err <msg>
//!   pos <tick> <ordinal> <inst> <ch> <lane-ordinal> <cycle>  global NoteOn
//!     ordinal plus the sounding note's lane identity for exact source
//!     highlighting (`- - -` when the lane is unknown; `-1` ordinal when
//!     idle). `cycle` counts generator-song bodies (0 = intro) so the
//!     editor maps ordinals into the right section.
//!   ended                emitted once when a non-looping play finishes
//!
//! The clock is virtual (wall-clock rate), so position reporting works
//! headless. With the `audio` feature the evaluated notes also sound via
//! the cpal backend.

use anyhow::Result;
use mmlx_core::{MusicalEventType, TimedMusicalEvent};
use mmlx_repl::{EvalOutcome, ReplEnv};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::time::Instant;

pub struct Player {
    repl: Option<ReplEnv>,
    queue: mmlx_audio::EventQueue,
    time: mmlx_audio::SynthTime,
    instruments: mmlx_audio::InstrumentsMap,
    out: Sender<String>,
    events: Vec<TimedMusicalEvent>,
    noteon_times: Vec<f32>,
    /// Per-NoteOn lane identity, parallel to `noteon_times`: (instrument,
    /// channel, lane ordinal). Chip lanes pin `ym_channel`/`sn_channel`
    /// params, so the editor can highlight the exact source lane.
    noteon_lanes: Vec<(String, i64, i64)>,
    /// Active generator song, if playing one: each loop wrap pulls the
    /// next body (intro once, then loop forever) with bounded memory.
    /// `cycle` counts pulled bodies (0 = intro) for highlight mapping.
    /// `None` replays the single collected body as before.
    stream: Option<mmlx_core::NoteIterator>,
    cycle: u64,
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
        instruments: mmlx_audio::InstrumentsMap,
        out: Sender<String>,
    ) -> Result<Self> {
        // The evcxr context is built lazily (see `ensure_repl`): a cold
        // build takes a minute or more, and compiled-in songs don't need
        // it at all, so starting playback must not wait for it.
        Ok(Player {
            repl: None,
            queue,
            time,
            instruments,
            out,
            events: Vec::new(),
            noteon_times: Vec::new(),
            noteon_lanes: Vec::new(),
            stream: None,
            cycle: 0,
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

    /// Start playback of an evaluated `Note` from `from` seconds in.
    /// Owns audio queueing outright (the JIT evaluates with queueing
    /// disabled): drops anything queued, silences voices, collects the
    /// highlight stream, and queues events rebased to the audio clock.
    fn start_playing(&mut self, note: mmlx_core::Note, label: String, from: f32) {
        self.stream = None;
        self.load_body(note, from);
        self.last_expr = Some(label);
        self.say(format!("ok playing {}", self.events.len()));
    }

    /// Start playback of a generator song from the top. Pulls one body
    /// per loop wrap (see `poll`), so infinite performances page with
    /// bounded memory; highlight ordinals restart per body.
    fn start_streaming(&mut self, stream: mmlx_core::NoteIterator, label: String) {
        self.stream = Some(stream);
        self.cycle = 0;
        if self.pull_stream() {
            self.last_expr = Some(label);
            self.say(format!("ok playing {}", self.events.len()));
        } else {
            self.stream = None;
            self.say("err stream yielded no bodies".to_string());
        }
    }

    /// Pull the next generator body into the highlight stream and audio
    /// queue from its top. False when a finite stream is exhausted.
    fn pull_stream(&mut self) -> bool {
        let next = match self.stream.as_mut().and_then(|s| s.next()) {
            Some(note) => note,
            None => return false,
        };
        self.load_body(next, 0.0);
        true
    }

    /// Shared body setup: silence, collect events + highlight, requeue.
    fn load_body(&mut self, note: mmlx_core::Note, from: f32) {
        self.silence();
        self.events = note.event_stream(0.0).collect();
        self.collect_noteons();
        self.requeue_from(from);
        self.offset = from;
        self.started = Some(Instant::now());
        self.playing = true;
        self.last_ordinal = -1;
    }

    /// Rebuild the highlight stream + per-NoteOn lane identities from the
    /// current events. Events arrive in time order, so lane ordinals
    /// count up per lane in sounding order.
    fn collect_noteons(&mut self) {
        self.noteon_times.clear();
        self.noteon_lanes.clear();
        let mut lane_counts: HashMap<(String, i64), i64> = HashMap::new();
        for event in &self.events {
            if !matches!(event.event, MusicalEventType::NoteOn { .. }) {
                continue;
            }
            self.noteon_times.push(event.time_seconds);
            match Self::lane_key(event) {
                Some((instrument, channel)) => {
                    let count = lane_counts
                        .entry((instrument.clone(), channel))
                        .or_insert(0);
                    *count += 1;
                    self.noteon_lanes.push((instrument, channel, *count));
                }
                None => self
                    .noteon_lanes
                    .push((event.instrument_name.clone(), -1, -1)),
            }
        }
    }

    /// Lane identity for highlight mapping: chip lanes pin
    /// `ym_channel`/`sn_channel` params on every note. Anything else
    /// (or unpinned lanes) maps globally like before.
    fn lane_key(event: &TimedMusicalEvent) -> Option<(String, i64)> {
        let key = match event.instrument_name.as_str() {
            "ym" => "ym_channel",
            "psg" => "sn_channel",
            _ => return None,
        };
        if let MusicalEventType::NoteOn { parameters, .. } = &event.event {
            if let Some(mmlx_core::ParamValue::Number(channel)) = parameters.get(key) {
                return Some((event.instrument_name.clone(), channel.round() as i64));
            }
        }
        None
    }

    /// Drop queued audio and silence sounding voices immediately.
    fn silence(&mut self) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.clear();
        }
        #[cfg(feature = "audio")]
        mmlx_audio::backend::all_notes_off(&self.instruments);
        // Headless voices hold no sound; nothing to do.
    }

    /// Queue collected events at/after `from`, rebased so `from` sounds
    /// now on the backend's audio clock. Diagnostic comments are skipped
    /// (already logged on first play).
    fn requeue_from(&mut self, from: f32) {
        let audio_now = *self.time.lock().unwrap();
        let mut rows: Vec<TimedMusicalEvent> = self
            .events
            .iter()
            .filter(|event| {
                event.time_seconds >= from && !matches!(event.event, MusicalEventType::Comment(_))
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| {
            a.time_seconds
                .partial_cmp(&b.time_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Ok(mut queue) = self.queue.lock() {
            for mut event in rows {
                event.time_seconds = event.time_seconds - from + audio_now;
                queue.push_back(event);
            }
        }
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
                // song starts instantly with no JIT involved. Generator
                // songs (intro once, loop forever) page per cycle.
                if let Some(name) = Self::bare_name(rest) {
                    if let Some(song) = mmlx_songs::song_by_name(name) {
                        self.start_playing(song(), rest.to_string(), 0.0);
                        return;
                    }
                    if let Some(stream) = mmlx_songs::song_stream_by_name(name) {
                        self.start_streaming(stream(), rest.to_string());
                        return;
                    }
                }
                let outcome = self
                    .ensure_repl()
                    .map(|repl| repl.evaluate_line_no_queue(rest));
                match outcome {
                    Ok(Ok(EvalOutcome::Note(note))) => {
                        self.start_playing(note, rest.to_string(), 0.0);
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
            // the playhead: live-editing without losing position. Generator
            // songs restart from the top (infinite streams have no end to
            // keep position against).
            "reload" => {
                if self.stream.is_some() {
                    if let Some(name) = self
                        .last_expr
                        .as_ref()
                        .and_then(|expr| Self::bare_name(expr))
                        .and_then(mmlx_songs::song_stream_by_name)
                    {
                        let label = self.last_expr.clone().unwrap();
                        self.start_streaming(name(), label);
                        return;
                    }
                    self.stream = None;
                }
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
                match self
                    .ensure_repl()
                    .map(|repl| repl.evaluate_line_no_queue(&expr))
                {
                    Ok(Ok(EvalOutcome::Note(note))) => {
                        let position = self.now();
                        self.silence();
                        self.events = note.event_stream(0.0).collect();
                        self.collect_noteons();
                        self.requeue_from(position);
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
                let outcome = self
                    .ensure_repl()
                    .map(|repl| repl.evaluate_line_no_queue(rest));
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
                // Pause, keeping position: freeze the highlight clock and
                // drop queued audio so sound stops too (`resume` re-queues).
                if self.playing {
                    self.offset = self.now();
                }
                self.playing = false;
                self.started = None;
                self.silence();
                self.say("ok stopped".to_string());
            }
            "reset" => {
                self.playing = false;
                self.started = None;
                self.offset = 0.0;
                self.last_ordinal = -1;
                // Parked streams rewind: rebuild the generator and collect
                // the first body (no queueing while stopped) so resume
                // restarts the performance from the top, not mid-loop.
                if self.stream.is_some() {
                    if let Some(name) = self
                        .last_expr
                        .as_ref()
                        .and_then(|expr| Self::bare_name(expr))
                        .and_then(mmlx_songs::song_stream_by_name)
                    {
                        self.stream = Some(name());
                        if let Some(note) = self.stream.as_mut().and_then(|s| s.next()) {
                            self.events = note.event_stream(0.0).collect();
                            self.collect_noteons();
                        } else {
                            self.stream = None;
                        }
                        self.cycle = 0;
                    } else {
                        self.stream = None;
                    }
                }
                self.silence();
                self.say("ok reset".to_string());
                self.say("pos 0 -1 - - -".to_string());
            }
            "resume" => {
                // Continue paused playback without re-evaluating: re-queue
                // from the kept offset. Instant (no JIT).
                if self.playing {
                    self.say("err already playing".to_string());
                } else if self.events.is_empty() {
                    self.say("err nothing to resume".to_string());
                } else {
                    self.silence();
                    self.requeue_from(self.offset);
                    self.started = Some(Instant::now());
                    self.playing = true;
                    self.last_ordinal = -1;
                    self.say("ok resumed".to_string());
                }
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
            if self.stream.is_some() {
                // Generator song: page the next body (intro once, then
                // loop forever) unless looping is off, and count the cycle
                // for highlight mapping. Exhausted finite streams end.
                if self.looping && self.pull_stream() {
                    self.cycle += 1;
                    return;
                }
                self.stream = None;
            } else if self.looping {
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
            // Lane-aware highlight: `pos <tick> <ordinal> <inst> <ch>
            // <lane-ordinal>`. Chip lanes pin ym_channel/sn_channel, so
            // the editor maps to the exact source lane; anything else
            // carries `- - -` and maps by global ordinal as before.
            let (inst, channel, lane_ordinal) = if ordinal >= 1 {
                match self.noteon_lanes.get(ordinal as usize - 1) {
                    Some((instrument, channel, lane_ordinal)) => (
                        instrument.clone(),
                        channel.to_string(),
                        lane_ordinal.to_string(),
                    ),
                    None => ("-".to_string(), "-".to_string(), "-".to_string()),
                }
            } else {
                ("-".to_string(), "-".to_string(), "-".to_string())
            };
            self.say(format!(
                "pos {} {ordinal} {inst} {channel} {lane_ordinal} {}",
                self.tick, self.cycle
            ));
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
            mmlx_audio::InstrumentsMap::default(),
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

    #[test]
    fn stream_play_pages_bodies() {
        // Generator songs start instantly (no JIT) and page one body per
        // pull; finite streams report exhaustion instead of hanging.
        let (mut player, out_rx) = test_player();
        player.handle_line("play alisia_stage1()");
        assert!(player.stream.is_some(), "stream parked");
        assert!(!player.events.is_empty(), "intro body collected");
        assert!(out_rx.try_recv().unwrap().starts_with("ok playing"));
        // Second body (first loop) re-collects highlight + queue.
        assert!(player.pull_stream(), "infinite stream keeps yielding");
        assert!(!player.events.is_empty());
        // A finite stream exhausts cleanly.
        player.stream = Some(Box::new(vec![mmlx_songs::all_features()].into_iter()));
        assert!(player.pull_stream(), "one body");
        assert!(!player.pull_stream(), "then exhausted");
    }
}
