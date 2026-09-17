# Vendored YM2612 cores — provenance and license

## GENS core (`emu/cores/ym2612.{c,h}`, `ym2612_int.h`, `emu/snddef.h`,
## `stdtype.h`, `common_def.h`)

From **libvgm** (Valley Bell), which embeds the GENS YM2612 core by
Stéphane Dallongeville, copied from Kog's `native/libvgm` vendor tree
solely to avoid a cross-repo build dependency. Legacy reference only;
the voice now runs Nuked (below).

## Nuked-OPN2 core (`nuked/Ym2612_Nuked.{h,cpp}`, `nuked_shim.cpp`)

Game Music Emu's Nuked-OPN2 YM2612 backend (by Nuke.YKT, via the GME
project), copied from Kog's `native/game-music-emu` vendor tree.
Cycle-accurate; resamples internally to any rate; this is the same DSP
our GME audit reference renders, so per-lane A/B compares like with
like. `nuked_shim.cpp` exposes it over a C ABI (factory, rate, reset,
port writes, stereo run).

libvgm as a whole is distributed under the **GNU GPL version 2 or later**;
Game Music Emu's GENS and Nuked backends are **LGPL v2.1+** (MAME backend
would be GPL and is NOT vendored). This crate (and therefore the mmlx
workspace) follows GPL — see the workspace `LICENSE.md`. No other mmlx
crate contains third-party code.
