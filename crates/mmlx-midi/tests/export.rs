//! SMF round-trip: export an event stream, parse it back with midly.

#[macro_use]
extern crate mmlx_core;

use mmlx_core::prelude::*;

#[test]
fn export_parses_back() {
    let song = ser!([param!(tempo = 120), c4q, e4q, g4q]);
    let events: Vec<_> = song.event_stream(0.0).collect();
    let expected_ons = events
        .iter()
        .filter(|event| matches!(event.event, mmlx_core::MusicalEventType::NoteOn { .. }))
        .count();

    let bytes = mmlx_midi::events_to_smf(&events, 480, 120.0);
    assert!(!bytes.is_empty());

    let smf = midly::Smf::parse(&bytes).expect("parse exported SMF");
    assert_eq!(smf.tracks.len(), 1);
    let mut ons = 0;
    let mut offs = 0;
    let mut first_pitch = None;
    for event in &smf.tracks[0] {
        match event.kind {
            midly::TrackEventKind::Midi { message, .. } => match message {
                midly::MidiMessage::NoteOn { key, .. } => {
                    ons += 1;
                    first_pitch.get_or_insert(u8::from(key));
                }
                midly::MidiMessage::NoteOff { .. } => offs += 1,
                _ => {}
            },
            _ => {}
        }
    }
    assert_eq!(ons, expected_ons);
    assert_eq!(offs, ons);
    assert_eq!(first_pitch, Some(60));
}
