//! `mmlx-repl` interactive loop: type DSL expressions, hear them.
//!
//! Without the `audio` feature, evaluated notes are queued in memory and
//! their event counts are printed (no sound). With `--features audio` the
//! cpal backend drains the queue live.

use anyhow::Result;
use log::error;
use mmlx_repl::{EvalOutcome, ReplEnv};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};
use std::borrow::Cow;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::as_24_bit_terminal_escaped;

struct SyntectHelper {
    ps: SyntaxSet,
    theme: Theme,
    history: DefaultHistory,
}

impl SyntectHelper {
    fn new(theme_name: &str) -> Self {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let theme = ts.themes.get(theme_name).expect("theme").clone();
        SyntectHelper {
            ps,
            theme,
            history: DefaultHistory::new(),
        }
    }
}

impl Completer for SyntectHelper {
    type Candidate = String;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<String>), ReadlineError> {
        let start = line[..pos]
            .rfind(|c: char| !c.is_alphanumeric())
            .map_or(0, |i| i + 1);
        let prefix = &line[start..pos];
        let suggestions = self
            .history
            .iter()
            .rev()
            .filter(|entry| entry.starts_with(prefix))
            .cloned()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        Ok((start, suggestions))
    }
}

impl Hinter for SyntectHelper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }
        let start = line
            .rfind(|c: char| !c.is_alphanumeric())
            .map_or(0, |i| i + 1);
        let prefix = &line[start..pos];
        self.history
            .iter()
            .rev()
            .find(|entry| entry.starts_with(prefix) && entry.len() > prefix.len())
            .map(|entry| entry[prefix.len()..].to_string())
    }
}

impl Validator for SyntectHelper {}

impl Highlighter for SyntectHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let syntax = self
            .ps
            .find_syntax_by_extension("rs")
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
        let mut hl = HighlightLines::new(syntax, &self.theme);
        match hl.highlight_line(line, &self.ps) {
            Ok(ranges) => Cow::Owned(as_24_bit_terminal_escaped(&ranges[..], false)),
            Err(_) => Cow::Borrowed(line),
        }
    }
    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true
    }
}

impl Helper for SyntectHelper {}

fn main() -> Result<()> {
    env_logger::init();

    if std::env::var("EVCXR_TMPDIR").is_err() {
        let dir = std::env::temp_dir().join("mmlx-evcxr");
        let _ = std::fs::create_dir_all(&dir);
        unsafe {
            std::env::set_var("EVCXR_TMPDIR", &dir);
        }
    }

    let queue;
    let time;
    #[cfg(feature = "audio")]
    let _stream;
    #[cfg(feature = "audio")]
    {
        let (queue_, time_, _instruments, _mute, stream) = mmlx_audio::backend::setup_audio()?;
        cpal::traits::StreamTrait::play(&stream)?;
        log::info!("audio backend live");
        _stream = stream;
        queue = queue_;
        time = time_;
    }
    #[cfg(not(feature = "audio"))]
    {
        queue = mmlx_audio::new_queue();
        time = mmlx_audio::new_synth_time();
    }

    let mut repl = ReplEnv::new(queue, time)?;
    let mut rl = Editor::<SyntectHelper, DefaultHistory>::new()?;
    rl.set_helper(Some(SyntectHelper::new("base16-ocean.dark")));

    loop {
        match rl.readline("mmlx>> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                match repl.evaluate_line(line.trim()) {
                    Ok(EvalOutcome::Note(note)) => println!("queued {note}"),
                    Ok(EvalOutcome::IterDone(n)) => println!("queued {n} notes"),
                    Ok(EvalOutcome::Text(text)) => println!("{text}"),
                    Ok(EvalOutcome::Empty) => {}
                    Err(err) => error!("{err:?}"),
                }
            }
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                error!("{err:?}");
                break;
            }
        }
    }
    Ok(())
}
