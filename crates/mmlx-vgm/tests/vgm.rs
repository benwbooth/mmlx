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
    let src = emit_song(
        "synth",
        &[("ym".to_string(), "lead".to_string(), vec![], notes)],
        735,
        112.5,
        128,
    );
    // Readable consts, compression markers, loop fns, tempo.
    assert!(src.contains("a4q"), "note literal:\n{src}");
    assert!(src.contains("repeat!(1)"), "run compression:\n{src}");
    assert!(src.contains("pub fn synth()"), "song fn");
    assert!(
        !src.contains("pub fn loop_synth()"),
        "single song fn, no loop twin:\n{src}"
    );
    assert!(src.contains("gen!("), "generator body:\n{src}");
    assert!(src.contains("yield_!"), "yields:\n{src}");
    assert!(src.contains("NoteIterator"), "streaming return:\n{src}");
    // Our fork's `gen!` boxes the stream itself: no adapters in songs.
    assert!(!src.contains("Box::new"), "no boxing:\n{src}");
    assert!(!src.contains("into_iter()"), "no adapter:\n{src}");
    // Flow shape: voices, then music straight through the yields —
    // no hoisted section lets, no wrappers.
    assert!(src.contains("yield_!(ser!("), "inline yield mixes:\n{src}");
    assert!(!src.contains("intro_body"), "no hoisted lets:\n{src}");
    assert!(!src.contains("Arc::new"), "no Arc handoff:\n{src}");
    assert!(
        !src.contains("::std::iter::once("),
        "no once/chain streaming:\n{src}"
    );
    assert!(src.contains("tempo=112.5"), "tempo header");
    assert!(src.contains("ym_algo"), "raw program");
    // Terse form: every voice program binds `let` and splices bare —
    // bar channels never carry inline `param!(instrument=...)` groups.
    assert!(!src.contains("fn voice_lead"), "no voice fns:\n{src}");
    assert!(!src.contains(".clone()"), "bare splices:\n{src}");
    assert!(src.contains("ym_algo"), "raw program");
    for chunk in src.split("bar!( // bar").skip(1) {
        assert!(
            !chunk.contains("param!(instrument"),
            "no inline programs in bars:\n{chunk}"
        );
    }
    assert!(!src.contains("ser!(['"), "terse lanes:\n{src}");
    assert!(!src.contains("par!(["), "terse mix:\n{src}");
    // Bar-major score: outer ser of per-bar `bar!`s with channel `track!`s.
    assert!(src.contains("bar!( // bar 1"), "bar macros:\n{src}");
    assert!(src.contains("track!("), "track lanes:\n{src}");
    assert!(
        src.contains("let voice_lead: Note"),
        "voice program variables:\n{src}"
    );
}

#[test]
fn bar_columns_align_by_time() {
    // Two lanes with offset rhythms: hits at the same tick share a
    // column; earlier ticks sit strictly left (whitespace-only layout).
    fn note(start: u64, dur: u64, midi: u8) -> TrackNote {
        TrackNote {
            start,
            duration: dur,
            voice: 0,
            midi,
            velocity: 1.0,
            params: Vec::new(),
            approx: false,
        }
    }
    let lead = vec![note(0, 32, 60), note(32, 32, 62)];
    let lead2 = vec![
        note(0, 16, 69),
        note(16, 16, 71),
        note(32, 16, 72),
        note(48, 16, 74),
    ];
    let src = emit_song(
        "align",
        &[
            ("ym".to_string(), "lead".to_string(), lead, Vec::new()),
            ("ym".to_string(), "lead2".to_string(), lead2, Vec::new()),
        ],
        1, // one sample per tick: exact grid, no rounding
        120.0,
        128,
    );
    let bar = src.split("bar!( // bar 1").nth(1).expect("intro bar 1");
    let sers: Vec<&str> = bar
        .lines()
        .filter(|line| line.contains("track!("))
        .take(2)
        .collect();
    assert_eq!(sers.len(), 2, "two channel lines:\n{bar}");
    let (a, b) = (sers[0], sers[1]);
    // Tick-32 hits carry no glued prefixes: same tick, same column.
    let col_a = a.find("d4q").expect("lead tick-32 hit");
    let col_b = b.find("c5e").expect("lead2 tick-32 hit");
    assert_eq!(col_a, col_b, "tick-32 alignment:\n{a}\n{b}");
    // The tick-16 hit sits strictly between tick 0 and tick 32.
    let col_16 = b.find("b4e").expect("lead2 tick-16 hit");
    let col_0 = b.find("a4e").expect("lead2 tick-0 hit");
    assert!(col_0 < col_16 && col_16 < col_b, "time order:\n{b}");
}
