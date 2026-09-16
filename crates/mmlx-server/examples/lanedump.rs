//! Dump per-lane NoteOn sequences for the highlight harness.
//! Usage: lanedump <song-fn> [body] -> JSON lines: {"inst","ch","midis":[...]}.
//! Plain `Note` songs ignore the body index; generator songs pull that
//! body (0 = intro, 1+ = loop cycles).
use mmlx_core::{MusicalEventType, ParamValue};
use std::collections::HashMap;

fn channel_of(event: &mmlx_core::TimedMusicalEvent) -> Option<(String, i64)> {
    let key = match event.instrument_name.as_str() {
        "ym" => "ym_channel",
        "psg" => "sn_channel",
        _ => return None,
    };
    if let MusicalEventType::NoteOn { parameters, .. } = &event.event {
        if let Some(ParamValue::Number(channel)) = parameters.get(key) {
            return Some((event.instrument_name.clone(), channel.round() as i64));
        }
    }
    None
}

fn main() {
    let name = std::env::args().nth(1).expect("song fn name");
    let body: usize = std::env::args()
        .nth(2)
        .map(|arg| arg.parse().expect("body index"))
        .unwrap_or(0);
    let events: Vec<_> = if let Some(song) = mmlx_songs::song_by_name(&name) {
        assert!(body == 0, "plain songs have one body");
        song().event_stream(0.0).collect()
    } else if let Some(stream) = mmlx_songs::song_stream_by_name(&name) {
        stream()
            .nth(body)
            .expect("stream body")
            .event_stream(0.0)
            .collect()
    } else {
        panic!("unknown song {name}");
    };
    let mut lanes: HashMap<(String, i64), Vec<u8>> = HashMap::new();
    let mut times: HashMap<(String, i64), Vec<f32>> = HashMap::new();
    let mut order: Vec<(String, i64)> = Vec::new();
    for event in &events {
        if let MusicalEventType::NoteOn { pitch_midi, .. } = &event.event {
            if let Some(key) = channel_of(event) {
                lanes
                    .entry(key.clone())
                    .or_insert_with(|| {
                        order.push(key.clone());
                        Vec::new()
                    })
                    .push(*pitch_midi);
                times.entry(key).or_default().push(event.time_seconds);
            }
        }
    }
    for key in &order {
        let midis = &lanes[key];
        let seq: Vec<String> = midis.iter().map(|midi| midi.to_string()).collect();
        // Parallel arrays for the column audit: per-NoteOn times in order.
        let times: Vec<String> = times[key].iter().map(|t| format!("{t:.6}")).collect();
        println!(
            "{} {} {} {} {}",
            key.0,
            key.1,
            midis.len(),
            seq.join(" "),
            times.join(" ")
        );
    }
}
