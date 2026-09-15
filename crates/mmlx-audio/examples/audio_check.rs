//! Live backend check: play a short phrase through the cpal output.
//! Run under nix develop on a machine with audio:
//! `nix develop -c cargo run -p mmlx-audio --features audio --example audio_check`

#[macro_use]
extern crate mmlx_core;

use mmlx_core::prelude::*;

fn main() {
    let (queue, time, _instruments, stream) = mmlx_audio::backend::setup_audio().unwrap();
    cpal::traits::StreamTrait::play(&stream).unwrap();
    println!("stream playing");
    mmlx_audio::queue_note(
        ser!([param!(tempo = 120, patch = "triangle"), c4q, e4q, g4q, c5h]),
        &queue,
        &time,
    );
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!(
            "synth_time={:.2} queued={}",
            *time.lock().unwrap(),
            queue.lock().unwrap().len()
        );
    }
    println!("AUDIO CHECK DONE");
}
