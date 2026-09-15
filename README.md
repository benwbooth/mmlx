# mmlx — Music Macro Language Extended

A Rust-embedded music composition DSL and text-first DAW core.
Ported from CP437 Castle Audio, with live-edit tooling ideas from lotw.

```rust
use mmlx_core::prelude::*;

let melody = ser!([c4q, d4q, e4h, d4q, c4q]);
let band = par!([
    ser!([param!(patch = "square"), c4q, e4q, g4q]),
    ser!([param!(patch = "triangle"), c3h, g3h]),
]);
```

## Layout

- `MMLX_SPEC.md` — full language spec (v0.1 draft)
- `MMLX_OPUS_PORT.md` — Opusmodus function port plan
- `crates/mmlx-macros` — proc macros generating note/rest/tie/`p` consts
- `crates/mmlx-core` — `Note`, params, envelopes, `ser/par/parmin/fork`
  composition, `genawaiter` event-stream (piano roll) generation
- `crates/mmlx-songs` — `all_features()` conformance song + tests
- `crates/mmlx-synth` — `BasicSynth` instrument + render test
- `crates/mmlx-audio` — playback queue; cpal backend (`audio` feature)
- `crates/mmlx-repl` — evcxr REPL library + `mmlx-repl` binary

## Build / test

```sh
cargo build
cargo test
cargo run -p mmlx-repl --example repl_smoke  # evcxr end-to-end
cargo run -p mmlx-repl                       # interactive (needs audio machine + --features audio for sound)
cargo fmt --check
```
