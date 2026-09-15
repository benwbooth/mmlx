//! Dump per-lane NoteOn sequences for the highlight harness.
//! Usage: lanedump <song-fn> -> JSON lines: {"inst","ch","midis":[...]}.
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
    let note = mmlx_songs::song_by_name(&name).expect("known song")();
    let events: Vec<_> = note.event_stream(0.0).collect();
    let mut lanes: HashMap<(String, i64), Vec<u8>> = HashMap::new();
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
            }
        }
    }
    for key in &order {
        let midis = &lanes[key];
        let seq: Vec<String> = midis.iter().map(|midi| midi.to_string()).collect();
        println!("{} {} {} {}", key.0, key.1, midis.len(), seq.join(" "));
    }
}
