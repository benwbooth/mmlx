//! Tracker backends: fixtures (always), engine renders (skip if lib missing).

use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
use mmlx_track::{events_to_mod, midi_to_period, mod_fixture, nsf_fixture, TrackError};
use std::collections::HashMap;

fn peak(buffer: &[[f32; 2]]) -> f32 {
    buffer
        .iter()
        .map(|frame| frame[0].abs().max(frame[1].abs()))
        .fold(0.0f32, f32::max)
}

fn skipped(what: &str) {
    eprintln!("SKIP ({what}): engine library not present; run under nix develop");
}

#[test]
fn nsf_fixture_layout() {
    let nsf = nsf_fixture();
    assert_eq!(&nsf[0..5], b"NESM\x1A");
    assert_eq!(u16::from_le_bytes([nsf[10], nsf[11]]), 0x8000); // init
    assert_eq!(u16::from_le_bytes([nsf[12], nsf[13]]), 0x8015); // play
    assert_eq!(nsf[128], 0xA9); // LDA
    assert_eq!(nsf[128 + 20], 0x60); // init RTS
    assert_eq!(nsf[128 + 21], 0x60); // play RTS
}

#[test]
fn mod_fixture_layout() {
    let module = mod_fixture();
    assert_eq!(&module[1080..1084], b"M.K.");
    assert_eq!(module[950], 1);
    // Row 0, channel 0: sample 1, period 856, no effect.
    assert_eq!(&module[1084..1088], &[0x03, 0x58, 0x10, 0x00]);
    assert_eq!(module.len(), 1084 + 1024 + 64);
}

#[test]
fn gme_renders_nsf_square() {
    let rendered = match mmlx_track::render_gme("nsf", &nsf_fixture(), 0, 44100, 1.0) {
        Err(TrackError::MissingLibrary(lib)) => return skipped(&lib),
        Err(TrackError::MissingSymbol(symbol)) => return skipped(&symbol),
        Err(err) => panic!("gme failed: {err}"),
        Ok(buffer) => buffer,
    };
    assert_eq!(rendered.len(), 44100);
    assert!(peak(&rendered) > 0.05, "NSF square audible");
}

#[test]
fn openmpt_renders_mod_sine() {
    let module = mod_fixture();
    let duration = match mmlx_track::mod_duration(&module) {
        Err(TrackError::MissingLibrary(lib)) => return skipped(&lib),
        Err(TrackError::MissingSymbol(symbol)) => return skipped(&symbol),
        Err(err) => panic!("duration failed: {err}"),
        Ok(duration) => duration,
    };
    assert!(duration > 5.0, "one pattern lasts ~7.7s");
    let rendered = match mmlx_track::render_mod(&module, 44100, 2.0) {
        Err(TrackError::MissingLibrary(lib)) => return skipped(&lib),
        Err(TrackError::MissingSymbol(symbol)) => return skipped(&symbol),
        Err(err) => panic!("openmpt failed: {err}"),
        Ok(buffer) => buffer,
    };
    assert!(peak(&rendered) > 0.02, "MOD sine audible");
}

fn atom(midi: u8, start: f32, duration: f32) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: start,
        real_duration: duration,
        event: MusicalEventType::NoteOn {
            note_id: midi as u64,
            pitch_midi: midi,
            velocity: 0.8,
            parameters: HashMap::new(),
            attack_envelope: None,
            sustain_envelope: None,
            release_envelope: None,
            other_envelopes: Vec::new(),
        },
        instrument_name: "test".to_string(),
    }
}

#[test]
fn events_synthesize_periods() {
    assert_eq!(midi_to_period(36), 856);
    assert_eq!(midi_to_period(60), 214);
    let events = vec![atom(60, 0.0, 0.5), atom(64, 0.5, 0.5), atom(67, 1.0, 1.0)];
    let module = events_to_mod(&events);
    assert_eq!(&module[1080..1084], b"M.K.");
    // C4 period 214 = 0xD6 lands in row 0 of channel 0.
    let row0 = &module[1084..1088];
    assert_eq!(row0[0] & 0x0F, 0x00, "period high nibble of 0xD6");
    assert_eq!(row0[1], 0xD6);
    assert_eq!(row0[2], 0x1C, "sample 1 + volume effect");
    let rendered = match mmlx_track::render_events_mod(&events, 44100, 3.0) {
        Err(TrackError::MissingLibrary(lib)) => return skipped(&lib),
        Err(TrackError::MissingSymbol(symbol)) => return skipped(&symbol),
        Err(err) => panic!("event render failed: {err}"),
        Ok(buffer) => buffer,
    };
    assert!(peak(&rendered) > 0.01, "synthesized module audible");
}

#[test]
fn fluidsynth_plays_bank() {
    let bank = mmlx_sf2::minimal_sf2(44100, 440.0, 69);
    let path = std::env::temp_dir().join("mmlx-test-bank.sf2");
    std::fs::write(&path, bank).expect("write bank");
    let mut voice = match mmlx_track::FluidVoice::open(&path.to_string_lossy(), 44100) {
        Err(TrackError::MissingLibrary(lib)) => return skipped(&lib),
        Err(TrackError::MissingSymbol(symbol)) => return skipped(&symbol),
        Err(err) => panic!("fluid open failed: {err}"),
        Ok(voice) => voice,
    };
    voice.process_event(&atom(69, 0.0, 0.5), 44100);
    let buffer = voice.generate_samples(22050, 44100);
    assert!(peak(&buffer) > 0.01, "fluid sine audible");
    voice.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.5,
            real_duration: 0.0,
            event: MusicalEventType::NoteOff { note_id: 69 },
            instrument_name: "test".to_string(),
        },
        44100,
    );
    let _ = voice.generate_samples(44100, 44100);
    assert!(voice.is_idle());
}

fn mt32_rom_dir() -> Option<String> {
    for candidate in [
        std::env::var("MMLX_MT32_ROMS").ok(),
        Some("/home/ben/.local/share/kog/roms/mt32/v1.07".to_string()),
        Some("/home/ben/.config/dosbox/mt32-roms".to_string()),
    ]
    .into_iter()
    .flatten()
    {
        if std::path::Path::new(&candidate).is_dir() {
            return Some(candidate);
        }
    }
    None
}

#[test]
fn munt_plays_with_roms() {
    let Some(dir) = mt32_rom_dir() else {
        return skipped("no MT-32 ROM directory found");
    };
    let mut voice = match mmlx_track::Mt32Voice::open(&dir, 44100) {
        Err(TrackError::MissingLibrary(lib)) => return skipped(&lib),
        Err(TrackError::MissingSymbol(symbol)) => return skipped(&symbol),
        Err(err) => panic!("munt open failed: {err}"),
        Ok(voice) => voice,
    };
    voice.process_event(&atom(69, 0.0, 0.5), 44100);
    let buffer = voice.generate_samples(22050, 44100);
    assert!(peak(&buffer) > 0.005, "MT-32 piano audible");
    voice.process_event(
        &TimedMusicalEvent {
            time_seconds: 0.5,
            real_duration: 0.0,
            event: MusicalEventType::NoteOff { note_id: 69 },
            instrument_name: "test".to_string(),
        },
        44100,
    );
    let _ = voice.generate_samples(44100 * 2, 44100);
    assert!(voice.is_idle());
}
