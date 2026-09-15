//! `mmlx-fx`: buffer effects over rendered stereo audio.
//!
//! First effect: an N-band graphic equalizer built from RBJ peaking
//! biquads (design inspired by Kog's 31-band EQ; own implementation).
//! Effects run on `&mut [[f32; 2]]` sample buffers, after instruments mix.

use std::f32::consts::PI;

/// One peaking biquad section (Direct Form I).
#[derive(Clone, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// Peaking filter at `freq_hz` with quality `q` and `gain_db`.
    pub fn peaking(freq_hz: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq_hz / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        let _ = sin_w0;
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        Biquad {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Standard 10-band graphic-EQ frequencies (Hz).
pub const GRAPHIC_FREQS_10: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Graphic equalizer: one peaking section per band, per channel.
pub struct Equalizer {
    frequencies: Vec<f32>,
    gains_db: Vec<f32>,
    preamp_db: f32,
    q: f32,
    sample_rate: f32,
    left: Vec<Biquad>,
    right: Vec<Biquad>,
}

impl Equalizer {
    pub fn new(frequencies: &[f32], sample_rate: f32) -> Self {
        let mut eq = Equalizer {
            frequencies: frequencies.to_vec(),
            gains_db: vec![0.0; frequencies.len()],
            preamp_db: 0.0,
            q: 1.4,
            sample_rate,
            left: Vec::new(),
            right: Vec::new(),
        };
        eq.rebuild();
        eq
    }

    fn rebuild(&mut self) {
        self.left = self
            .frequencies
            .iter()
            .zip(&self.gains_db)
            .map(|(freq, gain)| Biquad::peaking(*freq, self.q, *gain, self.sample_rate))
            .collect();
        self.right = self.left.clone();
    }

    pub fn set_gain(&mut self, band: usize, gain_db: f32) {
        if let Some(slot) = self.gains_db.get_mut(band) {
            *slot = gain_db.clamp(-20.0, 20.0);
            self.rebuild();
        }
    }

    pub fn set_preamp(&mut self, preamp_db: f32) {
        self.preamp_db = preamp_db.clamp(-20.0, 20.0);
    }

    pub fn process(&mut self, buffer: &mut [[f32; 2]]) {
        let preamp = 10.0f32.powf(self.preamp_db / 20.0);
        for frame in buffer.iter_mut() {
            let mut left = frame[0];
            let mut right = frame[1];
            for section in &mut self.left {
                left = section.process(left);
            }
            for section in &mut self.right {
                right = section.process(right);
            }
            frame[0] = left * preamp;
            frame[1] = right * preamp;
        }
    }
}

/// Text-DAW mix stage: named buses with gains plus a master gain.
///
/// Voices declare `param!(bus = "drums")`; the backend renders one stereo
/// buffer per bus and sums `gain * bus` here. Sends are pre-fader copies
/// added into the target bus before its gain. All pure buffer math, so it
/// is fully testable without an audio device.
#[derive(Default)]
pub struct BusMixer {
    gains: std::collections::HashMap<String, f32>,
    master: f32,
}

impl BusMixer {
    pub fn new() -> Self {
        BusMixer {
            gains: std::collections::HashMap::new(),
            master: 1.0,
        }
    }

    /// Set a bus gain (linear). Unknown buses default to 1.0.
    pub fn set_gain(&mut self, bus: &str, gain: f32) {
        self.gains.insert(bus.to_string(), gain.max(0.0));
    }

    pub fn set_master(&mut self, master: f32) {
        self.master = master.max(0.0);
    }

    /// Sum `master * Σ gain[bus] * buffer[bus]` over the longest buffer.
    pub fn mixdown(
        &self,
        buses: &std::collections::HashMap<String, Vec<[f32; 2]>>,
    ) -> Vec<[f32; 2]> {
        let frames = buses.values().map(Vec::len).max().unwrap_or(0);
        let mut out = vec![[0.0f32; 2]; frames];
        let mut names: Vec<&String> = buses.keys().collect();
        names.sort();
        for name in names {
            let gain = self.gains.get(name).copied().unwrap_or(1.0);
            for (i, frame) in buses[name].iter().enumerate() {
                out[i][0] += frame[0] * gain;
                out[i][1] += frame[1] * gain;
            }
        }
        for frame in &mut out {
            frame[0] *= self.master;
            frame[1] *= self.master;
        }
        out
    }
}
