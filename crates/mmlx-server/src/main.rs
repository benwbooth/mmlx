//! `mmlx-server` binary: stdin/stdout playback engine (see lib docs).

use anyhow::Result;
use mmlx_server::Player;
use std::io::BufRead;
use std::sync::mpsc;
use std::time::Duration;

fn main() -> Result<()> {
    env_logger::init();

    let queue;
    let time;
    let instruments;
    let mute;
    #[cfg(feature = "audio")]
    let _stream;
    #[cfg(feature = "audio")]
    {
        let (queue_, time_, instruments_, mute_, stream) = mmlx_audio::backend::setup_audio()?;
        mmlx_audio::backend::play_stream(&stream)?;
        _stream = stream;
        queue = queue_;
        time = time_;
        instruments = instruments_;
        mute = Some(mute_);
    }
    #[cfg(not(feature = "audio"))]
    {
        queue = mmlx_audio::new_queue();
        time = mmlx_audio::new_synth_time();
        instruments = mmlx_audio::InstrumentsMap::default();
        mute = None;
    }

    let (line_tx, line_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    let (out_tx, out_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for message in out_rx {
            println!("{message}");
        }
    });

    let mut player = Player::new(queue, time, instruments, mute, out_tx)?;
    loop {
        while let Ok(line) = line_rx.try_recv() {
            player.handle_line(&line);
        }
        player.poll();
        std::thread::sleep(Duration::from_nanos(1_000_000_000 / 60));
    }
}
