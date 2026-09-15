//! Server smoke test: drive Player directly (evcxr init takes a while).
//!
//! Run with: `cargo run -p mmlx-server --example server_smoke`
//! (Cannot run under `cargo test`: evcxr re-executes the current binary as
//! its worker, which breaks under the libtest harness.)

use mmlx_server::Player;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn drain(rx: &mpsc::Receiver<String>) -> Vec<String> {
    let mut messages = Vec::new();
    while let Ok(message) = rx.try_recv() {
        messages.push(message);
    }
    messages
}

fn main() {
    let (out_tx, out_rx) = mpsc::channel();
    let mut player = Player::new(
        mmlx_audio::new_queue(),
        mmlx_audio::new_synth_time(),
        out_tx,
    )
    .expect("player");
    let _ = drain(&out_rx);

    player.handle_line("loop off");
    player.handle_line("play ser!([c4q, d4q, e4q, g4h])");
    let messages = drain(&out_rx);
    assert!(
        messages
            .iter()
            .any(|message| message.starts_with("ok playing")),
        "playing ack: {messages:?}"
    );
    println!("PLAY OK: {messages:?}");

    // Default tempo is 60bpm, so q=1s and h=2s: 5s total. The first NoteOn
    // sits at t=0, so a pos report arrives on the first polls.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_pos = false;
    while Instant::now() < deadline {
        player.poll();
        for message in drain(&out_rx) {
            println!("EVENT: {message}");
            if message.starts_with("pos ") {
                saw_pos = true;
            }
        }
        if saw_pos {
            break;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    assert!(saw_pos, "expected pos events while playing");

    player.handle_line("stop");
    let messages = drain(&out_rx);
    assert!(
        messages.iter().any(|message| message == "ok stopped"),
        "stop ack: {messages:?}"
    );

    player.handle_line("bogus");
    let messages = drain(&out_rx);
    assert!(
        messages.iter().any(|message| message.starts_with("err")),
        "unknown command errors: {messages:?}"
    );

    player.handle_line("roll ser!([c4q, e4q])");
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut rows = 0;
    let mut ended = false;
    while Instant::now() < deadline {
        for message in drain(&out_rx) {
            if message.starts_with("rollrow ") {
                rows += 1;
            }
            if message == "rollend" {
                ended = true;
            }
        }
        if ended {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(ended && rows == 2, "roll yields 2 rows + end");

    println!("SERVER SMOKE PASS");
}
