//! `mmlx-sf2`: SoundFont wavetable instrument over mmlx event streams.
//!
//! [`Sf2Synth`] wraps RustySynth: `NoteOn` → channel-0 `note_on`,
//! `NoteOff` → `note_off`, rendered into stereo buffers.
//! [`minimal_sf2`] builds a tiny single-sine bank in memory so tests and
//! examples run without external sample files.

use mmlx_core::{Instrument, MusicalEventType, TimedMusicalEvent};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::collections::HashMap;
use std::io::Cursor;

pub struct Sf2Synth {
    synth: Synthesizer,
    active: HashMap<u64, i32>,
}

impl Sf2Synth {
    pub fn new(soundfont: &std::sync::Arc<SoundFont>, sample_rate: i32) -> Self {
        let settings = SynthesizerSettings::new(sample_rate);
        Sf2Synth {
            synth: Synthesizer::new(soundfont, &settings)
                .expect("synthesizer from valid SoundFont"),
            active: HashMap::new(),
        }
    }

    pub fn voice_count(&self) -> usize {
        self.active.len()
    }
}

impl Instrument for Sf2Synth {
    fn process_event(&mut self, event: &TimedMusicalEvent, _rate: usize) {
        match &event.event {
            MusicalEventType::NoteOn {
                note_id,
                pitch_midi,
                velocity,
                ..
            } => {
                let velocity = (*velocity * 127.0).round().clamp(1.0, 127.0) as i32;
                self.synth.note_on(0, *pitch_midi as i32, velocity);
                self.active.insert(*note_id, *pitch_midi as i32);
            }
            MusicalEventType::NoteOff { note_id } => {
                if let Some(key) = self.active.remove(note_id) {
                    self.synth.note_off(0, key);
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
        let mut left = vec![0.0f32; count];
        let mut right = vec![0.0f32; count];
        self.synth.render(&mut left, &mut right);
        left.into_iter()
            .zip(right)
            .map(|(left, right)| [left, right])
            .collect()
    }

    fn is_idle(&self) -> bool {
        self.active.is_empty()
    }
}

// --- minimal in-memory SoundFont (single sine preset) ---

fn chunk(tag: &[u8; 4], body: &[u8], out: &mut Vec<u8>) {
    // NOTE: pad odd bodies *inside* the size. This reader does not skip the
    // standard RIFF pad byte, so an excluded pad would misalign the stream.
    out.extend_from_slice(tag);
    let padded = body.len() % 2 == 1;
    out.extend_from_slice(&((body.len() + padded as usize) as u32).to_le_bytes());
    out.extend_from_slice(body);
    if padded {
        out.push(0);
    }
}

fn fixed_name(name: &str, len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    let copy = name.len().min(len);
    bytes[..copy].copy_from_slice(&name.as_bytes()[..copy]);
    bytes
}

/// Build a minimal SF2 bank: one preset, one sine-wave instrument looping
/// its whole sample. `sample_rate` Hz, `freq_hz` sine at MIDI `pitch`.
pub fn minimal_sf2(sample_rate: u32, freq_hz: f32, pitch: u8) -> Vec<u8> {
    let frames = sample_rate as usize;
    let mut samples = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let sample = (2.0 * std::f32::consts::PI * freq_hz * i as f32 / sample_rate as f32).sin();
        samples.extend_from_slice(&((sample * 10000.0) as i16).to_le_bytes());
    }

    let mut sdta = Vec::new();
    chunk(b"smpl", &samples, &mut sdta);

    // Preset header: one preset + terminal record (38 bytes each).
    let mut phdr = fixed_name("Sine", 20);
    phdr.extend_from_slice(&0u16.to_le_bytes()); // preset
    phdr.extend_from_slice(&0u16.to_le_bytes()); // bank
    phdr.extend_from_slice(&0u16.to_le_bytes()); // bag index
    phdr.extend_from_slice(&0u32.to_le_bytes()); // library
    phdr.extend_from_slice(&0u32.to_le_bytes()); // genre
    phdr.extend_from_slice(&0u32.to_le_bytes()); // morphology
    phdr.extend_from_slice(&fixed_name("EOP", 20));
    phdr.extend_from_slice(&0u16.to_le_bytes());
    phdr.extend_from_slice(&0u16.to_le_bytes());
    phdr.extend_from_slice(&1u16.to_le_bytes()); // one bag
    phdr.extend_from_slice(&0u32.to_le_bytes());
    phdr.extend_from_slice(&0u32.to_le_bytes());
    phdr.extend_from_slice(&0u32.to_le_bytes());

    // Preset bag: one zone (gen 0, mod 0) + terminal (gen/mod counts).
    let pbag = vec![0u8, 0, 0, 0, 1u8, 0, 0u8, 0];
    // Preset modulators: terminal record (10 bytes).
    let pmod = vec![0u8; 10];
    // Preset generators: instrument id (41) = 0, then terminal.
    let pgen = vec![41u8, 0, 0, 0, 0, 0, 0, 0];

    // Instrument: one + terminal (22 bytes each).
    let mut inst = fixed_name("SineInst", 20);
    inst.extend_from_slice(&0u16.to_le_bytes());
    inst.extend_from_slice(&fixed_name("EOI", 20));
    inst.extend_from_slice(&1u16.to_le_bytes());
    // Instrument bag: one zone + terminal (gen/mod counts).
    let ibag = vec![0u8, 0, 0, 0, 1u8, 0, 0u8, 0];
    let imod = vec![0u8; 10];
    // Instrument generators: sampleID (53) = 0, then terminal.
    let igen = vec![53u8, 0, 0, 0, 0, 0, 0, 0];

    // Sample header: one + terminal (46 bytes each). End indices are
    // inclusive in this reader's sanity check (end < wave length).
    let mut shdr = fixed_name("sine", 20);
    for value in [0u32, frames as u32 - 1, 0, frames as u32 - 1, sample_rate] {
        shdr.extend_from_slice(&value.to_le_bytes());
    }
    shdr.push(pitch);
    shdr.push(0); // pitch correction
    shdr.extend_from_slice(&0u16.to_le_bytes()); // link
    shdr.extend_from_slice(&1u16.to_le_bytes()); // mono sample
    shdr.extend_from_slice(&fixed_name("EOS", 20));
    shdr.extend_from_slice(&[0u8; 26]);

    let mut pdta = Vec::new();
    chunk(b"phdr", &phdr, &mut pdta);
    chunk(b"pbag", &pbag, &mut pdta);
    chunk(b"pmod", &pmod, &mut pdta);
    chunk(b"pgen", &pgen, &mut pdta);
    chunk(b"inst", &inst, &mut pdta);
    chunk(b"ibag", &ibag, &mut pdta);
    chunk(b"imod", &imod, &mut pdta);
    chunk(b"igen", &igen, &mut pdta);
    chunk(b"shdr", &shdr, &mut pdta);

    let mut info = Vec::new();
    let mut version = vec![2u8, 0, 1, 0];
    chunk(b"ifil", &version, &mut info);
    version.clear();
    chunk(b"isng", b"EMU8000", &mut info);
    chunk(b"INAM", b"mmlx-minimal", &mut info);

    let mut sfbk = Vec::new();
    sfbk.extend_from_slice(b"sfbk");
    let mut lists = Vec::new();
    let mut info_list = b"INFO".to_vec();
    info_list.extend(info);
    chunk(b"LIST", &info_list, &mut lists);
    let mut sdta_list = b"sdta".to_vec();
    sdta_list.extend(sdta);
    chunk(b"LIST", &sdta_list, &mut lists);
    let mut pdta_list = b"pdta".to_vec();
    pdta_list.extend(pdta);
    chunk(b"LIST", &pdta_list, &mut lists);
    sfbk.extend(lists);

    let mut riff = b"RIFF".to_vec();
    riff.extend_from_slice(&((sfbk.len()) as u32).to_le_bytes());
    riff.extend(sfbk);
    riff
}

/// Load the in-memory minimal bank.
pub fn minimal_soundfont(sample_rate: u32, freq_hz: f32, pitch: u8) -> std::sync::Arc<SoundFont> {
    let bytes = minimal_sf2(sample_rate, freq_hz, pitch);
    let font = SoundFont::new(&mut Cursor::new(bytes)).expect("minimal bank parses");
    std::sync::Arc::new(font)
}
