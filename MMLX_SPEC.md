# mmlx — Music Macro Language Extended

**Version:** 0.1 (Draft, derived from CP437 Castle Audio)
**Date:** 2026-09-14
**Status:** Language spec first; implementation follows as standalone Rust DSL.
**Name:** `mmlx` (MML extended; Rust-embedded, text-first DAW language).

## 0. Goals

1. **Rust DSL, not a separate file format.** Music is written as Rust expressions using macros + consts. This gives conditionals, functions, loops, and agent-generated code for free. No custom parser in v1.
2. **Everything streams.** Three lazy stages, all potentially infinite:
   `Note (composition) -> EventStream (piano roll) -> SampleStream (audio) -> Backend`.
   Backpressure via iterators/generators; never materialize full song.
3. **Stable Rust generators.** Use `genawaiter::sync::gen!` + `yield_!` (as CP437 does). No nightly `gen`/`yield`. `Note::event_stream()`, `note_stream_to_event_stream()`, and `Instrument::generate_samples()` are all `Iterator`-based.
4. **Text-defined DAW.** Mixer routing, instrument choice, tempo map, structure, and automation are all `Note`/`param!`/`env!` text. Future piano-roll UI is a *view* over the event stream, never the source of truth. Agent-friendly.
5. **Superset of CP437 Castle Audio.** Every CP437 construct works unchanged. Enhancements are additive and documented in §15.
6. **Powerful envelope sublanguage.** Any `Number`-ish param (including `pitch`, `note`, `duty`, `gate`, `volume`, `transpose`, `tempo`) can be automated with one concise `env!`-family macro.

Non-goals for v0.1: custom `.mmlx` text parser, VST/CLAP hosting, MIDI file I/O (reserved, see §14).

---

## 1. Streaming architecture

```
+----------------+    gen!/yield_     +------------------+   process_event/    +----------------+   cpal/fundsp   +---------+
| Composition    | ----------------> | Piano roll       | ------------------> | Sample stream  | --------------> | Backend |
| Note tree      |  event_stream()   | TimedMusicalEvent|  generate_samples() | [f32;2] stereo |   push/pull   | speaker |
| finite or loop |  note_stream_to_  | NoteOn/NoteOff/  |  per-Instrument,    | finite or      |                 |         |
| {loop{...}}    |  event_stream()   | Rest/SetParam    |  mixed per block    | infinite       |                 |         |
+----------------+                   +------------------+                     +----------------+                 +---------+
```

* **Stage 1 — Composition (`Note`).** Pure value. Built with `ser/par/parmin/forkseq/forkpar/repeat/param/comment` + note consts. May embed `gen!` loops yielding `Note` forever (CP437 SPEC §2.8).
* **Stage 2 — Piano roll (`TimedMusicalEvent`).** `Note::generate_events_recursive()` walks the tree with `active_params`, yielding timestamped events via `Co<TimedMusicalEvent>`. `par/parmin/forkpar` merge child streams with a `BinaryHeap` by `time_seconds`. Infinite inputs yield infinite outputs; consumer decides when to stop (`take()`, `NoteOff`, server `stop/reset`).
* **Stage 3 — Audio (`[f32;2]`).** Each `Instrument` implements `process_event/process_events/generate_samples/is_idle`. The backend (CP437 `audio.rs:setup_audio`) drains a shared `VecDeque<TimedMusicalEvent>` queue in fixed blocks (e.g. 16384 frames), segments blocks at mid-block event times for sample-accurate scheduling, mixes instruments, applies a global LPF, advances `synth_time`.

All time in stage 2 is absolute `f32/f64` seconds from sequence start. Stage 3 maps to sample counts via `rate`.

---

## 2. Core concepts

| Term | Meaning |
|---|---|
| Note atom | Pitched event: MIDI + metric duration + per-note params. |
| Rest | Silence with metric duration. |
| Tie | Bare duration (`q`, `hd`) that *adds* to previous atom/rest. |
| Previous pitch (`p`) | Reuse previous MIDI, optional new duration. |
| Implicit duration | `c4` (no duration) inherits last duration in block. |
| Param | Lexically scoped key-value controlling interpretation (`tempo`) or synthesis (`patch`). |
| Envelope | Time-varying function bound to a param target. |
| Sequence | `ser!` time advances by sum; `par!` by max; `parmin!` by min; `fork*!` by zero. |
| Macro | Named `Note` fragment substituted for each atom in scope, transposed to trigger pitch. |
| Event stream | Sorted `TimedMusicalEvent` iterator — the "piano roll". |
| Instrument | `process_event` + `generate_samples` impl (e.g. `BasicSynth`). |

---

## 3. Note literals

Lowercase only. Generated as `pub const` + `macro_rules!` with optional `(key=value, ...)` suffix (CP437 `play_macro/src/lib.rs`).

### 3.1 Pitched: `<letter><acc?><oct><dur?>`

* Letter: `c d e f g a b`.
* Accidental: `s`=+1, `f`=-1, `ss`=+2, `ff`=-2, `n`/`nn`=0. No suffix = natural.
* Octave: `-1..9`. `-1` spelled `_1` (e.g. `c_1`, `bb_1t`). Middle C = `c4` = MIDI 60. Formula: `(oct+1)*12 + base_pc + acc`.
* Duration (optional): base `w h q e i t x o` = `1.0 0.5 0.25 0.125 0.0625 0.03125 0.015625 0.0078125` whole-note fractions. Dotted: `d`=×1.5, `dd`=×1.75, `ddd`=×1.875. E.g. `qd hdd eddd`.
* With duration → `Note::Atom { midi, duration, parameters }`. Without → `Note::AtomImplicitDuration { midi, parameters }` (resolved in §5).
* Examples: `c4q`, `fs5hd`, `bb_1tddd!`, `c4`, `fs5!(volume=0.8)`.
* Per-note attrs: `c4q!(volume=0.8, duty=)` — same key/value resolution as `param!`; trailing `=` with no value means `Unset` for that note only.

### 3.2 Rests: `r<dur>`

`rq rhddd re` → `Note::Rest`. Optional attrs same as above.

### 3.3 Previous pitch: `p<dur?>`

`p` (inherit pitch+duration), `pq`, `phd` → `Note::PreviousPitch { optional_duration, parameters }`. Default pitch if none yet: C4 (60).

### 3.4 Ties: bare `<dur>`

`q hd eddd` used as *values* (not macro calls) → `Note::DurationTie { duration }`. Must follow atom/rest in same block; adds durations (§5). Example: `ser!([c4q, q, e4q])`.

---

## 4. Composition operators

All take array/`IntoIterator<Item=Note>`. Functions `ser/par/parmin/forkseq/forkpar` + `macro_rules!` wrappers `ser!/par!/parmin!/forkser!/forkpar!` are equivalent; spec shows `!` form.

Body forms (all equivalent — a proc-macro frontend splits items, the
runtime functions resolve): `ser!([c4q, d4q])` (legacy) ≡ `ser!(c4q, d4q)`
≡ `ser!(c4q d4q)`. Whitespace separation recognizes notes, rests, ties,
`repeat!`/`param!`/`comment!`, nested `ser!`/`par!`, per-note `!(...)`
attrs, `::` paths and block constructs as units; a lone `(...)` group
passes through as one expression.

| Op | Time advance | Param scope | Notes |
|---|---|---|---|
| `ser!([a,b,c])` | sum | `param!` inside applies forward within block (lexical, §7) | Resolves implicit/tie/repeat inline. |
| `par!([a,b])` | max child end | each child gets independent clone of entry params; writes discarded | heap-merged by time. |
| `parmin!([a,b])` | min child end | same as `par!` | events at/after min cut off. |
| `forkser!([...])` / `forkseq` | **0** | isolated clone | sequential fork playing alongside following siblings. |
| `forkpar!([...])` | **0** | isolated clone | parallel fork alongside siblings. |
| `repeat!(n)` → `RepeatMarker(n)` | repeats previous item `n` extra times | inherits context at each repetition | must follow atom/rest/`ser`/`par`; `0` = no-op. See §5.4. |
| `comment!("...")` | 0 | none | yields `Comment` event for logging/highlight. |

`ser!([c4q, forkser!([e4q, g4q]), d4q])`: `e+g` starts at `c4q` end, runs alongside `d4q`.

Nesting is free: `par!([ser!([...]), ser!([...])])` is the standard multi-voice pattern.

Voice programs as variables: a `ser!` holding only `param!` setters splices
like any nested block — its params leak forward to following siblings
(`nested_ser_shares_ambient_params`). Decompiled songs bind each recurring
voice program once per song fn as `let voice_<role>: Note` and splice
`voice.clone()`, so a lane reads as program changes plus notes; programs
used exactly once inline as `param!(...)` and small tweaks stay inline
`param!` diffs. Lanes emit in score order
(melody on top, drums at the bottom) with one `// bar N` line per bar, so
parts align vertically like staff systems. Repeated phrases recurring 3+
times with net savings extract to bar-local `seg_*()` functions (never
crossing a barline, never opening with a bare tie).

---

## 5. Block resolution (`ser/par/parmin/fork*`)

Runs at construction (before event generation). Each block tracks `last_duration`, `last_pitch_midi`, `current_attrs`, and for `par`-family `last_note_index` for ties.

1. **Tie:** pop last resolved atom/rest, `duration += tie`. Error if none.
2. **Repeat:** pop last resolved repeatable (atom/rest/serial/parallel/fork), push back original, then push `n` resolved clones with `current_attrs` applied.
3. **Implicit:** `AtomImplicitDuration` → `Atom` with `last_duration` (panic with clear message if none). `PreviousPitch` → `Atom` with `last_pitch.unwrap_or(60)` + `optional_duration.or(last_duration)`.
4. **Attrs:** `apply_parameters()` merges `current_attrs` into atom/rest/implicit/previous (skips `key`; existing per-note keys win over block keys unless overwritten). Recurses into nested `Serial/Parallel/...`.
5. **Context update:** atoms/rests update `last_*`; `ParamSetter` updates `current_attrs` and (in `ser/forkseq`) is also pushed as a `Note::ParamSetter` node so event generation emits `SetParameter`; in `par/parmin/forkpar` setters update context but are *not* pushed as siblings (they ride with their branch).
6. **Bare envelope is an error.** `env!(...)` alone inside any block panics: must be `param!(target = env!(...))`.

---

## 6. `param!` — parameters

```rust
param!(tempo=120)
param!(tempo=120, patch="sawtooth")
param!(macro=)              // unset
c4q!(volume=0.8, duty=)     // per-note; duty unset for this note only
```

* Single pair → one `Note::ParamSetter`. Multiple pairs → `Note::Serial` of setters.
* **Key shortening:** unambiguous case-insensitive prefix accepted (`time_b`→`time_beat`, `wave`→`patch` if unique). Ambiguous/unknown → panic with candidates (CP437 `macros.rs:match_prefix`).
* **Value coercion:** `"120"` string parses to `Number` if numeric; patch/noise/preset strings prefix-match (`"sq"`→`"square"`).
* **Unset:** `param!(key=)` / `ParamValue::Unset` removes key from scope and emits `SetParameter{Unset}` so synth clears block envelopes.
* **Scope:** lexical. `ser/forkseq` propagate forward; exiting block restores outer map. `par`-family branches are isolated. Per-note attrs never leak.

### Canonical keys (v0.1, frozen from CP437)

| Key | Kind | Type | Default | Desc |
|---|---|---|---|---|
| `tempo` | interp | Number | 60.0 | BPM. |
| `time_note` | interp | Number | 4.0 | note value per beat. `time_b` alias for beats-per-bar reserved. |
| `time_beat` | interp | Number | — | beats per bar (display/metronome; does not alter duration math). |
| `key` | interp | String | unset (C) | e.g. `"d"`, `"am"`, `"Bbmaj"`. Applies sharps/flats to naturals; explicit accidental wins. |
| `instrument` | interp | String | `"basic_synth"` | selects `Instrument`. |
| `macro` | interp | Note | Unset | ornament fragment (§8). |
| `gate` | interp | Number/Env | 1.0 | sounding/metric ratio. >1 legato overlap. |
| `patch` | synth | String | `"square"` | `sine square triangle sawtooth noise`. Prefix-matched. |
| `duty` | synth | Number/Env | 0.5 | square pulse width 0..1. |
| `noise_type` | synth | String | `"white"` | `white periodic brown pink`. |
| `volume` | synth | Number/Env | 1.0 | static/block multiplier. |
| `velocity` | synth | Number/Env | 100/127 | per-note 0..127 scaled. |
| `note_volume`/`adsr` | synth | Number/Env | — | legacy primary-volume override; `Number` → `velocity=num/127`; `Envelope` retargeted to `volume`. |
| `transpose` | synth | Number/Env | 0 | semitones, static or automated. |
| `pitch` | synth | Env | — | fine continuous semitone offset. |
| `note` | synth | Env | — | stepped (rounded) semitone offset. |
| `attack_envelope` | synth | Env | Unset | ADS shape, once. |
| `sustain_envelope` | synth | Env | Unset | looped until NoteOff. |
| `release_envelope` | synth | Env | Unset | played on NoteOff. |
| `release_duration` | synth | Number | 0.1 | seconds to play release if no env duration. |
| `pan` | synth | Number/Env | 0.5 | stereo position 0=left..1=right (equal-power). |
| `bus` | mixer | String | — | named mix bus this voice routes to (Phase 5). |
| `send` | mixer | Number | — | send amount to `bus` (Phase 5). |
| `envelope_preset` | meta | String | — | e.g. `"percussion"`; sugar expanding to env triple. |
| `accidental_suffix` | meta | String | — | internal parser residue; do not set. |

Adding keys requires spec amendment; instruments ignore unknown keys they don't implement.

---

## 7. Time model

Metric duration `d` (whole=1.0) → seconds:

```
beats_per_quarter = 4 / time_note
whole_secs = 4 * beats_per_quarter * (60 / tempo)
real_secs  = d * whole_secs
sounding   = max(0, real_secs * gate)
```

`NoteOn.real_duration = real_secs`; `NoteOff` scheduled at `start + real_secs`. Synth may hold ADS/sustain looping until `NoteOff`, then play release.

---

## 8. Note macros (`param!(macro = ...)`)

Reusable ornament (arp/trill/flourish) triggered by any atom in scope.

```rust
ser!([
  param!(macro = ser!([c4i, e4i, g4i, c5i])),
  c4h,   // plays C-E-G-C 16ths transposed to C, in half-note slot
  g4q,   // same shape transposed to G, in quarter slot
  param!(macro=),
  c5h,   // normal
])
```

* Value must be `Note`. `param!(macro=)` unsets.
* On atom with active macro: compute `offset = trigger_midi - macro_root` (`macro_root` defaults to C4=60; spec amendment allows `param!(macro_root=...)`), `transposed = transpose_note(def, offset)`, generate its events at trigger start with `macro` removed from params.
* Timeline advances by *trigger's* `real_secs`, not fragment length; overflow overlaps siblings (by design).
* `Rest/Tie/PreviousPitch` never trigger. Nested `macro` shadows outer.

---

## 9. Envelopes — the automation sublanguage

Envelopes are first-class values. They only take effect when bound: `param!(volume = env!(...))`, `param!(pitch = cosenv!(...))`, `c4w!(attack_envelope = env!(...))`. Binding sets `Envelope.target` to the key (validated against §9.5).

### 9.1 Constructors / interpolation

| Macro | Interpolation |
|---|---|
| `env!` / `linenv!` | Linear |
| `cosenv!` | Cosine `(1-cos(πt))/2` — smooth, default for musical ADSR |
| `expenv!` | Exponential `prev*(next/prev)^(t²)`; falls back to linear if either end ≤0 |
| `cubenv!` | Cubic smoothstep `t²(3-2t)` — smoothest |

(`en!/linen!/cosen!/expen!/cuben!` are accepted deprecated aliases from CP437.)

### 9.2 Point syntax `<Duration><Value>`

Each comma-separated point is written **duration-first, value-second, no separator**: the parser splits at the first character that can start a float after a valid duration code.

* Metric: `w h q e i t x o` + `d/dd/ddd`. E.g. `q0 e1 h0.5 q0`.
* Absolute: `<num>s` seconds, `<num>ms` milliseconds. E.g. `s0 0.11s1.0 1.0s0.5 0.2s0` (also written `s0, s0.11.0, ...` — the `s` between duration and value is the unit, not a separator).
* Percent: `<num>p` = % of total envelope duration. E.g. `0p0 2p0.8 100p0`.
* Bare number: value with implicit `q` duration (discouraged; be explicit).

First point's duration is ignored (envelope starts there instantly). Single-point env auto-appends a 0.1 s hold at same value.

Examples:

```rust
env!(q0, e1, h0.5, q0)                 // linear ADSR-ish
cosenv!(0p0, 2p0.8)                    // attack: 0→0.8 over first 2%
cosenv!(0s0, 0.05s0.8, 0.1s0.68)       // ms-precise attack
env!(s0, s0.11.0, s1.00.5, s0.20.0)    // ADSR in seconds
param!(duty = cubenv!(q0.2, q0.8))     // PWM sweep on ANY note in scope
param!(pitch = env!(q0, e2, q0))       // +2 semitone blip (fine)
param!(note = env!(q0, q12))           // octave step (rounded)
```

### 9.3 Evaluation

For time `t` since envelope start, with `tempo/time_note` and `total_duration`:

1. Resolve each point to absolute seconds: metric via §7; `s/ms` directly; `p` as `p/100*total`. First point at 0. Sort points `1..` by time (percent may reorder).
2. Clamp: `t ≤ first → first.value`; `t ≥ last → last.value`.
3. Otherwise interpolate per §9.1 within surrounding segment.
4. `total_duration` is the note's `real_secs` for note envelopes, or summed point durations for block envelopes; if missing/≈0, sum non-percent points (warn; percent then inaccurate — caller should pass real total).

Envelopes are evaluated at `env_update_interval` (e.g. every 64 samples) and exponentially smoothed to avoid zipper noise; release forces exact 0 at end.

### 9.4 ADSR phases (synth contract)

* `attack_envelope`: played once from NoteOn; at end → `sustain` (hold last attack value if no sustain env).
* `sustain_envelope`: looped until `NoteOff`.
* `release_envelope` + `release_duration`: played on `NoteOff`; then voice Off. No release env → voice Off immediately (with short fade to avoid click).
* `other_envelopes` (`pitch/duty/note/...`) evaluate over ADS duration (or wall time for indefinite notes).

### 9.5 Targets

Any key in §6 accepting `Number/Env` is a legal target; canonical: `volume velocity pitch note duty gate transpose tempo attack_envelope release_envelope note_volume`. Binding to anything else is a spec error (panic in v0.1, diagnostic in editor future).

---

## 10. Event stream (piano roll)

```rust
TimedMusicalEvent { time_seconds: f32, real_duration: f32, event: MusicalEventType, instrument_name: String }
MusicalEventType::{ NoteOn{note_id,pitch_midi,velocity,parameters,attack_envelope,sustain_envelope,release_envelope,other_envelopes},
  NoteOff{note_id}, Rest{duration_secs}, SetParameter{key,value}, Comment(String) }
```

* `note_id: u64` monotonic (`generate_unique_note_id`). Every `NoteOn` gets a paired `NoteOff` at `start+real_duration`.
* `parameters` on `NoteOn` = scope params minus consumed envelope keys, plus per-note attrs. Includes `tempo/time_note/patch/...` snapshot so synth voices are self-describing.
* `SetParameter` carries scope changes (including `Unset`) for block envelopes and global defaults.
* Ordering: `ser` chronological; `par`-family heap-merged; ties already resolved so stream is flat and playable.
* Infinite: `gen!({ loop { yield_!(c4q); ... } })` + `play_iter`/`note_stream_to_event_stream(Box::new(gen), t0)` never returns; player `take()`s or loops a window.

Display: every `Note` implements `Display` round-tripping to macro syntax (`ser([...])`, `c4q!(volume=0.8)`, `env!(...)`), pretty with `{:#}`. Agents can parse logs back into code.

---

## 11. Instruments + audio stream

```rust
trait Instrument: Send+Sync+'static {
  fn process_event(&mut self, e: &TimedMusicalEvent, rate: usize);
  fn process_events(&mut self, it: Box<dyn Iterator<Item=TimedMusicalEvent>+Send+Sync>, rate: usize);
  fn generate_samples(&mut self, count: usize, rate: usize) -> Vec<[f32;2]>;
  fn is_idle(&self) -> bool { false }
}
```

* **v0.1 `BasicSynth`.** Patches `sine/square/triangle/sawtooth/noise(+white/periodic/brown/pink)`; `duty` PWM; `key` signature (`get_key_signature_adjustment`); `transpose`; `pitch/note/duty/volume/velocity` automation; ADS/R phases per §9.4; LFSR periodic noise with pitch-tracked update period; smoothing + click-free release.
* **FM programming is params.** `Fm4` patches (`fm-lead fm-bell fm-bass fm-pad fm-brass`) are starting points only; every field is overridable per block or per note: `fm_routing` (0-4), `fm_feedback` (0-1), and per operator `op1_ratio … op4_release` (`ratio level attack decay sustain release` each). Example: `c4q!(op2_ratio=1.5, op4_level=0)` retunes and mutes operators on that note alone.
* **SID programming is params.** `Sid` patches (`sid-pulse sid-saw sid-tri`) read `duty`, `cutoff` (Hz), `resonance` (0-1) per note; `cutoff`/`resonance` are envelope targets too.
* Voice model: `ActiveNote{note_id, pitch_midi, phase: Attack/Sustain/Release/Off, envelopes, block_context_envelopes, osc/noise state, cached+smoothed mods}`.
* Mixer: sum all active voices + `static_volume`; future: per-instrument filters/effects stack, named patches (CP437 README TODO, reserved keys `fx`, `filter`, `patch_name`).
* More instruments (separate crates, same trait): `2a03 NES, DMG GB, YM2612, OPL3, SNES wavetable, PC speaker`. Kog decoders (`gme/libvgm/adlmidi/adplug/openmpt/sid/vgmstream`) are reference oracles, not dependencies.

---

## 12. REPL / server (text DAW transport)

CP437 `castle_audio_repl*` (evcxr-based: try `Note`, then `Iterator<Item=Note>`, then raw Rust) evolves into:

* `mmlx-server`: stdin commands `src <tmpfile> <index> [section] | sfxsrc ... | play | stop | reset | loop on|off | preview <ch> <tempo> <token>`; stdout `pos <tick> <t0> <t1> <t2> <t3> | ended | err ...`. Compiles `mmlx-songs` cdylib/JIT or interprets event stream directly; patches playback in place preserving position (lotw `editor/extension.js` protocol, adapted from `song/section/line` to `ser/par` + section index).
* VSCode extension (full lotw port): CodeLens `▶/⏸ ⏹ 🔁` per song fn + per top-level `ser!` section; green highlight of sounding atoms per channel (map server token index → source span via tree-sitter; `env!` = 1 token); debounced live reload; type-to-play preview voice on complete token (`c4e`, `hite`-equivalent `rq`, ...).

---

## 13. Text DAW encoding (v0.1 reserves, v0.2 defines)

Everything below is still `Note` text — no binary project files:

* Sections: top-level `ser!([sectionA, sectionB, ...])` where each is `par!([pulse1_ser, pulse2_ser, tri_ser, noise_ser])` or any voice count (we exceed MIDI 16 by construction; voice = `instrument` param, not channel number).
* Mixer: `param!(instrument="...", volume=..., patch=...)` at section/voice scope; future `param!(bus="...", send=...)`.
* Automation lanes: block `param!(volume = env!(...))` spanning a section.
* Markers: `comment!("verse")` doubles as arrange markers + log lines.

---

## 14. MIDI compat (exceeding MIDI)

Core has no 16-voice / 7-bit limits (`voice_id: u64`, `f32` pitch/velocity, `f64` time planned). A separate `mmlx-midi` crate quantizes: voices → ports × 16 channels (or folds with program change), velocity `0..1 → 0..127`, pitch bend for `pitch` cents, stepped `note` as re-trigger. Import via `midly` is future work; Kog `midly/spessasynth` usage is the reference.

Plugin formats for later (not v0.1 deps): `CLAP` (primary open), `LV2/LADSPA` (Linux), `VST3` (compat via `nih-plug`/wrappers).

---

## 15. Deltas from CP437 (normative for mmlx v0.1)

1. `env!` canonical; `en!` kept as alias (CP437 used both).
2. `linenv!/cosenv!/expenv!/cubenv!` canonical; `linen!/cosen!/expen!/cuben!` aliases.
3. `forkseq!/forkpar!` canonical (`forkser!` alias; CP437 SPEC vs code differed).
4. `time_beat` clarified as display-only; duration math uses `tempo+time_note` only.
5. `macro_root` defaults to C4; future param (CP437 hardcoded 60).
6. `note_volume/adsr` legacy: honored but deprecated in favor of explicit `attack/sustain/release_envelope`.
7. Bare `env!` in block = hard error (CP437 already panics; now spec'd).
8. `f64` time + `f32` pitch-cents reserved; v0.1 still `f32` seconds / `u8` MIDI on the wire for CP437 compat.

---

## 16. Minimal examples

```rust
use mmlx::prelude::*;

// Melody (CP437-compatible)
let melody = ser!([c4q, d4q, e4h, d4q, c4q]);

// Voices exceeding MIDI limits: N parallel instruments, not 16 channels
let band = par!([
  ser!([param!(instrument="2a03_pulse1", patch="square"), c4q, e4q, g4q]),
  ser!([param!(instrument="2a03_pulse2", patch="square", duty=0.25), g4q, g4q]),
  ser!([param!(instrument="triangle", patch="triangle"), c4i, c4i, c5i]),
]);

// Automation on ANY param, including pitch
let swell = ser!([
  param!(tempo=120, patch="sawtooth",
         volume = cosenv!(0p0, 10p1.0, 90p0.8, 100p0),
         pitch  = env!(q0, e0.5, q0)),
  c4w,
]);

// Infinite generator (stable genawaiter)
use genawaiter::sync::gen;
let arp = gen!({
  loop { yield_!(c4q); yield_!(e4q); yield_!(g4q); yield_!(c5q); }
});
// play_iter(arp) streams forever; take(n) for tests.

// Ornament macro
let ornamented = ser!([
  param!(macro = ser!([c4i, e4i, g4i, c5i])),
  c4h, g4q,
  param!(macro=),
  c5h,
]);
```

---

## 17. Conformance + future

* v0.1 conformance: all CP437 `examples_songs` (twinkle, interpolation, sfx) render identical event streams under mmlx.
* Next: `mmlx-core` crate split, `mmlx-server`, `vscode-mmlx` full port (transport+highlight+live+preview), chiptune plugin pack, `mmlx-midi`, AI helpers (continue/harmonize) operating purely on `Note` text.
