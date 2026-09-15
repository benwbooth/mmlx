//! Synthetic VGM: tracker precision, emitter compression and exactness.

use mmlx_vgm::*;

// Build a tiny VGM: setup ch0 (algo 0, sine-ish ops), two A440 notes.
fn synth_vgm() -> Vec<u8> {
    let mut header = vec![0u8; 0x40];
    header[0..4].copy_from_slice(b"Vgm ");
    header[8..12].copy_from_slice(&0x150u32.to_le_bytes());
    header[0x18..0x1C].copy_from_slice(&70560u32.to_le_bytes()); // total
    header[0x2C..0x30].copy_from_slice(&7_670_453u32.to_le_bytes());
    header[0x34..0x38].copy_from_slice(&0x0Cu32.to_le_bytes());
    let mut body = vec![];
    fn ym(body: &mut Vec<u8>, addr: u8, val: u8) {
        body.push(0x52);
        body.push(addr);
        body.push(val);
    }
    ym(&mut body, 0xB0, 0x00);
    ym(&mut body, 0xB4, 0xC0);
    for row in [0x30u8, 0x34, 0x38, 0x3C] {
        ym(&mut body, row, 0x01);
        ym(&mut body, row + 0x10, 0x10);
        ym(&mut body, row + 0x20, 0x1F);
        ym(&mut body, row + 0x30, 0x10);
        ym(&mut body, row + 0x40, 0x00);
        ym(&mut body, row + 0x50, 0x0F);
        ym(&mut body, row + 0x60, 0x00);
    }
    // Note 1: A440 (block 4, fnum 1082 = 0x43A).
    ym(&mut body, 0xA4, 0x24);
    ym(&mut body, 0xA0, 0x3A);
    ym(&mut body, 0x28, 0xF0);
    body.extend([0x61, 0xE0, 0x5B]); // 23520 samples = 32 ticks
                                     // Note 2: retriggered immediately (legato run with note 1).
    ym(&mut body, 0x28, 0x00);
    ym(&mut body, 0xA4, 0x24);
    ym(&mut body, 0xA0, 0x3A);
    ym(&mut body, 0x28, 0xF0);
    body.extend([0x61, 0xE0, 0x5B]); // 23520 samples = 32 ticks
    ym(&mut body, 0x28, 0x00);
    body.push(0x66);
    header.extend(body);
    header
}

#[test]
fn tracker_extracts_exact_notes() {
    let data = synth_vgm();
    let header = parse_header(&data).expect("header");
    assert_eq!(header.ym2612_clock, 7_670_453);
    assert_eq!(header.data_start, 0x40);
    let commands = parse_commands(&data, &header).expect("commands");
    let notes = track_fm(&commands, header.ym2612_clock, header.total_samples as u64);
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].midi, 69);
    assert_eq!(notes[0].start, 0);
    assert_eq!(notes[0].duration, 23520);
    assert_eq!(notes[0].voice, 0);
    assert_eq!(notes[1].start, 23520);
    // Raw program captured.
    let tl = notes[0]
        .params
        .iter()
        .find(|(key, _)| key == "op1_tl")
        .unwrap()
        .1;
    assert_eq!(tl, 16.0);
    let algo = notes[0]
        .params
        .iter()
        .find(|(key, _)| key == "ym_algo")
        .unwrap()
        .1;
    assert_eq!(algo, 0.0);
    assert!(!notes[0].approx);
}

#[test]
fn emitter_is_exact_and_compressed() {
    // 735-sample ticks (60 Hz driver): 44100 samples = 60 ticks.
    assert_eq!(ticks_to_durations(60), vec!["qddd"]);
    assert_eq!(ticks_to_durations(128), vec!["w"]);
    // Every tick count 1..=256 round-trips exactly.
    for ticks in 1..=256u64 {
        let sum: u64 = ticks_to_durations(ticks)
            .iter()
            .map(|suffix| {
                [
                    ("w", 128),
                    ("h", 64),
                    ("q", 32),
                    ("e", 16),
                    ("i", 8),
                    ("t", 4),
                    ("x", 2),
                    ("o", 1),
                ]
                .iter()
                .find(|(name, _)| suffix.starts_with(name))
                .map(|(_, base)| {
                    let mult = if suffix.ends_with("ddd") {
                        15
                    } else if suffix.ends_with("dd") {
                        14
                    } else if suffix.ends_with('d') {
                        12
                    } else {
                        8
                    };
                    base * mult / 8
                })
                .unwrap()
            })
            .sum();
        assert_eq!(sum, ticks, "ticks {ticks}");
    }
    assert_eq!(midi_name(60), ("c", 4));
    assert_eq!(midi_name(61), ("cs", 4));

    let data = synth_vgm();
    let header = parse_header(&data).expect("header");
    let commands = parse_commands(&data, &header).expect("commands");
    let notes = track_fm(&commands, header.ym2612_clock, header.total_samples as u64);
    let src = emit_song("synth", &[("ym".to_string(), vec![], notes)], 735, 112.5);
    // Readable consts, compression markers, loop fns, tempo.
    assert!(src.contains("a4q"), "note literal:\n{src}");
    assert!(src.contains("repeat!(1)"), "run compression:\n{src}");
    assert!(src.contains("pub fn synth()"), "song fn");
    assert!(src.contains("pub fn loop_synth()"), "loop fn");
    assert!(src.contains("tempo=112.5"), "tempo header");
    assert!(src.contains("ym_algo"), "raw program");
}
