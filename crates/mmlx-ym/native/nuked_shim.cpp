// C ABI shim over GME's Nuked-OPN2 YM2612 core (Ym2612_Nuked.{h,cpp}).
// Keeps C++ linkage inside the static lib; Rust talks to these symbols.
#include "Ym2612_Nuked.h"
#include <new>

extern "C" {

void *mmlx_nuked_new(void) {
    return new (std::nothrow) Ym2612_Nuked_Emu();
}

void mmlx_nuked_free(void *chip) {
    delete static_cast<Ym2612_Nuked_Emu *>(chip);
}

// Returns 0 on success, nonzero on error (message ignored; null-checked).
int mmlx_nuked_set_rate(void *chip, double sample_rate, double clock_rate) {
    const char *err =
        static_cast<Ym2612_Nuked_Emu *>(chip)->set_rate(sample_rate, clock_rate);
    return err ? 1 : 0;
}

void mmlx_nuked_reset(void *chip) {
    static_cast<Ym2612_Nuked_Emu *>(chip)->reset();
}

void mmlx_nuked_write(void *chip, int port, int addr, int data) {
    if (port == 0)
        static_cast<Ym2612_Nuked_Emu *>(chip)->write0(addr, data);
    else
        static_cast<Ym2612_Nuked_Emu *>(chip)->write1(addr, data);
}

void mmlx_nuked_mute(void *chip, int mask) {
    static_cast<Ym2612_Nuked_Emu *>(chip)->mute_voices(mask);
}

// Renders `frames` stereo int16 pairs, zeroing the buffer first
// (the core mixes additively into it).
void mmlx_nuked_run(void *chip, int frames, short *out_stereo) {
    for (int i = 0; i < frames * 2; i++) out_stereo[i] = 0;
    static_cast<Ym2612_Nuked_Emu *>(chip)->run(frames, out_stereo);
}

} // extern "C"
