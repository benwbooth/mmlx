# mmlx — master checklist

Standing rules: `cargo build` + `cargo test` + `cargo fmt --check` green,
then commit + push after every feature/fix.

## Phase 0 — core DSL ✅ DONE (`45d8377`, `a803683`)

- [x] `mmlx-macros`: note/rest/tie/`p` symbol generation
- [x] `mmlx-core`: Note/params/envelopes/ser-par event streams, canonical aliases
- [x] `mmlx-songs`: `all_features()` conformance song + tests
- [x] `MMLX_SPEC.md` v0.1, `MMLX_OPUS_PORT.md`, README
- [x] Public repo `github.com/benwbooth/mmlx`, pushed

## Phase 1 — synth + audio + REPL (IN PROGRESS)

- [x] `mmlx-synth`: BasicSynth port + render test
- [x] `mmlx-audio`: `EventQueue`/`SynthTime`/`InstrumentsMap`, `queue_note`
  (headless-safe + test); cpal `setup_audio` backend behind `audio` feature
  (this env has no ALSA headers — live sound unverified here, verify on an
  audio machine with `cargo run -p mmlx-repl --features audio`)
- [x] `mmlx-repl`: evcxr `ReplEnv` (Note → queue, Iterator → drain-queue,
  raw Rust fallback; `:dep` via absolute crate path) + rustyline/syntect
  `mmlx-repl` binary. Verified end-to-end via
  `cargo run -p mmlx-repl --example repl_smoke` (SMOKE PASS).
  Known limit: evcxr round-trip can't run under `cargo test` (worker
  re-exec breaks under libtest) — covered by the smoke example instead.

## Phase 2 — server + VSCode extension

- [x] `mmlx-server`: stdin line protocol
  (`load <path> / play <expr> / stop / reset / loop on|off /
  preview <expr>`), stdout
  (`ok / err / pos <tick> <ordinal> / ended`), virtual clock (headless
  test via `examples/server_smoke.rs`; live audio with `audio` feature)
- [x] `vscode-mmlx`: port of lotw `editor/extension.js` — CodeLens
  transport per `fn() -> Note`, ordinal highlight, debounced
  reload (restarts), type-to-play preview (`node --check` clean)

## Phase 3 — `mmlx-algo` (Opusmodus port, per MMLX_OPUS_PORT.md)

- [ ] P1 deterministic: retrograde/rotate/divide/dedup/transpose_all/invert/
      ambitus/scale/chordize/dechord/demix/mix/augment/articulate/cresc/
      zip-lanes/to-bars
- [ ] P2 seeded random: sample/shuffle/walk/markov/euclidean (explicit `seed`)
- [ ] P3 systems: pcset/twelve-tone/tonality/schillinger/messiaen/timepoint/
      tendency/canon
- [ ] P4 esoteric: lindenmayer/cellular/infinity-series/waveform/spectral
- [ ] Each fn: doc example + property test + demo song where musical

## Phase 4 — plugins + MIDI

- [x] Plugin format decision: **CLAP primary** (`clack`), LV2 second
- [x] Effect plugin 1: `mmlx-fx` graphic EQ (peaking biquads, flat/boost/cut tests)
- [x] `mmlx-midi`: SMF export + import + multi-port (per-instrument tracks),
  all with round-trip tests
- [x] CLAP scaffold: `mmlx-clap` cdylib (Clack gain plugin, descriptor test,
  real `libmmlx_clap.so`); full instrument-param export next
- [ ] Instrument plugin: SF2 wavetable (RustySynth, cf. Kog `decoder.rs`)
- [x] Instrument plugin: `mmlx-sf2` (RustySynth + in-memory bank, render test)
- [x] Chiptune voices: `mmlx-chip` NES 2A03 + GB DMG (pure Rust, patch tests)
- [x] CLAP export: `mmlx-clap` cdylib with `gain_db` params extension
- [ ] FM voices (YM2612/OPL3), SID filter, tracker wrappers (need C++ or big DSP)

## Phase 5 — AI text DAW

- [x] Mixer text encoding: `pan` (BasicSynth equal-power), `bus`/`send`
  keys reserved; automation via envelopes on any target
- [x] `mmlx-fx::BusMixer` mix stage (gain/master math test)
- [x] Agent helpers: `humanize` (seeded velocity/gate jitter),
  `harmonize` (interval stacks), `continue_melody` (Markov), all tested
- [x] Mixer bus rendering in backend (per-bus buffers into BusMixer,
  `param!(bus)` live routing; compiles under nix audio env)
- [x] Agent helpers: remix + piano-roll text view in `mmlx-algo`
- [ ] Piano-roll graphical view in vscode-mmlx (never source of truth)
