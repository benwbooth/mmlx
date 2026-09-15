# vscode-mmlx

VSCode transport for mmlx songs. Above each `fn name() -> Note` it shows
CodeLens buttons:

- **▶ Play / ⏸ Pause**, **⏹ Stop** (stop rewinds), **🔁 Loop on/off**.
- While playing, the currently sounding note is highlighted; edits reload
  the song (v1 restarts from the top); typing a complete note (`c4e`, `rq`)
  previews it immediately.

## How it works

```
extension.js ──(load <tmp> / play <fn>() / stop / reset / loop, stdin)──▶ mmlx-server
    ▲  pos <tick> <ordinal> / ended / err  ◀────────────────────────────  (evcxr eval + virtual clock)
```

The ordinal counts NoteOns so far (1-based); the extension highlights the
Nth atom-like element in the function. Approximate after rests (rests emit
no NoteOn).

## Setup (development install)

```sh
cd vscode-mmlx && npm install
ln -s "$PWD" ~/.vscode/extensions/mmlx
```

Open the mmlx workspace as the folder (the server runs
`cargo run -p mmlx-server` from the workspace root). Set `mmlx.audio: true`
for live sound (needs ALSA headers — provided by the nix flake env).

## Limitations (v1)

- Live reload restarts from the top (no position-preserving re-tick yet).
- No per-section lenses yet (section source often references fn locals).
- Ordinal highlight drifts after rests and inside dense `par!` chords.
