//! `mmlx-track`: tracker/chip-log playback via system libraries.
//!
//! [`render_gme`] drives Game Music Emu (NSF/AY/SPC/…) and [`render_mod`]
//! drives libopenmpt (MOD/S3M/XM/IT/…), both `dlopen`ed at runtime so the
//! workspace builds and tests without them. Libraries resolve from
//! `MMLX_GME_LIB` / `MMLX_OPENMPT_LIB`, else the bare sonames.
//! [`nsf_fixture`] / [`mod_fixture`] build tiny audible songs in memory.

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_long, c_void};
#[derive(Debug)]
pub enum TrackError {
    MissingLibrary(String),
    MissingSymbol(String),
    Backend(String),
}

impl std::fmt::Display for TrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackError::MissingLibrary(lib) => write!(f, "library not found: {lib}"),
            TrackError::MissingSymbol(symbol) => write!(f, "symbol not found: {symbol}"),
            TrackError::Backend(message) => write!(f, "backend error: {message}"),
        }
    }
}

impl std::error::Error for TrackError {}

fn open(env: &str, soname: &str) -> Result<Library, TrackError> {
    let candidate = std::env::var(env).unwrap_or_else(|_| soname.to_string());
    // RTLD_NOW: resolve every relocation at open time so a broken library
    // fails here with an error instead of segfaulting on first call.
    // (Arity mismatches are still only caught by the render tests.)
    // SAFETY: flags are valid dlopen constants; the handle is closed on drop.
    unsafe {
        libloading::os::unix::Library::open(Some(&candidate), libc::RTLD_NOW | libc::RTLD_LOCAL)
            .map(Library::from)
            .map_err(|_| TrackError::MissingLibrary(candidate))
    }
}

fn gme_check(err: *const c_char) -> Result<(), TrackError> {
    if err.is_null() {
        Ok(())
    } else {
        Err(TrackError::Backend(
            unsafe { CStr::from_ptr(err) }
                .to_string_lossy()
                .into_owned(),
        ))
    }
}

// --- Game Music Emu ---

struct Gme {
    lib: Library,
}

impl Gme {
    fn load() -> Result<Self, TrackError> {
        Ok(Gme {
            lib: open("MMLX_GME_LIB", "libgme.so.0")?,
        })
    }

    unsafe fn emu_type(&self, symbol: &[u8]) -> Result<*const c_void, TrackError> {
        let identify: Symbol<unsafe extern "C" fn(*const c_char) -> *const c_void> = self
            .lib
            .get(b"gme_identify_extension")
            .map_err(|_| TrackError::MissingSymbol("gme_identify_extension".into()))?;
        let name = CString::new(symbol).map_err(|_| TrackError::Backend("bad emu name".into()))?;
        let emu_type = identify(name.as_ptr());
        if emu_type.is_null() {
            return Err(TrackError::Backend(format!(
                "unknown GME extension: {}",
                String::from_utf8_lossy(symbol)
            )));
        }
        Ok(emu_type)
    }

    /// Render `seconds` of `track` from a chip-log image of `emu` type.
    /// `emu` is a file extension like `"nsf"` (see `gme_identify_extension`).
    fn render(
        &self,
        emu_extension: &str,
        data: &[u8],
        track: i32,
        sample_rate: u32,
        seconds: f32,
    ) -> Result<Vec<[f32; 2]>, TrackError> {
        unsafe {
            let emu_type = self.emu_type(emu_extension.as_bytes())?;
            let new_emu: Symbol<unsafe extern "C" fn(*const c_void, c_int) -> *mut c_void> = self
                .lib
                .get(b"gme_new_emu")
                .map_err(|_| TrackError::MissingSymbol("gme_new_emu".into()))?;
            let emu = new_emu(emu_type, sample_rate as c_int);
            if emu.is_null() {
                return Err(TrackError::Backend("gme_new_emu failed".into()));
            }
            let load: Symbol<
                unsafe extern "C" fn(*mut c_void, *const c_void, c_long) -> *const c_char,
            > = self
                .lib
                .get(b"gme_load_data")
                .map_err(|_| TrackError::MissingSymbol("gme_load_data".into()))?;
            gme_check(load(
                emu,
                data.as_ptr() as *const c_void,
                data.len() as c_long,
            ))?;
            let start: Symbol<unsafe extern "C" fn(*mut c_void, c_int) -> *const c_char> = self
                .lib
                .get(b"gme_start_track")
                .map_err(|_| TrackError::MissingSymbol("gme_start_track".into()))?;
            gme_check(start(emu, track as c_int))?;
            let play: Symbol<unsafe extern "C" fn(*mut c_void, c_int, *mut i16) -> *const c_char> =
                self.lib
                    .get(b"gme_play")
                    .map_err(|_| TrackError::MissingSymbol("gme_play".into()))?;
            let frames = (seconds * sample_rate as f32) as usize;
            let mut pcm = vec![0i16; frames * 2];
            gme_check(play(emu, (frames * 2) as c_int, pcm.as_mut_ptr()))?;
            let delete: Symbol<unsafe extern "C" fn(*mut c_void)> = self
                .lib
                .get(b"gme_delete")
                .map_err(|_| TrackError::MissingSymbol("gme_delete".into()))?;
            delete(emu);
            Ok(pcm
                .chunks_exact(2)
                .map(|pair| [pair[0] as f32 / 32768.0, pair[1] as f32 / 32768.0])
                .collect())
        }
    }
}

/// Render a GME chip log (`emu_extension` like `"nsf"`, `"ay"`, `"spc"`)
/// to stereo float.
pub fn render_gme(
    emu_extension: &str,
    data: &[u8],
    track: i32,
    sample_rate: u32,
    seconds: f32,
) -> Result<Vec<[f32; 2]>, TrackError> {
    Gme::load()?.render(emu_extension, data, track, sample_rate, seconds)
}

// --- libopenmpt ---

/// Call `openmpt_module_create_from_memory2` with the full 9-argument form
/// (0.8 added errfunc/error/error_message before ctls).
#[allow(clippy::too_many_arguments)]
unsafe fn create_module(
    create: &Symbol<
        unsafe extern "C" fn(
            *const c_void,
            usize,
            *const c_void,
            *const c_void,
            *const c_void,
            *const c_void,
            *mut c_int,
            *mut *const c_char,
            *const c_void,
        ) -> *mut c_void,
    >,
    data: &[u8],
) -> Result<*mut c_void, TrackError> {
    let mut error_code: c_int = 0;
    let mut error_message: *const c_char = std::ptr::null();
    let module = create(
        data.as_ptr() as *const c_void,
        data.len(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        &mut error_code,
        &mut error_message,
        std::ptr::null(),
    );
    if module.is_null() {
        let detail = if error_message.is_null() {
            String::new()
        } else {
            CStr::from_ptr(error_message).to_string_lossy().into_owned()
        };
        return Err(TrackError::Backend(format!(
            "openmpt rejected the module (code {error_code}): {detail}"
        )));
    }
    Ok(module)
}

struct OpenMpt {
    lib: Library,
}

impl OpenMpt {
    fn load() -> Result<Self, TrackError> {
        Ok(OpenMpt {
            lib: open("MMLX_OPENMPT_LIB", "libopenmpt.so.0")?,
        })
    }

    fn render(
        &self,
        data: &[u8],
        sample_rate: u32,
        seconds: f32,
    ) -> Result<Vec<[f32; 2]>, TrackError> {
        unsafe {
            let create: Symbol<
                unsafe extern "C" fn(
                    *const c_void,
                    usize,
                    *const c_void,
                    *const c_void,
                    *const c_void,
                    *const c_void,
                    *mut c_int,
                    *mut *const c_char,
                    *const c_void,
                ) -> *mut c_void,
            > = self
                .lib
                .get(b"openmpt_module_create_from_memory2")
                .map_err(|_| {
                    TrackError::MissingSymbol("openmpt_module_create_from_memory2".into())
                })?;
            let module = create_module(&create, data)?;
            if module.is_null() {
                return Err(TrackError::Backend("openmpt rejected the module".into()));
            }
            let read: Symbol<
                unsafe extern "C" fn(*mut c_void, i32, usize, *mut f32, *mut f32) -> usize,
            > = self
                .lib
                .get(b"openmpt_module_read_float_stereo")
                .map_err(|_| {
                    TrackError::MissingSymbol("openmpt_module_read_float_stereo".into())
                })?;
            let frames = (seconds * sample_rate as f32) as usize;
            let mut left = vec![0.0f32; frames];
            let mut right = vec![0.0f32; frames];
            let mut done = 0usize;
            while done < frames {
                let got = read(
                    module,
                    sample_rate as i32,
                    frames - done,
                    left[done..].as_mut_ptr(),
                    right[done..].as_mut_ptr(),
                );
                if got == 0 {
                    break;
                }
                done += got;
            }
            left.truncate(done);
            right.truncate(done);
            let destroy: Symbol<unsafe extern "C" fn(*mut c_void)> = self
                .lib
                .get(b"openmpt_module_destroy")
                .map_err(|_| TrackError::MissingSymbol("openmpt_module_destroy".into()))?;
            destroy(module);
            Ok(left.into_iter().zip(right).map(|(l, r)| [l, r]).collect())
        }
    }

    fn duration(&self, data: &[u8]) -> Result<f64, TrackError> {
        unsafe {
            let create: Symbol<
                unsafe extern "C" fn(
                    *const c_void,
                    usize,
                    *const c_void,
                    *const c_void,
                    *const c_void,
                    *const c_void,
                    *mut c_int,
                    *mut *const c_char,
                    *const c_void,
                ) -> *mut c_void,
            > = self
                .lib
                .get(b"openmpt_module_create_from_memory2")
                .map_err(|_| {
                    TrackError::MissingSymbol("openmpt_module_create_from_memory2".into())
                })?;
            let module = create_module(&create, data)?;
            if module.is_null() {
                return Err(TrackError::Backend("openmpt rejected the module".into()));
            }
            let get_duration: Symbol<unsafe extern "C" fn(*mut c_void) -> c_double> = self
                .lib
                .get(b"openmpt_module_get_duration_seconds")
                .map_err(|_| {
                    TrackError::MissingSymbol("openmpt_module_get_duration_seconds".into())
                })?;
            let seconds = get_duration(module);
            let destroy: Symbol<unsafe extern "C" fn(*mut c_void)> = self
                .lib
                .get(b"openmpt_module_destroy")
                .map_err(|_| TrackError::MissingSymbol("openmpt_module_destroy".into()))?;
            destroy(module);
            Ok(seconds)
        }
    }
}

/// Render a tracker module (MOD/S3M/XM/IT/…) to stereo float.
pub fn render_mod(
    data: &[u8],
    sample_rate: u32,
    seconds: f32,
) -> Result<Vec<[f32; 2]>, TrackError> {
    OpenMpt::load()?.render(data, sample_rate, seconds)
}

/// Total song duration in seconds, for UI/loop planning.
pub fn mod_duration(data: &[u8]) -> Result<f64, TrackError> {
    OpenMpt::load()?.duration(data)
}

// --- fixtures: tiny audible songs built in memory ---

/// Minimal NSF: init routine holds pulse1 at A440 (50% duty, length halted),
/// play routine is RTS. Renders a constant square tone.
pub fn nsf_fixture() -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[0..5].copy_from_slice(b"NESM\x1A");
    header[5] = 1; // version
    header[6] = 1; // one song
    header[7] = 1; // starts at song 1
    header[8..10].copy_from_slice(&0x8000u16.to_le_bytes()); // load
    header[10..12].copy_from_slice(&0x8000u16.to_le_bytes()); // init
    header[12..14].copy_from_slice(&0x8015u16.to_le_bytes()); // play
    header[14..46].copy_from_slice(b"mmlx square test tune\0\0\0\0\0\0\0\0\0\0\0");
    header[46..78].copy_from_slice(b"mmlx\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    header[78..110].copy_from_slice(b"(c) mmlx test\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    header[110..112].copy_from_slice(&16639u16.to_le_bytes()); // NTSC speed
                                                               // bankswitch[8] stays 0; PAL speed:
    header[120..122].copy_from_slice(&19997u16.to_le_bytes());
    header[122] = 0x01; // NTSC
                        // soundchip byte 123 stays 0 (plain 2A03).
    let code: &[u8] = &[
        0xA9, 0xBF, // LDA #$BF (50% duty, halt+const vol, vol 15)
        0x8D, 0x00, 0x40, // STA $4000
        0xA9, 0x08, // LDA #$08 (sweep off)
        0x8D, 0x01, 0x40, // STA $4001
        0xA9, 0xFD, // LDA #$FD (period lo: A440)
        0x8D, 0x02, 0x40, // STA $4002
        0xA9, 0x00, // LDA #$00
        0x8D, 0x03, 0x40, // STA $4003
        0x60, // RTS (init ends at $8011)
        0x60, // RTS (play routine)
    ];
    header.extend_from_slice(code);
    header
}

/// Minimal ProTracker MOD: one 64-byte sine sample looping, row 0 plays C-3.
pub fn mod_fixture() -> Vec<u8> {
    let mut data = vec![0u8; 1084];
    data[0..15].copy_from_slice(b"mmlx sine test\0");
    // Sample 1 header at offset 20: name, len 32 words, vol 64, loop all.
    data[20..26].copy_from_slice(b"sine  ");
    data[42..44].copy_from_slice(&32u16.to_be_bytes()); // length words
    data[44] = 0; // finetune
    data[45] = 64; // volume
    data[46..48].copy_from_slice(&0u16.to_be_bytes()); // loop start
    data[48..50].copy_from_slice(&32u16.to_be_bytes()); // loop length
    data[950] = 1; // song length: 1 pattern
    data[951] = 0; // restart
                   // pattern table already zeros -> pattern 0.
    data[1080..1084].copy_from_slice(b"M.K.");
    // Pattern 0 (1024 bytes): row 0, channel 0 = sample 1, period 856 (C-3).
    let mut pattern = vec![0u8; 1024];
    pattern[0] = 0x03;
    pattern[1] = 0x58;
    pattern[2] = 0x10;
    pattern[3] = 0x00;
    data.extend_from_slice(&pattern);
    // Sample data: 64-byte looping sine at +/-100.
    for i in 0..64 {
        let sample = ((i as f32 / 64.0 * std::f32::consts::TAU).sin() * 100.0) as i8;
        data.push(sample as u8);
    }
    data
}

// --- streaming backends: piano-roll events in, engine audio out ---

use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
use std::collections::HashMap;

/// FluidSynth realtime voice: `NoteOn`/`NoteOff` become channel-0 MIDI, rendered
/// with `write_float`. Select the bank's preset with `program` (default 0).
pub struct FluidVoice {
    lib: Library,
    synth: *mut c_void,
    active: HashMap<u64, i32>,
}

// FluidSynth calls here are single-threaded from the render path.
unsafe impl Send for FluidVoice {}
unsafe impl Sync for FluidVoice {}

type FluidSettings = *mut c_void;
type FluidSynthHandle = *mut c_void;

impl FluidVoice {
    /// Open the bank at `soundfont_path` for `sample_rate` output.
    pub fn open(soundfont_path: &str, sample_rate: u32) -> Result<Self, TrackError> {
        unsafe {
            let lib = open("MMLX_FLUID_LIB", "libfluidsynth.so.3")?;
            let new_settings: Symbol<unsafe extern "C" fn() -> FluidSettings> = lib
                .get(b"new_fluid_settings")
                .map_err(|_| TrackError::MissingSymbol("new_fluid_settings".into()))?;
            let setnum: Symbol<
                unsafe extern "C" fn(FluidSettings, *const c_char, c_double) -> c_int,
            > = lib
                .get(b"fluid_settings_setnum")
                .map_err(|_| TrackError::MissingSymbol("fluid_settings_setnum".into()))?;
            let settings = new_settings();
            if settings.is_null() {
                return Err(TrackError::Backend("new_fluid_settings failed".into()));
            }
            let rate_name = CString::new("synth.sample-rate").unwrap();
            setnum(settings, rate_name.as_ptr(), sample_rate as c_double);
            let new_synth: Symbol<unsafe extern "C" fn(FluidSettings) -> FluidSynthHandle> = lib
                .get(b"new_fluid_synth")
                .map_err(|_| TrackError::MissingSymbol("new_fluid_synth".into()))?;
            let synth = new_synth(settings);
            if synth.is_null() {
                return Err(TrackError::Backend("new_fluid_synth failed".into()));
            }
            let sfload: Symbol<
                unsafe extern "C" fn(FluidSynthHandle, *const c_char, c_int) -> c_int,
            > = lib
                .get(b"fluid_synth_sfload")
                .map_err(|_| TrackError::MissingSymbol("fluid_synth_sfload".into()))?;
            let path = CString::new(soundfont_path)
                .map_err(|_| TrackError::Backend("bank path contains NUL".into()))?;
            if sfload(synth, path.as_ptr(), 1) < 0 {
                return Err(TrackError::Backend(format!(
                    "sfload failed: {soundfont_path}"
                )));
            }
            Ok(FluidVoice {
                lib,
                synth,
                active: HashMap::new(),
            })
        }
    }

    fn call_note(&self, on: bool, key: i32, velocity: i32) -> Result<(), TrackError> {
        unsafe {
            if on {
                let noteon: Symbol<
                    unsafe extern "C" fn(FluidSynthHandle, c_int, c_int, c_int) -> c_int,
                > = self
                    .lib
                    .get(b"fluid_synth_noteon")
                    .map_err(|_| TrackError::MissingSymbol("fluid_synth_noteon".into()))?;
                if noteon(self.synth, 0, key as c_int, velocity as c_int) != 0 {
                    return Err(TrackError::Backend("noteon failed".into()));
                }
            } else {
                let noteoff: Symbol<unsafe extern "C" fn(FluidSynthHandle, c_int, c_int) -> c_int> =
                    self.lib
                        .get(b"fluid_synth_noteoff")
                        .map_err(|_| TrackError::MissingSymbol("fluid_synth_noteoff".into()))?;
                if noteoff(self.synth, 0, key as c_int) != 0 {
                    return Err(TrackError::Backend("noteoff failed".into()));
                }
            }
            Ok(())
        }
    }

    fn render_frames(&mut self, frames: usize) -> Result<Vec<[f32; 2]>, TrackError> {
        unsafe {
            let write: Symbol<
                unsafe extern "C" fn(
                    FluidSynthHandle,
                    c_int,
                    *mut c_void,
                    c_int,
                    c_int,
                    *mut c_void,
                    c_int,
                    c_int,
                ) -> c_int,
            > = self
                .lib
                .get(b"fluid_synth_write_float")
                .map_err(|_| TrackError::MissingSymbol("fluid_synth_write_float".into()))?;
            let mut left = vec![0.0f32; frames];
            let mut right = vec![0.0f32; frames];
            if write(
                self.synth,
                frames as c_int,
                left.as_mut_ptr() as *mut c_void,
                0,
                1,
                right.as_mut_ptr() as *mut c_void,
                0,
                1,
            ) != 0
            {
                return Err(TrackError::Backend("write_float failed".into()));
            }
            Ok(left.into_iter().zip(right).map(|(l, r)| [l, r]).collect())
        }
    }
}

impl Drop for FluidVoice {
    fn drop(&mut self) {
        unsafe {
            if let Ok(delete) = self
                .lib
                .get::<unsafe extern "C" fn(FluidSynthHandle)>(b"delete_fluid_synth")
            {
                delete(self.synth);
            }
        }
    }
}

impl Instrument for FluidVoice {
    fn process_event(&mut self, event: &TimedMusicalEvent, _rate: usize) {
        match &event.event {
            MusicalEventType::NoteOn {
                note_id,
                pitch_midi,
                velocity,
                ..
            } => {
                let velocity = (*velocity * 127.0).round().clamp(1.0, 127.0) as i32;
                if self.call_note(true, *pitch_midi as i32, velocity).is_ok() {
                    self.active.insert(*note_id, *pitch_midi as i32);
                }
            }
            MusicalEventType::NoteOff { note_id } => {
                if let Some(key) = self.active.remove(note_id) {
                    let _ = self.call_note(false, key, 0);
                }
            }
            _ => {}
        }
    }

    fn process_events(
        &mut self,
        events: Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync>,
        rate: usize,
    ) {
        for event in events {
            self.process_event(&event, rate);
        }
    }

    fn generate_samples(&mut self, count: usize, _rate: usize) -> Vec<[f32; 2]> {
        // FluidSynth voices decay on their own; a render failure yields silence.
        self.render_frames(count)
            .unwrap_or_else(|_| vec![[0.0; 2]; count])
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}

// --- Munt MT-32 realtime voice ---

// Report-handler v0: 15 no-op callbacks (signatures mirror c_types.h).
extern "C" fn munt_version(_handler: *const c_void) -> u32 {
    0
}
extern "C" fn munt_debug(_data: *mut c_void, _fmt: *const c_char) {}
extern "C" fn munt_void(_data: *mut c_void) {}
extern "C" fn munt_str(_data: *mut c_void, _message: *const c_char) {}
extern "C" fn munt_queue_overflow(_data: *mut c_void) -> u32 {
    0
}
extern "C" fn munt_realtime(_data: *mut c_void, _message: u8) {}
extern "C" fn munt_reverb(_data: *mut c_void, _mode: u8) {}
extern "C" fn munt_poly(_data: *mut c_void, _part: u8) {}
extern "C" fn munt_program(
    _data: *mut c_void,
    _part: u8,
    _group: *const c_char,
    _patch: *const c_char,
) {
}

#[repr(C)]
struct MuntHandlerV0 {
    get_version: extern "C" fn(*const c_void) -> u32,
    print_debug: extern "C" fn(*mut c_void, *const c_char),
    on_error_control: extern "C" fn(*mut c_void),
    on_error_pcm: extern "C" fn(*mut c_void),
    show_lcd: extern "C" fn(*mut c_void, *const c_char),
    on_midi_played: extern "C" fn(*mut c_void),
    on_queue_overflow: extern "C" fn(*mut c_void) -> u32,
    on_realtime: extern "C" fn(*mut c_void, u8),
    on_reset: extern "C" fn(*mut c_void),
    on_reconfig: extern "C" fn(*mut c_void),
    on_reverb_mode: extern "C" fn(*mut c_void, u8),
    on_reverb_time: extern "C" fn(*mut c_void, u8),
    on_reverb_level: extern "C" fn(*mut c_void, u8),
    on_poly: extern "C" fn(*mut c_void, u8),
    on_program: extern "C" fn(*mut c_void, u8, *const c_char, *const c_char),
}

static MUNT_HANDLER: MuntHandlerV0 = MuntHandlerV0 {
    get_version: munt_version,
    print_debug: munt_debug,
    on_error_control: munt_void,
    on_error_pcm: munt_void,
    show_lcd: munt_str,
    on_midi_played: munt_void,
    on_queue_overflow: munt_queue_overflow,
    on_realtime: munt_realtime,
    on_reset: munt_void,
    on_reconfig: munt_void,
    on_reverb_mode: munt_reverb,
    on_reverb_time: munt_reverb,
    on_reverb_level: munt_reverb,
    on_poly: munt_poly,
    on_program: munt_program,
};

const MT32_RC_OK: i32 = 0;
const MT32_RT_FLOAT: u32 = 1;
const MT32_AOM_COARSE: u32 = 1;

/// Roland MT-32 realtime voice: LA synthesis behind a MIDI short-message
/// stream. Needs control + PCM ROMs (`rom_dir`); without them [`open`]
/// returns [`TrackError::Backend`].
pub struct Mt32Voice {
    lib: Library,
    context: *mut c_void,
    active: HashMap<u64, i32>,
}

// Munt calls here are single-threaded from the render path.
unsafe impl Send for Mt32Voice {}
unsafe impl Sync for Mt32Voice {}

impl Mt32Voice {
    /// Open ROMs from every file in `rom_dir` at `sample_rate` output.
    pub fn open(rom_dir: &str, sample_rate: u32) -> Result<Self, TrackError> {
        unsafe {
            let lib = open("MMLX_MT32_LIB", "libmt32emu.so.2")?;
            let create: Symbol<unsafe extern "C" fn(*const c_void, *mut c_void) -> *mut c_void> =
                lib.get(b"mt32emu_create_context")
                    .map_err(|_| TrackError::MissingSymbol("mt32emu_create_context".into()))?;
            let context = create(
                &MUNT_HANDLER as *const MuntHandlerV0 as *const c_void,
                std::ptr::null_mut(),
            );
            if context.is_null() {
                return Err(TrackError::Backend("create_context failed".into()));
            }
            let add_rom: Symbol<unsafe extern "C" fn(*mut c_void, *const c_char) -> i32> = lib
                .get(b"mt32emu_add_rom_file")
                .map_err(|_| TrackError::MissingSymbol("mt32emu_add_rom_file".into()))?;
            let entries = std::fs::read_dir(rom_dir)
                .map_err(|err| TrackError::Backend(format!("ROM dir {rom_dir}: {err}")))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(name) = CString::new(path.to_string_lossy().into_owned()) {
                        // Unrecognized files report an error code; ROM pairing
                        // is validated by open_synth below.
                        let _ = add_rom(context, name.as_ptr());
                    }
                }
            }
            let select_renderer: Symbol<unsafe extern "C" fn(*mut c_void, u32)> = lib
                .get(b"mt32emu_select_renderer_type")
                .map_err(|_| TrackError::MissingSymbol("mt32emu_select_renderer_type".into()))?;
            select_renderer(context, MT32_RT_FLOAT);
            let set_analog: Symbol<unsafe extern "C" fn(*mut c_void, u32)> = lib
                .get(b"mt32emu_set_analog_output_mode")
                .map_err(|_| TrackError::MissingSymbol("mt32emu_set_analog_output_mode".into()))?;
            set_analog(context, MT32_AOM_COARSE);
            let set_rate: Symbol<unsafe extern "C" fn(*mut c_void, f64)> = lib
                .get(b"mt32emu_set_stereo_output_samplerate")
                .map_err(|_| {
                    TrackError::MissingSymbol("mt32emu_set_stereo_output_samplerate".into())
                })?;
            set_rate(context, sample_rate as f64);
            let open_synth: Symbol<unsafe extern "C" fn(*mut c_void) -> i32> = lib
                .get(b"mt32emu_open_synth")
                .map_err(|_| TrackError::MissingSymbol("mt32emu_open_synth".into()))?;
            let code = open_synth(context);
            if code != MT32_RC_OK {
                return Err(TrackError::Backend(format!(
                    "open_synth failed (code {code}): need control + PCM ROMs in {rom_dir}"
                )));
            }
            Ok(Mt32Voice {
                lib,
                context,
                active: HashMap::new(),
            })
        }
    }

    fn play_msg(&self, message: u32) {
        unsafe {
            if let Ok(play) = self
                .lib
                .get::<unsafe extern "C" fn(*mut c_void, u32) -> i32>(b"mt32emu_play_msg")
            {
                let _ = play(self.context, message);
            }
        }
    }
}

impl Drop for Mt32Voice {
    fn drop(&mut self) {
        unsafe {
            if let Ok(close) = self
                .lib
                .get::<unsafe extern "C" fn(*mut c_void)>(b"mt32emu_close_synth")
            {
                close(self.context);
            }
            if let Ok(free) = self
                .lib
                .get::<unsafe extern "C" fn(*mut c_void)>(b"mt32emu_free_context")
            {
                free(self.context);
            }
        }
    }
}

impl Instrument for Mt32Voice {
    fn process_event(&mut self, event: &TimedMusicalEvent, _rate: usize) {
        match &event.event {
            MusicalEventType::NoteOn {
                note_id,
                pitch_midi,
                velocity,
                ..
            } => {
                let velocity = (*velocity * 127.0).round().clamp(1.0, 127.0) as u32;
                self.play_msg(0x90 | ((*pitch_midi as u32) << 8) | (velocity << 16));
                self.active.insert(*note_id, *pitch_midi as i32);
            }
            MusicalEventType::NoteOff { note_id } => {
                if let Some(key) = self.active.remove(note_id) {
                    self.play_msg(0x80 | ((key as u32) << 8));
                }
            }
            _ => {}
        }
    }

    fn process_events(
        &mut self,
        events: Box<dyn Iterator<Item = TimedMusicalEvent> + Send + Sync>,
        rate: usize,
    ) {
        for event in events {
            self.process_event(&event, rate);
        }
    }

    fn generate_samples(&mut self, count: usize, _rate: usize) -> Vec<[f32; 2]> {
        unsafe {
            let mut interleaved = vec![0.0f32; count * 2];
            if let Ok(render) = self
                .lib
                .get::<unsafe extern "C" fn(*mut c_void, *mut f32, u32)>(b"mt32emu_render_float")
            {
                render(self.context, interleaved.as_mut_ptr(), count as u32);
            }
            interleaved
                .chunks_exact(2)
                .map(|pair| [pair[0], pair[1]])
                .collect()
        }
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}

/// Row length of the synthesized modules: speed 6 at 125 BPM.
pub const MOD_ROW_SECS: f32 = 0.12;

/// MIDI note to ProTracker period, clamped to the PT range.
/// Calibrated: MIDI 69 (A440) renders at 440 Hz through libopenmpt.
pub fn midi_to_period(midi: u8) -> u16 {
    (856.0 * 2.0f32.powf((36.0 - midi as f32) / 12.0))
        .round()
        .clamp(113.0, 856.0) as u16
}

/// Synthesize a 4-channel ProTracker module from a piano-roll event stream.
///
/// Each `NoteOn` lands on channel `n % 4` (overlaps retrigger, like real
/// trackers) with a `Cxx` volume effect; note ends become `EC0` cuts where
/// the slot is free. The single shared sample is a looping sine.
pub fn events_to_mod(events: &[TimedMusicalEvent]) -> Vec<u8> {
    struct Hit {
        start_row: usize,
        end_row: usize,
        period: u16,
        volume: u8,
    }
    let mut hits: Vec<Hit> = Vec::new();
    let mut last_end = 0.0f32;
    for event in events {
        if let MusicalEventType::NoteOn {
            pitch_midi,
            velocity,
            ..
        } = &event.event
        {
            let start_row = (event.time_seconds / MOD_ROW_SECS).floor().max(0.0) as usize;
            let end_row = ((event.time_seconds + event.real_duration) / MOD_ROW_SECS)
                .ceil()
                .max(0.0) as usize;
            last_end = last_end.max(event.time_seconds + event.real_duration);
            hits.push(Hit {
                start_row,
                end_row: end_row.max(start_row + 1),
                period: midi_to_period(*pitch_midi),
                volume: (*velocity * 64.0).round().clamp(1.0, 64.0) as u8,
            });
        }
    }
    let rows_total = (((last_end / MOD_ROW_SECS).ceil() as usize) + 1).max(64);
    let patterns = rows_total.div_ceil(64);
    // channels[c][row]: sounding note or empty; cuts[c][row]: EC0 cut.
    let mut channels: Vec<Vec<Option<(u16, u8)>>> = vec![vec![None; rows_total]; 4];
    let mut cuts: Vec<Vec<bool>> = vec![vec![false; rows_total]; 4];
    for (index, hit) in hits.iter().enumerate() {
        let channel = index % 4;
        if hit.start_row < rows_total {
            channels[channel][hit.start_row] = Some((hit.period, hit.volume));
        }
        if hit.end_row < rows_total && channels[channel][hit.end_row].is_none() {
            cuts[channel][hit.end_row] = true;
        }
    }

    let mut data = vec![0u8; 1084];
    data[0..12].copy_from_slice(b"mmlx events\0");
    data[20..26].copy_from_slice(b"sine  ");
    data[42..44].copy_from_slice(&32u16.to_be_bytes());
    data[44] = 0;
    data[45] = 64;
    data[46..48].copy_from_slice(&0u16.to_be_bytes());
    data[48..50].copy_from_slice(&32u16.to_be_bytes());
    data[950] = patterns as u8;
    data[951] = 0;
    for (i, entry) in data[952..952 + 128].iter_mut().enumerate() {
        *entry = (i % patterns) as u8;
    }
    data[1080..1084].copy_from_slice(b"M.K.");
    for pattern in 0..patterns {
        let mut bytes = vec![0u8; 1024];
        for row in 0..64 {
            let global = pattern * 64 + row;
            if global >= rows_total {
                break;
            }
            for channel in 0..4 {
                let slot = &mut bytes[(row * 4 + channel) * 4..][..4];
                if let Some((period, volume)) = channels[channel][global] {
                    slot[0] = (period >> 8) as u8 & 0x0F; // sample 1 hi-nibble (0) + period high
                    slot[1] = (period & 0xFF) as u8;
                    slot[2] = 0x1C; // sample 1 lo-nibble + effect C (volume)
                    slot[3] = volume;
                } else if cuts[channel][global] {
                    slot[0] = 0x00;
                    slot[1] = 0x00;
                    slot[2] = 0xE0;
                    slot[3] = 0xC0;
                }
            }
        }
        data.extend_from_slice(&bytes);
    }
    for i in 0..64 {
        let sample = ((i as f32 / 64.0 * std::f32::consts::TAU).sin() * 100.0) as i8;
        data.push(sample as u8);
    }
    data
}

/// Play an event stream through the module engine: synthesize + render.
pub fn render_events_mod(
    events: &[TimedMusicalEvent],
    sample_rate: u32,
    seconds: f32,
) -> Result<Vec<[f32; 2]>, TrackError> {
    render_mod(&events_to_mod(events), sample_rate, seconds)
}
