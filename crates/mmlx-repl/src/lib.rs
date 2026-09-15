//! evcxr-backed evaluator: try `Note`, then `Iterator<Item=Note>`, then raw Rust.
//!
//! Notes that evaluate successfully are queued for playback via
//! `mmlx_audio::queue_note`. Draining an *infinite* iterator never returns
//! (same as CP437) — bound it with `.take(n)` in the REPL.

use anyhow::{Context, Result};
use crossbeam_channel::Receiver;
use evcxr::{CommandContext, EvalOutputs};
use mmlx_audio::{queue_note, EventQueue, SynthTime};
use mmlx_core::Note;
use std::thread;

/// Absolute path to the workspace's `mmlx-core` for the evcxr `:dep` line.
const CORE_DEP_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../mmlx-core");

fn unique_suffix() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..16)
        .map(|_| CHARS[rand::Rng::random_range(&mut rng, 0..CHARS.len())] as char)
        .collect()
}

/// Wrap an expression so a `Note` result comes back as JSON.
fn wrap_note(expr: &str) -> String {
    format!(
        r#"{{
            let __note_{s}: Note = {{ {e} }};
            serde_json::to_string(&__note_{s}).unwrap()
        }}"#,
        s = unique_suffix(),
        e = expr
    )
}

/// Wrap an expression so an `Iterator<Item=Note>` result binds to a variable
/// whose name comes back as JSON.
fn wrap_iter(expr: &str) -> String {
    format!(
        r#"{{
            let result = {{ {e} }};
            let mut __iter_{s}: Box<dyn Iterator<Item = Note> + Send + Sync + 'static> =
                Box::new(result);
            serde_json::to_string("__iter_{s}").unwrap()
        }}"#,
        s = unique_suffix(),
        e = expr
    )
}

pub enum EvalOutcome {
    /// A single Note, already queued.
    Note(Note),
    /// An iterator drained into the queue (count of notes queued).
    IterDone(usize),
    /// Raw Rust output text.
    Text(String),
    /// Empty / unit output.
    Empty,
}

pub struct ReplEnv {
    ctx: CommandContext,
    queue: EventQueue,
    time: SynthTime,
}

impl ReplEnv {
    pub fn new(queue: EventQueue, time: SynthTime) -> Result<Self> {
        evcxr::runtime_hook();
        let (mut ctx, outputs) = CommandContext::new().context("evcxr context")?;
        Self::spawn_output_forwarders(outputs.stdout, outputs.stderr);

        ctx.execute(":load_config --quiet")
            .context("evcxr :load_config")?;
        ctx.execute(&format!(
            ":dep mmlx-core = {{ path = \"{}\" }}",
            CORE_DEP_PATH
        ))
        .context("evcxr :dep mmlx-core")?;
        ctx.execute(":dep serde_json = \"1.0\"")
            .context("evcxr :dep serde_json")?;
        ctx.execute(
            r#"
            #[macro_use]
            extern crate mmlx_core;
            use mmlx_core::prelude::*;
            use mmlx_core::env;
            ()
        "#,
        )
        .context("evcxr prelude")?;
        Ok(ReplEnv { ctx, queue, time })
    }

    fn spawn_output_forwarders(stdout: Receiver<String>, stderr: Receiver<String>) {
        thread::spawn(move || {
            while let Ok(line) = stdout.recv() {
                println!("{line}");
            }
        });
        thread::spawn(move || {
            while let Ok(evt) = stderr.recv() {
                eprintln!("{evt}");
            }
        });
    }

    fn execute(&mut self, command: &str) -> Result<EvalOutputs> {
        self.ctx
            .execute(command)
            .with_context(|| format!("evcxr failed for: {command}"))
    }

    fn plain_text(output: &EvalOutputs) -> Option<&str> {
        output
            .content_by_mime_type
            .get("text/plain")
            .map(String::as_str)
    }

    pub fn evaluate_line(&mut self, line: &str) -> Result<EvalOutcome> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(EvalOutcome::Empty);
        }

        // Attempt 1: a Note expression.
        if let Ok(output) = self.execute(&wrap_note(trimmed)) {
            if let Some(text) = Self::plain_text(&output) {
                if let Ok(inner) = serde_json::from_str::<String>(text) {
                    if let Ok(note) = serde_json::from_str::<Note>(&inner) {
                        queue_note(note.clone(), &self.queue, &self.time);
                        return Ok(EvalOutcome::Note(note));
                    }
                }
            }
        }

        // Attempt 2: an Iterator<Item=Note> expression.
        if let Ok(output) = self.execute(&wrap_iter(trimmed)) {
            if let Some(text) = Self::plain_text(&output) {
                if let Ok(var) = serde_json::from_str::<String>(text) {
                    if var.starts_with("__iter_") {
                        let mut count = 0;
                        loop {
                            let cmd = format!(
                                "{{ let v = {var}.next(); serde_json::to_string(&v).unwrap() }}"
                            );
                            let next = self.execute(&cmd)?;
                            let Some(js_outer) = Self::plain_text(&next) else {
                                break;
                            };
                            let js_inner: String = serde_json::from_str(js_outer)?;
                            let opt: Option<Note> = serde_json::from_str(&js_inner)?;
                            match opt {
                                Some(note) => {
                                    queue_note(note, &self.queue, &self.time);
                                    count += 1;
                                }
                                None => break,
                            }
                        }
                        return Ok(EvalOutcome::IterDone(count));
                    }
                }
            }
        }

        // Attempt 3: raw Rust.
        let raw = self.execute(trimmed)?;
        if let Some(text) = Self::plain_text(&raw) {
            let text = text.trim();
            if !text.is_empty() && text != "()" {
                return Ok(EvalOutcome::Text(text.to_string()));
            }
        }
        Ok(EvalOutcome::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrappers_shape_code() {
        let wrapped = wrap_note("ser!([c4q])");
        assert!(wrapped.contains("ser!([c4q])"));
        assert!(wrapped.contains(": Note"));
        assert!(wrapped.contains("serde_json::to_string"));
        let iter_wrapped = wrap_iter("std::iter::once(c4q)");
        assert!(iter_wrapped.contains("Iterator<Item = Note>"));
        assert!(iter_wrapped.contains("__iter_"));
    }

    #[test]
    fn suffixes_unique() {
        assert_ne!(unique_suffix(), unique_suffix());
        assert_eq!(unique_suffix().len(), 16);
    }

    // Full evcxr round-trip lives in examples/repl_smoke.rs: it cannot run
    // under `cargo test` because evcxr re-executes the current binary as its
    // worker subprocess, which breaks under the libtest harness.
}
