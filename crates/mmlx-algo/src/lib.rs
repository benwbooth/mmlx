//! `mmlx-algo`: Opusmodus-style composition functions over `Note` values.
//!
//! All transforms are pure (`Vec<Note>` in, `Vec<Note>` out) and operate on
//! resolved atoms/rests; other variants pass through untouched. Random
//! processes take an explicit `seed` (deterministic LCG) for idempotent builds.

use mmlx_core::{transpose_note, Note, ParamValue};

// --- accessors ---

/// Split a note into pitched (`Some(midi)`) + duration, if it is an atom/rest.
pub fn atom_parts(note: &Note) -> Option<(Option<u8>, f32)> {
    match note {
        Note::Atom { midi, duration, .. } => Some((Some(*midi), *duration)),
        Note::Rest { duration, .. } => Some((None, *duration)),
        _ => None,
    }
}

/// Clone a note with a new duration (atoms/rests only; others pass through).
pub fn with_duration(note: &Note, duration: f32) -> Note {
    match note {
        Note::Atom {
            midi, parameters, ..
        } => Note::Atom {
            midi: *midi,
            duration,
            parameters: parameters.clone(),
        },
        Note::Rest { parameters, .. } => Note::Rest {
            duration,
            parameters: parameters.clone(),
        },
        other => other.clone(),
    }
}

// --- P1: deterministic sequence transforms ---

/// Reverse note order. Involution: `retrograde(retrograde(x)) == x`.
pub fn retrograde(mut notes: Vec<Note>) -> Vec<Note> {
    notes.reverse();
    notes
}

/// Rotate left by `n` (negative rotates right). Length-preserving.
pub fn rotate(mut notes: Vec<Note>, n: isize) -> Vec<Note> {
    if notes.is_empty() {
        return notes;
    }
    let len = notes.len() as isize;
    let n = ((n % len) + len) % len;
    notes.rotate_left(n as usize);
    notes
}

/// Split one atom/rest into `n` equal parts (duration `/ n`).
pub fn divide(note: &Note, n: usize) -> Vec<Note> {
    assert!(n >= 1, "divide by zero");
    match atom_parts(note) {
        Some((_, duration)) => (0..n)
            .map(|_| with_duration(note, duration / n as f32))
            .collect(),
        None => vec![note.clone()],
    }
}

/// Divide every atom/rest in the list.
pub fn divide_all(notes: Vec<Note>, n: usize) -> Vec<Note> {
    notes.iter().flat_map(|note| divide(note, n)).collect()
}

/// Remove runs of the same pitch longer than `max_run` (rests break runs).
/// `filter_repeat(1, …)` keeps no immediate pitch repeats.
pub fn dedup_repeats(max_run: usize, notes: Vec<Note>) -> Vec<Note> {
    let mut out = Vec::with_capacity(notes.len());
    let mut run_pitch: Option<u8> = None;
    let mut run_len = 0;
    for note in &notes {
        match atom_parts(note) {
            Some((Some(midi), _)) if Some(midi) == run_pitch => {
                run_len += 1;
                if run_len <= max_run {
                    out.push(note.clone());
                }
            }
            Some((Some(midi), _)) => {
                run_pitch = Some(midi);
                run_len = 1;
                out.push(note.clone());
            }
            _ => {
                run_pitch = None;
                run_len = 0;
                out.push(note.clone());
            }
        }
    }
    out
}

/// Transpose every pitched note by semitones (clamped 0..=127).
pub fn transpose_all(notes: Vec<Note>, semitones: i8) -> Vec<Note> {
    notes
        .iter()
        .map(|note| transpose_note(note, semitones))
        .collect()
}

/// Mirror pitches around `axis_midi` (`pitch -> 2*axis - pitch`, clamped).
pub fn invert(notes: Vec<Note>, axis_midi: u8) -> Vec<Note> {
    notes
        .iter()
        .map(|note| match note {
            Note::Atom {
                midi,
                duration,
                parameters,
            } => Note::Atom {
                midi: (2 * axis_midi as i16 - *midi as i16).clamp(0, 127) as u8,
                duration: *duration,
                parameters: parameters.clone(),
            },
            other => other.clone(),
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AmbitusMode {
    Clip,
    Fold,
}

/// Force every pitch into `[lo, hi]` by clipping or octave-folding.
pub fn ambitus(notes: Vec<Note>, lo: u8, hi: u8, mode: AmbitusMode) -> Vec<Note> {
    assert!(lo <= hi);
    notes
        .iter()
        .map(|note| match note {
            Note::Atom {
                midi,
                duration,
                parameters,
            } => {
                let mut pitch = *midi;
                match mode {
                    AmbitusMode::Clip => pitch = pitch.clamp(lo, hi),
                    AmbitusMode::Fold => {
                        while pitch < lo {
                            pitch = pitch.saturating_add(12);
                        }
                        while pitch > hi {
                            pitch = pitch.saturating_sub(12);
                        }
                    }
                }
                Note::Atom {
                    midi: pitch,
                    duration: *duration,
                    parameters: parameters.clone(),
                }
            }
            other => other.clone(),
        })
        .collect()
}

/// Generate a scale run: `len` atoms of `duration`, stepping `intervals`.
pub fn scale(root: u8, intervals: &[i8], duration: f32, len: usize) -> Vec<Note> {
    let mut pitch = root as i16;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        if i > 0 {
            pitch += intervals[(i - 1) % intervals.len()] as i16;
        }
        out.push(Note::Atom {
            midi: pitch.clamp(0, 127) as u8,
            duration,
            parameters: Vec::new(),
        });
    }
    out
}

/// Stack atoms as a chord (`Note::Parallel`).
pub fn chordize(notes: Vec<Note>) -> Note {
    mmlx_core::par(notes)
}

/// Flatten one parallel level back into a voice list.
pub fn dechord(note: &Note) -> Vec<Note> {
    match note {
        Note::Parallel(voices) => voices.clone(),
        other => vec![other.clone()],
    }
}

/// Split chord-symbols into per-voice lines (cf. Opusmodus `pitch-demix`).
pub fn demix(chords: &[Note], voice: usize) -> Vec<Note> {
    chords
        .iter()
        .filter_map(|chord| dechord(chord).into_iter().nth(voice))
        .collect()
}

/// Recombine voice lines into chord-symbols (cf. `pitch-mix`).
pub fn mix(voices: &[Vec<Note>]) -> Vec<Note> {
    let len = voices.iter().map(Vec::len).max().unwrap_or(0);
    (0..len)
        .map(|i| {
            chordize(
                voices
                    .iter()
                    .filter_map(|voice| voice.get(i).cloned())
                    .collect(),
            )
        })
        .collect()
}

/// Multiply every atom/rest duration by `ratio` (augmentation/diminution).
pub fn augment(notes: Vec<Note>, ratio: f32) -> Vec<Note> {
    notes
        .iter()
        .map(|note| match atom_parts(note) {
            Some((_, duration)) => with_duration(note, duration * ratio),
            None => note.clone(),
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Articulation {
    Staccato,
    Tenuto,
    Legato,
}

/// Apply articulation as per-note `gate` (staccato 0.5, tenuto 0.9, legato 1.1).
pub fn articulate(notes: Vec<Note>, articulation: Articulation) -> Vec<Note> {
    let gate = match articulation {
        Articulation::Staccato => 0.5,
        Articulation::Tenuto => 0.9,
        Articulation::Legato => 1.1,
    };
    notes
        .into_iter()
        .map(|note| note.param("gate".to_string(), ParamValue::Number(gate)))
        .collect()
}

/// Linear velocity ramp across notes (`from`..=`to`, MIDI 0..=127 scale).
pub fn crescendo(notes: Vec<Note>, from: f32, to: f32) -> Vec<Note> {
    let len = notes.len().max(1) as f32;
    notes
        .into_iter()
        .enumerate()
        .map(|(i, note)| {
            let velocity = from + (to - from) * (i as f32 / (len - 1.0).max(1.0));
            note.param("velocity".to_string(), ParamValue::Number(velocity))
        })
        .collect()
}

fn existing_number(note: &Note, key: &str) -> Option<f32> {
    let parameters = match note {
        Note::Atom { parameters, .. }
        | Note::Rest { parameters, .. }
        | Note::AtomImplicitDuration { parameters, .. }
        | Note::PreviousPitch { parameters, .. } => parameters,
        _ => return None,
    };
    parameters.iter().find_map(|(name, value)| {
        if name == key {
            match value {
                ParamValue::Number(number) => Some(*number),
                _ => None,
            }
        } else {
            None
        }
    })
}

/// Humanize: seeded jitter of per-note velocity (±`velocity_amount` MIDI
/// units) and gate (±`gate_amount`), around existing values when present.
/// Deterministic for the same seed — the agent-friendly "make it feel live".
pub fn humanize(notes: Vec<Note>, velocity_amount: f32, gate_amount: f32, seed: u64) -> Vec<Note> {
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    let jitter =
        |state: &mut u64, amount: f32| ((lcg_next(state) % 2000) as f32 / 1000.0 - 1.0) * amount;
    notes
        .into_iter()
        .map(|note| {
            let base_velocity = existing_number(&note, "velocity").unwrap_or(100.0);
            let base_gate = existing_number(&note, "gate").unwrap_or(1.0);
            note.param(
                "velocity".to_string(),
                ParamValue::Number(
                    (base_velocity + jitter(&mut state, velocity_amount)).clamp(1.0, 127.0),
                ),
            )
            .param(
                "gate".to_string(),
                ParamValue::Number((base_gate + jitter(&mut state, gate_amount)).clamp(0.05, 2.0)),
            )
        })
        .collect()
}

/// Harmonize: original voice plus a transposed copy per interval, as `par!`.
/// E.g. intervals `[4, 7]` stacks a major triad above every note.
pub fn harmonize(notes: Vec<Note>, intervals: &[i8]) -> Note {
    let mut voices = vec![mmlx_core::ser(notes.clone())];
    for interval in intervals {
        voices.push(mmlx_core::ser(transpose_all(notes.clone(), *interval)));
    }
    mmlx_core::par(voices)
}

/// Continue a melody: build a first-order Markov chain over the corpus
/// pitches and generate `len` new atoms of `duration`. Deterministic per seed.
pub fn continue_melody(corpus: &[Note], duration: f32, len: usize, seed: u64) -> Vec<Note> {
    let classes: Vec<u8> = corpus
        .iter()
        .filter_map(|note| match note {
            Note::Atom { midi, .. } => Some(*midi),
            _ => None,
        })
        .collect();
    markov(&classes, len, seed)
        .into_iter()
        .map(|midi| Note::Atom {
            midi,
            duration,
            parameters: Vec::new(),
        })
        .collect()
}

/// Zip parallel lanes (cf. Opusmodus `make-omn`): pitches × durations.
pub fn zip(pitches: Vec<u8>, durations: Vec<f32>) -> Vec<Note> {
    let len = pitches.len().max(durations.len());
    (0..len)
        .map(|i| Note::Atom {
            midi: pitches[i % pitches.len()],
            duration: durations[i % durations.len()],
            parameters: Vec::new(),
        })
        .collect()
}

/// Project a list into its pitch lane (`None` for rests/non-atoms) + durations.
pub fn lanes(notes: &[Note]) -> (Vec<Option<u8>>, Vec<f32>) {
    notes
        .iter()
        .map(|note| match atom_parts(note) {
            Some((pitch, duration)) => (pitch, duration),
            None => (None, 0.0),
        })
        .unzip()
}

/// Group atoms into bars of `bar_quarters` quarter-notes by duration sum.
pub fn to_bars(notes: Vec<Note>, bar_quarters: u32) -> Vec<Vec<Note>> {
    let bar_whole = bar_quarters as f32 * 0.25;
    let mut bars: Vec<Vec<Note>> = vec![Vec::new()];
    let mut used = 0.0;
    for note in notes {
        let duration = atom_parts(&note)
            .map(|(_, duration)| duration)
            .unwrap_or(0.0);
        if used + duration > bar_whole + 1e-6 && !bars.last().unwrap().is_empty() {
            bars.push(Vec::new());
            used = 0.0;
        }
        bars.last_mut().unwrap().push(note);
        used += duration;
    }
    bars
}

// --- P2: seeded processes (deterministic LCG, no external RNG) ---

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 33
}

/// Shuffle with explicit seed. Deterministic for idempotent builds.
pub fn shuffle(mut notes: Vec<Note>, seed: u64) -> Vec<Note> {
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    for i in (1..notes.len()).rev() {
        let j = (lcg_next(&mut state) as usize) % (i + 1);
        notes.swap(i, j);
    }
    notes
}

/// Sample `n` elements with replacement at `seed`.
pub fn sample(notes: &[Note], n: usize, seed: u64) -> Vec<Note> {
    if notes.is_empty() || n == 0 {
        return Vec::new();
    }
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    (0..n)
        .map(|_| notes[(lcg_next(&mut state) as usize) % notes.len()].clone())
        .collect()
}

/// Random walk over scale degrees: yields `len` MIDI pitches.
pub fn walk(start: u8, step_max: i8, len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    let mut pitch = start as i16;
    let span = (2 * step_max as i16 + 1).max(1);
    (0..len)
        .map(|_| {
            let step = (lcg_next(&mut state) as i16 % span) - step_max as i16;
            pitch = (pitch + step).clamp(0, 127);
            pitch as u8
        })
        .collect()
}

/// First-order Markov chain over pitch classes from a corpus, seeded.
pub fn markov(corpus: &[u8], len: usize, seed: u64) -> Vec<u8> {
    use std::collections::HashMap;
    if corpus.is_empty() || len == 0 {
        return Vec::new();
    }
    let mut transitions: HashMap<u8, Vec<u8>> = HashMap::new();
    for pair in corpus.windows(2) {
        transitions.entry(pair[0]).or_default().push(pair[1]);
    }
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    let mut current = corpus[(lcg_next(&mut state) as usize) % corpus.len()];
    (0..len)
        .map(|_| {
            let out = current;
            current = match transitions.get(&current) {
                Some(next) if !next.is_empty() => {
                    next[(lcg_next(&mut state) as usize) % next.len()]
                }
                _ => corpus[(lcg_next(&mut state) as usize) % corpus.len()],
            };
            out
        })
        .collect()
}

/// Euclidean rhythm: `pulses` hits spread over `steps` slots (Bjorklund).
/// Returns durations: hit `q`, rest `rq` per slot.
pub fn euclidean(pulses: usize, steps: usize, hit: Note, rest: Note) -> Vec<Note> {
    if steps == 0 {
        return Vec::new();
    }
    let pulses = pulses.min(steps);
    let mut pattern = vec![false; steps];
    // Even spread via modular arithmetic (equivalent to Bjorklund).
    for i in 0..pulses {
        pattern[(i * steps) / pulses] = true;
    }
    pattern
        .into_iter()
        .map(|hit_slot| if hit_slot { hit.clone() } else { rest.clone() })
        .collect()
}

// --- P3: systems ---

/// Twelve-tone row forms: prime/retrograde/inversion/retrograde-inversion.
pub fn twelve_tone(row: &[u8]) -> [Vec<u8>; 4] {
    assert_eq!(row.len(), 12, "row must have 12 pitch classes");
    let prime = row.to_vec();
    let mut retrograde = prime.clone();
    retrograde.reverse();
    let inversion: Vec<u8> = prime.iter().map(|pitch| (24 - *pitch) % 12).collect();
    let mut retrograde_inversion = inversion.clone();
    retrograde_inversion.reverse();
    [prime, retrograde, inversion, retrograde_inversion]
}

/// Schillinger-style interference: rhythmic resultant of periods `a` and `b`.
/// Yields attack-point gaps (in quarter units) over the LCM cycle.
pub fn schillinger(a: u32, b: u32) -> Vec<u32> {
    assert!(a > 0 && b > 0);
    let lcm = a * b / gcd(a, b);
    let mut attacks: Vec<u32> = (0..lcm).filter(|t| t % a == 0 || t % b == 0).collect();
    attacks.push(lcm);
    attacks.windows(2).map(|pair| pair[1] - pair[0]).collect()
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

/// Messiaen-style permutation: reorder by 1-based `table` (wraps).
pub fn messiaen_permute(mut notes: Vec<Note>, table: &[usize]) -> Vec<Note> {
    if notes.is_empty() || table.is_empty() {
        return notes;
    }
    let original = notes.clone();
    for (i, slot) in notes.iter_mut().enumerate() {
        *slot = original[(table[i % table.len()] - 1) % original.len()].clone();
    }
    notes
}

/// Morph lane `a` into lane `b` over `steps` (linear tendency + noise).
pub fn tendency(a: &[f32], b: &[f32], steps: usize, variance: f32, seed: u64) -> Vec<f32> {
    if steps == 0 || a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    (0..steps)
        .map(|i| {
            let t = if steps == 1 {
                1.0
            } else {
                i as f32 / (steps - 1) as f32
            };
            let from = a[i % a.len()];
            let to = b[i % b.len()];
            let noise = if variance > 0.0 {
                ((lcg_next(&mut state) % 2000) as f32 / 1000.0 - 1.0) * variance
            } else {
                0.0
            };
            from + (to - from) * t + noise
        })
        .collect()
}

/// Canon: original plus transposed entries delayed by whole-note rests.
pub fn canon(notes: Vec<Note>, entries: &[(i8, f32)]) -> Note {
    let mut voices = vec![mmlx_core::ser(notes.clone())];
    for (interval, delay) in entries {
        voices.push(mmlx_core::ser(vec![
            Note::Rest {
                duration: *delay,
                parameters: Vec::new(),
            },
            mmlx_core::ser(transpose_all(notes.clone(), *interval)),
        ]));
    }
    mmlx_core::par(voices)
}

// --- P4: generative series ---

/// Lindenmayer system: rewrite `axiom` with `rules` for `depth` iterations.
pub fn lindenmayer(axiom: &str, rules: &[(char, &str)], depth: usize) -> String {
    let mut current = axiom.to_string();
    for _ in 0..depth {
        let mut next = String::with_capacity(current.len() * 2);
        for ch in current.chars() {
            match rules.iter().find(|(from, _)| *from == ch) {
                Some((_, to)) => next.push_str(to),
                None => next.push(ch),
            }
        }
        current = next;
    }
    current
}

/// Map an L-system string to pitches: `up` steps +1 degree, `down` -1.
pub fn lsystem_to_pitches(
    system: &str,
    start: u8,
    scale: &[i8],
    up: char,
    down: char,
    duration: f32,
) -> Vec<Note> {
    let mut degree = 0i32;
    let mut pitch = start as i16;
    let mut out = Vec::new();
    for ch in system.chars() {
        if ch == up {
            degree += 1;
        } else if ch == down {
            degree -= 1;
        } else {
            continue;
        }
        pitch += scale[degree.rem_euclid(scale.len() as i32) as usize] as i16;
        out.push(Note::Atom {
            midi: pitch.clamp(0, 127) as u8,
            duration,
            parameters: Vec::new(),
        });
    }
    out
}

/// Nørgård infinity series (integer skeleton, first `len` values from seed pair).
pub fn infinity_series(seed0: i32, seed1: i32, len: usize) -> Vec<i32> {
    let mut series = vec![seed0, seed1];
    while series.len() < len {
        let mut next = Vec::with_capacity(series.len() * 2);
        for value in &series {
            next.push(-*value);
            next.push(*value + 1);
        }
        series = next;
    }
    series.truncate(len);
    series
}

/// Map spectral partials (Hz) to MIDI pitches relative to a base frequency.
pub fn partials_to_pitches(partials_hz: &[f32], base_hz: f32) -> Vec<u8> {
    partials_hz
        .iter()
        .map(|frequency| {
            (69.0 + 12.0 * (frequency / base_hz).log2())
                .round()
                .clamp(0.0, 127.0) as u8
        })
        .collect()
}
