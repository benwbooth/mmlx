//! Property + example tests for every `mmlx-algo` function.

use mmlx_algo::*;
use mmlx_core::{Note, ParamValue};

fn atom(midi: u8, duration: f32) -> Note {
    Note::Atom {
        midi,
        duration,
        parameters: Vec::new(),
    }
}

fn rest(duration: f32) -> Note {
    Note::Rest {
        duration,
        parameters: Vec::new(),
    }
}

fn midis(notes: &[Note]) -> Vec<Option<u8>> {
    notes
        .iter()
        .map(|note| match note {
            Note::Atom { midi, .. } => Some(*midi),
            _ => None,
        })
        .collect()
}

#[test]
fn retrograde_is_involution() {
    let notes = vec![atom(60, 0.25), atom(62, 0.25), atom(64, 0.5)];
    assert_eq!(retrograde(retrograde(notes.clone())), notes);
    assert_eq!(
        midis(&retrograde(notes)),
        vec![Some(64), Some(62), Some(60)]
    );
}

#[test]
fn rotate_properties() {
    let notes = vec![atom(60, 0.25), atom(62, 0.25), atom(64, 0.25)];
    assert_eq!(rotate(notes.clone(), 0), notes);
    assert_eq!(
        midis(&rotate(notes.clone(), 1)),
        vec![Some(62), Some(64), Some(60)]
    );
    assert_eq!(rotate(notes.clone(), 3), notes);
    assert_eq!(rotate(notes.clone(), -1), rotate(notes, 2));
}

#[test]
fn divide_splits_duration() {
    let parts = divide(&atom(60, 0.5), 4);
    assert_eq!(parts.len(), 4);
    let total: f32 = parts.iter().map(|note| atom_parts(note).unwrap().1).sum();
    assert!((total - 0.5).abs() < 1e-6);
    assert_eq!(divide_all(vec![atom(60, 0.25), rest(0.25)], 2).len(), 4);
}

#[test]
fn dedup_keeps_runs() {
    let notes = vec![
        atom(60, 0.25),
        atom(60, 0.25),
        atom(60, 0.25),
        atom(62, 0.25),
    ];
    assert_eq!(dedup_repeats(1, notes).len(), 2);
}

#[test]
fn transpose_clamps_and_invert_mirrors() {
    assert_eq!(
        midis(&transpose_all(vec![atom(127, 0.25)], 5)),
        vec![Some(127)]
    );
    let notes = vec![atom(60, 0.25), atom(64, 0.25)];
    assert_eq!(midis(&invert(notes, 60)), vec![Some(60), Some(56)]);
}

#[test]
fn ambitus_clip_and_fold() {
    let notes = vec![atom(40, 0.25), atom(90, 0.25)];
    assert_eq!(
        midis(&ambitus(notes.clone(), 60, 72, AmbitusMode::Clip)),
        vec![Some(60), Some(72)]
    );
    // 40 + 24 = 64 in range; 90 - 24 = 66 in range.
    assert_eq!(
        midis(&ambitus(notes, 60, 72, AmbitusMode::Fold)),
        vec![Some(64), Some(66)]
    );
}

#[test]
fn scale_chords_and_voices() {
    let run = scale(60, &[2, 2, 1, 2, 2, 2, 1], 0.25, 8);
    assert_eq!(midis(&run).last(), Some(&Some(72)));
    let chord = chordize(vec![atom(60, 0.5), atom(64, 0.5)]);
    assert_eq!(dechord(&chord).len(), 2);
    let chords = vec![chord];
    assert_eq!(midis(&demix(&chords, 1)), vec![Some(64)]);
    assert_eq!(mix(&[vec![atom(60, 0.5)], vec![atom(64, 0.5)]]).len(), 1);
}

#[test]
fn augment_articulate_crescendo() {
    let doubled = augment(vec![atom(60, 0.25)], 2.0);
    assert!((atom_parts(&doubled[0]).unwrap().1 - 0.5).abs() < 1e-6);
    let stacc = articulate(vec![atom(60, 0.25)], Articulation::Staccato);
    match &stacc[0] {
        Note::Atom { parameters, .. } => {
            assert!(parameters.iter().any(|(key, value)| key == "gate"
                && matches!(value, ParamValue::Number(v) if (*v - 0.5).abs() < 1e-6)));
        }
        _ => panic!("expected atom"),
    }
    let ramp = crescendo(vec![atom(60, 0.25), atom(62, 0.25)], 60.0, 100.0);
    assert_eq!(ramp.len(), 2);
}

#[test]
fn zip_lanes_bars() {
    let zipped = zip(vec![60, 64], vec![0.25]);
    assert_eq!(zipped.len(), 2);
    let (pitches, durations) = lanes(&zipped);
    assert_eq!(pitches, vec![Some(60), Some(64)]);
    assert_eq!(durations, vec![0.25, 0.25]);
    // Two quarters (0.5 whole) fill one 2-beat bar; third starts a new bar.
    let bars = to_bars(vec![atom(60, 0.25), atom(62, 0.25), atom(64, 0.25)], 2);
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].len(), 2);
}

#[test]
fn seeded_processes_deterministic() {
    let notes = vec![
        atom(60, 0.25),
        atom(62, 0.25),
        atom(64, 0.25),
        atom(65, 0.25),
    ];
    assert_eq!(shuffle(notes.clone(), 7), shuffle(notes.clone(), 7));
    // Permutation check: same multiset of pitches.
    let mut a = midis(&shuffle(notes.clone(), 7));
    let mut b = midis(&notes);
    a.sort();
    b.sort();
    assert_eq!(a, b);
    assert_eq!(sample(&notes, 5, 3).len(), 5);
    assert_eq!(sample(&notes, 5, 3), sample(&notes, 5, 3));
    let walked = walk(60, 2, 50, 11);
    assert!(walked.iter().all(|pitch| *pitch <= 127));
    let chain = markov(&[60, 62, 64, 62, 60], 20, 5);
    assert!(chain.iter().all(|pitch| [60, 62, 64].contains(pitch)));
}

#[test]
fn euclidean_spreads_hits() {
    let pattern = euclidean(3, 8, atom(60, 0.25), rest(0.25));
    assert_eq!(pattern.len(), 8);
    assert_eq!(
        pattern
            .iter()
            .filter(|note| matches!(note, Note::Atom { .. }))
            .count(),
        3
    );
}

#[test]
fn row_forms_and_systems() {
    let row: Vec<u8> = (0..12).collect();
    let [prime, retro, inv, ri] = twelve_tone(&row);
    assert_eq!(prime, row);
    assert_eq!(retro, row.iter().rev().cloned().collect::<Vec<_>>());
    assert_eq!(inv[0], 0);
    assert_eq!(ri, inv.iter().rev().cloned().collect::<Vec<_>>());
    // Schillinger(3,4): attacks at 0,3,4,6,8,9,12 -> gaps sum to 12.
    let gaps = schillinger(3, 4);
    assert_eq!(gaps.iter().sum::<u32>(), 12);
    let notes = vec![atom(60, 0.25), atom(62, 0.25), atom(64, 0.25)];
    let permuted = messiaen_permute(notes.clone(), &[3, 1, 2]);
    assert_eq!(midis(&permuted), vec![Some(64), Some(60), Some(62)]);
    // Tendency with zero variance interpolates exactly.
    let morph = tendency(&[0.0], &[10.0], 3, 0.0, 0);
    assert_eq!(morph, vec![0.0, 5.0, 10.0]);
    let canon_note = canon(vec![atom(60, 0.25)], &[(7, 0.25)]);
    assert!(matches!(canon_note, Note::Parallel(voices) if voices.len() == 2));
}

#[test]
fn humanize_jitters_within_bounds() {
    let notes = vec![atom(60, 0.25), atom(62, 0.25), atom(64, 0.25)];
    let once = humanize(notes.clone(), 10.0, 0.1, 42);
    assert_eq!(humanize(notes.clone(), 10.0, 0.1, 42), once);
    for note in &once {
        match note {
            Note::Atom { parameters, .. } => {
                let velocity = parameters.iter().find(|(key, _)| key == "velocity");
                match velocity {
                    Some((_, ParamValue::Number(velocity))) => {
                        assert!((90.0..=110.0).contains(velocity), "velocity {velocity}");
                    }
                    _ => panic!("humanize sets velocity"),
                }
            }
            _ => panic!("expected atoms"),
        }
    }
    // Zero amount leaves values untouched.
    assert_eq!(
        humanize(notes.clone(), 0.0, 0.0, 1),
        crescendo(notes, 100.0, 100.0)
            .into_iter()
            .map(|note| note.param("gate".to_string(), ParamValue::Number(1.0)))
            .collect::<Vec<_>>()
    );
}

#[test]
fn harmonize_and_continue() {
    let line = vec![atom(60, 0.25), atom(62, 0.25)];
    let harmony = harmonize(line.clone(), &[4, 7]);
    let mmlx_core::Note::Parallel(voices) = harmony else {
        panic!("harmonize yields Parallel");
    };
    assert_eq!(voices.len(), 3);
    let continued = continue_melody(&line, 0.25, 8, 9);
    assert_eq!(continued.len(), 8);
    assert_eq!(continue_melody(&line, 0.25, 8, 9), continued);
    assert!(
        continued
            .iter()
            .all(|note| matches!(note, mmlx_core::Note::Atom { midi: 60 | 62, .. })),
        "continuation stays in the corpus pitch set"
    );
}

#[test]
fn remix_and_piano_roll() {
    let sections = vec![
        vec![atom(60, 0.25)],
        vec![atom(64, 0.25)],
        vec![atom(67, 0.25)],
    ];
    let remixed = remix(sections, &[2, 0], false, 0);
    assert_eq!(midis(&remixed), vec![Some(67), Some(60)]);
    // Out-of-range positions are skipped.
    assert_eq!(remix(vec![vec![atom(60, 0.25)]], &[5], false, 0).len(), 0);

    let song = mmlx_core::ser(vec![atom(60, 0.5), atom(64, 0.25)]);
    let events: Vec<_> = song.event_stream(0.0).collect();
    let roll = piano_roll(&events);
    let rows: Vec<&str> = roll.lines().collect();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].contains(" 60 ") && rows[0].starts_with("0.000"));
    assert!(rows[1].contains(" 64 "));
}

#[test]
fn generative_series() {
    // L-system: A->AB, B->A gives Fibonacci words.
    let word = lindenmayer("A", &[('A', "AB"), ('B', "A")], 4);
    assert_eq!(word, "ABAABABA");
    let melody = lsystem_to_pitches("+-++", 60, &[2], '+', '-', 0.25);
    assert_eq!(melody.len(), 4);
    assert_eq!(infinity_series(0, 1, 8).len(), 8);
    // A440 partials resolve to MIDI 69.
    assert_eq!(partials_to_pitches(&[440.0, 880.0], 440.0), vec![69, 81]);
}
