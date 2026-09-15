//! Routing tests with a constant dummy voice.

use mmlx_core::{Instrument, MusicalEventType, ParamValue, TimedMusicalEvent};
use mmlx_fx::{mix_segment, route_event, BusState};
use std::sync::{Arc, Mutex};

struct Constant(f32);

impl Instrument for Constant {
    fn process_event(&mut self, _event: &TimedMusicalEvent, _rate: usize) {}
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
        vec![[self.0, self.0]; count]
    }
}

fn constant(name: &str, level: f32) -> (String, Arc<Mutex<dyn Instrument>>) {
    (name.to_string(), Arc::new(Mutex::new(Constant(level))))
}

fn setter(instrument: &str, key: &str, value: ParamValue) -> TimedMusicalEvent {
    TimedMusicalEvent {
        time_seconds: 0.0,
        real_duration: 0.0,
        event: MusicalEventType::SetParameter {
            key: key.to_string(),
            value,
        },
        instrument_name: instrument.to_string(),
    }
}

#[test]
fn bus_and_send_routing() {
    let mut state = BusState::new();
    assert_eq!(state.bus_of("lead"), "main");
    route_event(
        &mut state,
        &setter("lead", "bus", ParamValue::String("music".to_string())),
    );
    assert_eq!(state.bus_of("lead"), "music");
    route_event(
        &mut state,
        &setter("lead", "send_bus", ParamValue::String("fx".to_string())),
    );
    route_event(&mut state, &setter("lead", "send", ParamValue::Number(0.5)));

    let instruments = vec![constant("lead", 1.0), constant("drums", 2.0)];
    let mut out = vec![[0.0; 2]; 4];
    mix_segment(&instruments, &state, &mut out, 44100);
    // lead: 1.0 -> music bus (gain 1); send 0.5 -> fx bus.
    // drums: 2.0 -> main bus. Master 1.0.
    // left[0] = 1.0 (music) + 0.5 (fx send) + 2.0 (main) = 3.5.
    assert!((out[0][0] - 3.5).abs() < 1e-6, "mixed: {:?}", out[0]);

    // Unset returns the voice to main with no send.
    route_event(&mut state, &setter("lead", "bus", ParamValue::Unset));
    route_event(&mut state, &setter("lead", "send", ParamValue::Unset));
    let mut out = vec![[0.0; 2]; 4];
    mix_segment(&instruments, &state, &mut out, 44100);
    assert!(
        (out[0][0] - 3.0).abs() < 1e-6,
        "lead back on main: {:?}",
        out[0]
    );

    // Unknown keys are ignored.
    let mut state2 = BusState::new();
    route_event(
        &mut state2,
        &setter("lead", "patch", ParamValue::String("sine".to_string())),
    );
    assert_eq!(state2.bus_of("lead"), "main");
}

#[test]
fn mixer_gain_applies_per_bus() {
    let mut state = BusState::new();
    state.mixer_mut().set_gain("music", 0.0);
    route_event(
        &mut state,
        &setter("lead", "bus", ParamValue::String("music".to_string())),
    );
    let instruments = vec![constant("lead", 1.0)];
    let mut out = vec![[0.0; 2]; 4];
    mix_segment(&instruments, &state, &mut out, 44100);
    assert!((out[0][0]).abs() < 1e-6, "muted bus sums to silence");
}
