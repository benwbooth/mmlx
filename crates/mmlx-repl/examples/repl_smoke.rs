//! Smoke test: real evcxr context evaluates DSL and queues notes.
//!
//! Run with: `cargo run -p mmlx-repl --example repl_smoke`
//! (Cannot run under `cargo test`: evcxr re-executes the current binary as
//! its worker, which breaks under the libtest harness.)

use mmlx_repl::{EvalOutcome, ReplEnv};

fn main() {
    let queue = mmlx_audio::new_queue();
    let time = mmlx_audio::new_synth_time();
    let mut env = ReplEnv::new(queue.clone(), time).expect("evcxr context");

    match env.evaluate_line("ser!([c4q, d4q])").expect("eval") {
        EvalOutcome::Note(note) => println!("NOTE OK: {note}"),
        other => panic!("expected Note, got {}", describe(&other)),
    }
    let queued = queue.lock().unwrap().len();
    assert!(queued > 0, "queue must hold events");
    println!("QUEUE OK: {queued} events");

    match env.evaluate_line("1 + 1").expect("eval") {
        EvalOutcome::Text(text) => println!("RAW OK: {text}"),
        other => panic!("expected Text, got {}", describe(&other)),
    }
    // Regression: item-only `execute_code` (song files) must compile.
    // evcxr emits a dummy `(mut x …)` shim for expression-less executes,
    // whose parameter trips E0530 against the prelude's `x` tie const;
    // `execute_code` appends `()` to suppress it.
    env.execute_code("pub fn smoke_song() -> Note { ser!([c4q]) }")
        .expect("item load");
    match env.evaluate_line("smoke_song()").expect("eval") {
        EvalOutcome::Note(_) => println!("ITEM LOAD OK"),
        other => panic!("expected Note, got {}", describe(&other)),
    }
    println!("SMOKE PASS");
}

fn describe(outcome: &EvalOutcome) -> &'static str {
    match outcome {
        EvalOutcome::Note(_) => "Note",
        EvalOutcome::IterDone(_) => "IterDone",
        EvalOutcome::Text(_) => "Text",
        EvalOutcome::Empty => "Empty",
    }
}
