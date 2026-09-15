//! Build the vendored GENS YM2612 core (see native/README.md for provenance).
//! Only stdout shows on failure; warnings are the core's own.

fn main() {
    cc::Build::new()
        .file("native/emu/cores/ym2612.c")
        .include("native")
        .include("native/emu/cores")
        .warnings(false)
        .opt_level(2)
        .compile("mmlx_ym2612");
}
