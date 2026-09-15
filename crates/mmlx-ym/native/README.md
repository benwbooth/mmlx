# Vendored YM2612 core — provenance and license

`emu/cores/ym2612.{c,h}`, `ym2612_int.h`, `emu/snddef.h`, `stdtype.h`,
`common_def.h` are from **libvgm** (Valley Bell), which embeds the GENS
YM2612 core by Stéphane Dallongeville, copied from Kog's
`native/libvgm` vendor tree solely to avoid a cross-repo build dependency.

libvgm as a whole is distributed under the **GNU GPL version 2 or later**;
this crate (and therefore the mmlx workspace) follows suit — see the
workspace `LICENSE.md`. No other mmlx crate contains third-party code.
