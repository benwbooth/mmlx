//! Build the vendored Nuked-OPN2 YM2612 core (see native/README.md).
//! Only stdout shows on failure; warnings are the core's own.

fn main() {
    cc::Build::new()
        .cpp(true)
        .file("native/nuked/Ym2612_Nuked.cpp")
        .file("native/nuked_shim.cpp")
        .include("native/nuked")
        .define("VGM_YM2612_NUKED", None)
        .warnings(false)
        .opt_level(2)
        .compile("mmlx_ym_nuked");
}
