//! Text-DAW routing: which mix bus each instrument renders into, plus sends.
//!
//! Voices declare `param!(bus = "drums")` with optional
//! `param!(send_bus = "fx", send = 0.3)`; the backend updates [`BusState`]
//! live from `SetParameter` events and renders through [`mix_segment`].
//! Pure buffer math — fully testable without an audio device.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::BusMixer;

pub struct BusState {
    bus_of: HashMap<String, String>,
    send_of: HashMap<String, (String, f32)>,
    mixer: BusMixer,
}

impl BusState {
    pub fn new() -> Self {
        BusState {
            bus_of: HashMap::new(),
            send_of: HashMap::new(),
            mixer: BusMixer::new(),
        }
    }

    pub fn bus_of(&self, instrument: &str) -> &str {
        self.bus_of
            .get(instrument)
            .map(String::as_str)
            .unwrap_or("main")
    }

    pub fn mixer(&self) -> &BusMixer {
        &self.mixer
    }

    pub fn mixer_mut(&mut self) -> &mut BusMixer {
        &mut self.mixer
    }
}

impl Default for BusState {
    fn default() -> Self {
        Self::new()
    }
}

/// Fold one event's routing params into state (`bus`/`send_bus` strings,
/// `send` amount; `Unset` clears).
pub fn route_event(state: &mut BusState, event: &TimedMusicalEvent) {
    if let MusicalEventType::SetParameter { key, value } = &event.event {
        let instrument = &event.instrument_name;
        match (key.as_str(), value) {
            ("bus", ParamValue::String(bus)) => {
                state.bus_of.insert(instrument.clone(), bus.clone());
            }
            ("bus", ParamValue::Unset) => {
                state.bus_of.remove(instrument);
            }
            ("send_bus", ParamValue::String(bus)) => {
                let amount = state
                    .send_of
                    .get(instrument)
                    .map(|(_, amount)| *amount)
                    .unwrap_or(0.0);
                state
                    .send_of
                    .insert(instrument.clone(), (bus.clone(), amount));
            }
            ("send_bus", ParamValue::Unset) => {
                state.send_of.remove(instrument);
            }
            ("send", ParamValue::Number(amount)) => {
                let bus = state
                    .send_of
                    .get(instrument)
                    .map(|(bus, _)| bus.clone())
                    .unwrap_or_else(|| "fx".to_string());
                state
                    .send_of
                    .insert(instrument.clone(), (bus, amount.clamp(0.0, 1.0)));
            }
            ("send", ParamValue::Unset) => {
                state.send_of.remove(instrument);
            }
            _ => {}
        }
    }
}

/// Render every instrument into its bus buffer (plus send copies), then sum
/// buses into `out`. `out` must be zeroed by the caller.
pub fn mix_segment(
    instruments: &[(String, Arc<Mutex<dyn Instrument>>)],
    state: &BusState,
    out: &mut [[f32; 2]],
    rate: usize,
) {
    let mut buses: HashMap<String, Vec<[f32; 2]>> = HashMap::new();
    for (name, inst_arc) in instruments {
        let samples = inst_arc.lock().unwrap().generate_samples(out.len(), rate);
        let entry = buses
            .entry(state.bus_of(name).to_string())
            .or_insert_with(|| vec![[0.0; 2]; out.len()]);
        for (frame, sample) in entry.iter_mut().zip(samples.iter()) {
            frame[0] += sample[0];
            frame[1] += sample[1];
        }
        if let Some((send_bus, amount)) = state.send_of.get(name) {
            if *amount > 0.0 {
                let entry = buses
                    .entry(send_bus.clone())
                    .or_insert_with(|| vec![[0.0; 2]; out.len()]);
                for (frame, sample) in entry.iter_mut().zip(samples.iter()) {
                    frame[0] += sample[0] * amount;
                    frame[1] += sample[1] * amount;
                }
            }
        }
    }
    for (i, frame) in state.mixer.mixdown(&buses).iter().enumerate() {
        out[i][0] += frame[0];
        out[i][1] += frame[1];
    }
}
