# mmlx — Opusmodus function port plan

**Source surveyed:** `opusmodus.com` (OMN Language, NCODE, def-score docs),
forums (`gen-retrograde`, `gen-rotate`, `pitch-transpose`, `gen-divide`,
`pitch-mix/demix`, `chord-closest-path`, …), TOT extension library,
Wikipedia overview. Opusmodus full function reference lives inside the
app Assistant (not public web); signatures below are reconstructed from
public examples and must be verified against the Assistant before
stabilizing.

## 1. What Opusmodus is (for mapping)

* **OMN (Opusmodus Notation):** one s-expression per event stream:
  `(length pitch velocity articulation …)`, e.g. `(q c4 mf stacc)`,
  chords as `(h c4e4g4)`. Four parallel parameter lists kept in lockstep.
* **NCODE:** interactive browser over the function library; every result
  saved as a named variable (numbers/lengths/pitches/velocities/
  articulations/OMN). Equivalent to our future VSCode preview + history.
* **`make-omn` / `def-score` / `ps`:** assemble parameter lists into OMN,
  then assign OMN vars to instruments + global tempo/key/time-sig for
  notation/MIDI playback.
* **Library shape:** ~hundreds of small pure list functions in families
  `pitch-*`, `length-*`/`gen-*`, `velocity-*`, `articulation-*`,
  `chord/harmonic-*`, `rnd-*`, plus high-level algorithmic generators
  (L-systems, cellular automata, Euclidean, Markov, twelve-tone,
  Schillinger, Messiaen, Stravinsky rotation, Nørgård infinity,
  spectral import).

## 2. Mapping principles to mmlx

1. **OMN parameter lists → `Note` values.** OMN length→`duration`,
   pitch→`midi` (+cents later), velocity→`velocity/volume`,
   articulation (`stacc/legato/tenuto/…`)→`gate` + `attack/release_envelope`.
   No parallel-list bookkeeping; params ride on the note or in scope.
2. **Pure list functions → pure `Vec<Note>` functions + lazy iterators.**
   New crate `mmlx-algo`: `fn f(Vec<Note>) -> Vec<Note>` for finite
   material, plus `fn f_iter(It<Item=Note>) -> impl Iterator<Item=Note>`
   for infinite `gen!` streams. Streaming stays lazy (§1 of MMLX_SPEC).
3. **Deterministic + seeded.** Every `rnd-*` takes explicit `seed: u64`
   (e.g. `rnd-order … :seed 8` in forums). Required for idempotent builds.
4. **Text-first.** No `.omnc` session binaries; NCODE history = Rust
   `let` bindings + VSCode CodeLens preview (already planned).
5. **Out of scope for port:** notation engraving, MusicXML import,
   `def-sound-set` keyswitches, CLM synthesis — those belong to
   `mmlx-midi` / instrument crates later.

## 3. Function catalog (proposed `mmlx-algo` API)

Status: ✅ already in mmlx/CP437 · 🔶 port next · 🔷 later.

### 3.1 Constructors / accessors / measures

| Opusmodus (observed) | Meaning | mmlx proposal |
|---|---|---|
| `make-omn :pitch :length :velocity :articulation` | zip param lists into OMN | ✅ `ser!` + `param!` already zips; add `mmlx_algo::zip(pitches, durs, vels)` helper 🔶 |
| `pitch` / `length` / `velocity` / `articulation` extractors | project one lane | 🔶 `lanes(notes) -> {pitches, durs, vels, arts}` + `with_lane()` setters |
| `omn-to-measure`, `find-bar`, `get-time-signature` | bar/measure ops | 🔶 `to_bars(notes, time_sig)`, `find_bar(n)` over `ser!` sections |
| `filter-repeat` | dedupe repeats | 🔶 `dedup(n)` |

### 3.2 Generic sequence transforms (`gen-*`) — port first, they compose everything

| Opusmodus | mmlx |
|---|---|
| `gen-retrograde` (reverse) | 🔶 `retrograde(Vec<Note>)` + `retrograde_iter` |
| `gen-rotate n` | 🔶 `rotate(Vec<Note>, isize)` (cf. forum `gen-rotate 3`) |
| `gen-repeat` / `gen-loop` | ✅ `repeat!` covers single; add `repeat_section(notes, n)` 🔶 |
| `gen-divide n bar` (split event into n) | 🔶 `divide(note, n)` / `divide_all(notes, n)` |
| `gen-tendency` (morph list A→B over n steps) | 🔶 `tendency(a: Vec<f32>, b: Vec<f32>, steps, variance, seed)` → then `apply_to_lane` |
| `gen-mix / gen-shuffle / rnd-order` | 🔶 `shuffle(notes, seed)`, `interleave(a, b)` |
| `gen-eval` / `apply-eval` | ✅ Rust closures/iterators natively; document pattern |
| `gen-palindrome`, canon/rotation (Stravinsky) | 🔶 `palindrome()`, `canon(notes, interval, delay)` |

### 3.3 Pitch

| Opusmodus | mmlx |
|---|---|
| `pitch-transpose n` | ✅ `transpose_note(note, offset)` exists; add `transpose_all(notes, n)` + diatonic `scale_transpose(notes, degrees, scale)` 🔶 |
| `pitch-invert [axis]` | 🔶 `invert(notes, axis_midi)` |
| `pitch-mix / pitch-demix i` (chord↔voices) | 🔶 `demix(chords, voice_idx)` / `mix(voices)` — maps to `par!` voices |
| `pitch-to-integer / integer-to-pitch` | 🔶 `to_midis()` / `from_midis()` (+ microtonal `to_cents()`) |
| `ambitus 'violin …` (clamp/fold to range) | 🔶 `ambitus(notes, lo, hi, strategy: Clip/Fold)` |
| `make-scale 'd4 8 :alt '(…)` | 🔶 `scale(root, len, intervals)` generator |
| `chordize-list`, `chord-pitch-unique`, `dechord` | 🔶 `chordize(notes) -> Note::Parallel`, `dechord(predicate)` (cf. forum dechord-by-length recipe) |
| `chord-closest-path / chord-relative-path` | 🔷 `closest_path(chords)` voice-leading optimizer |
| pitch-class sets, twelve-tone row/all-interval, Klangreihen, Hauer tropes, tonality mapping | 🔷 `pcset::*`, `twelve_tone::row/matrix/retrograde/inversion`, `tonality::map(notes, key)` (key param already exists) |

### 3.4 Rhythm / length

| Opusmodus | mmlx |
|---|---|
| augmentation / diminution | 🔶 `augment(notes, ratio)` (multiply durations; ties re-resolved) |
| Euclidean rhythms | 🔶 `euclidean(pulses, steps, pitch, dur)` → rests + hits |
| Time-point system (Babbitt), Schillinger interference, Messiaen permutation | 🔷 `timepoint::…`, `schillinger::interfere(a, b)`, `messiaen::permute(notes, table)` |
| `length` pattern fns | 🔶 rhythm-only transforms reuse `gen-*` on durations lane |

### 3.5 Velocity / dynamics / articulation

| Opusmodus | mmlx |
|---|---|
| velocity lists, cresc./dim., `velocity-to-dynamic` | 🔶 `cresc(notes, from, to, curve)` = `volume`/`velocity` env spoiler; single-note `c4q!(velocity=…)` already ✅ |
| articulations (`stacc legato tenuto arp-down …`) | ✅ encode as `gate` + `attack/release_envelope` presets; add `articulate(notes, Art)` table 🔶 (forum `arp-down` → strum via `forkser!` arpeggio macro) |

### 3.6 Stochastic / algorithmic generators

Seeded, iterator-first:

| Opusmodus | mmlx |
|---|---|
| `rnd-sample n list`, `rnd-order`, `rnd-seed` | 🔶 `sample(notes, n, seed)`, `shuffle` (above) |
| random walk / Brownian / white/pink/Gaussian noise | 🔶 `walk::brownian(start, step, seed)` as `Iterator<f32>` → map to pitch/dur/param lane; synth noise меры already exist separately |
| Markov chains | 🔶 `markov::chain(order, corpus, seed)` iterator |
| L-systems (Lindenmayer), cellular automata, Mandelbrot, Rubin, Nørgård infinity series, sine/saw/square/triangle shaping, spectral data → pitches | 🔷 `gen::lindenmayer`, `gen::cellular(rule, …)`, `gen::infinity_series`, `gen::waveform`, `spectral::partials_to_pitches` — each a lazy iterator + `quantize_to_scale()` |

### 3.7 Score / audition (maps to our server, not algo crate)

`def-score` (layout + instruments + globals) → mmlx text-DAW section in
MMLX_SPEC §13 (`ser!` of `par!` voices + `param!(tempo/key/instrument)`).
`ps` preview → `mmlx-server` + VSCode transport. MusicXML/MIDI export →
`mmlx-midi` crate (planned).

## 4. Port order (suggested)

1. **P1 — deterministic core:** `retrograde rotate divide dedup transpose_all invert ambitus scale chordize/dechord demix/mix augment articulate cresc zip/lanes/to_bars`.
2. **P2 — seeded random:** `sample shuffle walk markov euclidean`.
3. **P3 — systems:** `pcset twelve_tone tonality schillinger messiaen timepoint tendency canon`.
4. **P4 — esoteric/analysis:** `lindenmayer cellular infinity spectral`.

Each function: doc example in Rust (runnable), property test
(e.g. `retrograde(retrograde(x))==x`, `rotate` length-preserving,
seeded determinism), and an `examples_songs` demo where musical.

## 5. Open questions for you

* Do you want OMN-style multi-lane authoring (`pitches=[…] lengths=[…]`
  zipped) as first-class sugar, or always note-atoms (`c4q`)? Proposal
  above supports both via `zip`.
* Microtonality now (cents in `pitch` env + `integer-to-pitch` float) or
  defer to P4?
* Which P1 subset should land with the `mmlx-core` split vs. follow-up?
