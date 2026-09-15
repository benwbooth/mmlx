//! `mmlx-clap`: CLAP export scaffold for mmlx instruments/effects.
//!
//! [`MmlxGain`] is a gain plugin proving the cdylib entry, descriptor,
//! buffer plumbing, and a first-class `gain_db` parameter (f64, -60..+24 dB
//! — no MIDI 7-bit limits). Next: expose every instrument param the same
//! way and render event streams inside `process`.

use clack_common::{
    events::{
        event_types::ParamValueEvent,
        io::{InputEvents, OutputEvents},
    },
    utils::{ClapId, Cookie},
};
use clack_extensions::params::{
    ParamDisplayWriter, ParamInfo, ParamInfoFlags, ParamInfoWriter, PluginAudioProcessorParams,
    PluginMainThreadParams,
};
use clack_plugin::prelude::*;
use core::ffi::CStr;
use core::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

const GAIN_ID: u32 = 0;
const GAIN_MIN_DB: f64 = -60.0;
const GAIN_MAX_DB: f64 = 24.0;
const GAIN_DEFAULT_DB: f64 = 6.0206; // ×2.0, the scaffold's historic behavior.

pub fn db_to_linear(db: f64) -> f32 {
    10.0f32.powf(db as f32 / 20.0)
}

pub struct MmlxGainShared {
    gain_db_bits: AtomicU64,
}

impl MmlxGainShared {
    fn new() -> Self {
        MmlxGainShared {
            gain_db_bits: AtomicU64::new(GAIN_DEFAULT_DB.to_bits()),
        }
    }

    fn load_gain_db(&self) -> f64 {
        f64::from_bits(self.gain_db_bits.load(Ordering::Relaxed))
    }

    fn store_gain_db(&self, db: f64) {
        self.gain_db_bits.store(
            db.clamp(GAIN_MIN_DB, GAIN_MAX_DB).to_bits(),
            Ordering::Relaxed,
        );
    }
}

fn apply_param_events(shared: &MmlxGainShared, input: &InputEvents) {
    for event in input.iter() {
        if let Some(param_event) = event.as_event::<ParamValueEvent>() {
            if param_event.param_id() == Some(ClapId::new(GAIN_ID)) {
                shared.store_gain_db(param_event.value());
            }
        }
    }
}

impl<'a> PluginShared<'a> for MmlxGainShared {}

pub struct MmlxGain;

impl Plugin for MmlxGain {
    type AudioProcessor<'a> = MmlxGainAudioProcessor<'a>;

    type Shared<'a> = MmlxGainShared;
    type MainThread<'a> = MmlxGainMainThread<'a>;
}

impl DefaultPluginFactory for MmlxGain {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("xyz.mmlx.gain", "mmlx Gain (scaffold)")
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<Self::Shared<'_>, PluginError> {
        Ok(MmlxGainShared::new())
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(MmlxGainMainThread { shared })
    }
}

pub struct MmlxGainMainThread<'a> {
    shared: &'a MmlxGainShared,
}

impl<'a> PluginMainThread<'a, MmlxGainShared> for MmlxGainMainThread<'a> {}

impl PluginMainThreadParams for MmlxGainMainThread<'_> {
    fn count(&self) -> u32 {
        1
    }

    fn get_info(&self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index != 0 {
            return;
        }
        info.set(&ParamInfo {
            id: ClapId::new(GAIN_ID),
            flags: ParamInfoFlags::IS_AUTOMATABLE,
            cookie: Cookie::empty(),
            name: b"Gain",
            module: b"mmlx",
            min_value: GAIN_MIN_DB,
            max_value: GAIN_MAX_DB,
            default_value: GAIN_DEFAULT_DB,
        });
    }

    fn get_value(&self, param_id: ClapId) -> Option<f64> {
        if param_id == ClapId::new(GAIN_ID) {
            Some(self.shared.load_gain_db())
        } else {
            None
        }
    }

    fn value_to_text(
        &self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> core::fmt::Result {
        if param_id != ClapId::new(GAIN_ID) {
            return Err(core::fmt::Error);
        }
        write!(writer, "{value:.2} dB")
    }

    fn text_to_value(&self, param_id: ClapId, text: &CStr) -> Option<f64> {
        if param_id != ClapId::new(GAIN_ID) {
            return None;
        }
        text.to_str()
            .ok()?
            .trim()
            .trim_end_matches("dB")
            .trim()
            .parse::<f64>()
            .ok()
            .map(|value| value.clamp(GAIN_MIN_DB, GAIN_MAX_DB))
    }

    fn flush(
        &self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
        apply_param_events(self.shared, input_parameter_changes);
    }
}

pub struct MmlxGainAudioProcessor<'a> {
    shared: &'a MmlxGainShared,
}

impl<'a> PluginAudioProcessor<'a, MmlxGainShared, MmlxGainMainThread<'a>>
    for MmlxGainAudioProcessor<'a>
{
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &MmlxGainMainThread<'a>,
        shared: &'a MmlxGainShared,
        _audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Self { shared })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        _events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        let gain = db_to_linear(self.shared.load_gain_db());
        for mut port_pair in &mut audio {
            let Some(channel_pairs) = port_pair.channels()?.into_f32() else {
                continue;
            };

            for channel_pair in channel_pairs {
                match channel_pair {
                    ChannelPair::InputOnly(_) => {}
                    ChannelPair::OutputOnly(buf) => buf.fill(0.0),
                    ChannelPair::InputOutput(input, output) => {
                        for (input, output) in input.iter().zip(output) {
                            *output = input * gain
                        }
                    }
                    ChannelPair::InPlace(buf) => {
                        for sample in buf {
                            *sample *= gain
                        }
                    }
                }
            }
        }

        Ok(ProcessStatus::Continue)
    }
}

impl PluginAudioProcessorParams for MmlxGainAudioProcessor<'_> {
    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
        apply_param_events(self.shared, input_parameter_changes);
    }
}

clack_export_entry!(SinglePluginEntry<MmlxGain>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_identifies_mmlx() {
        let descriptor = MmlxGain::get_descriptor();
        assert_eq!(descriptor.id().unwrap().to_bytes(), b"xyz.mmlx.gain");
        assert_eq!(
            descriptor.name().unwrap().to_str().unwrap(),
            "mmlx Gain (scaffold)"
        );
    }

    #[test]
    fn gain_param_round_trips() {
        let shared = MmlxGainShared::new();
        assert!((shared.load_gain_db() - GAIN_DEFAULT_DB).abs() < 1e-9);
        shared.store_gain_db(100.0);
        assert_eq!(shared.load_gain_db(), GAIN_MAX_DB);
        shared.store_gain_db(-100.0);
        assert_eq!(shared.load_gain_db(), GAIN_MIN_DB);
        assert!((db_to_linear(6.0206) - 2.0).abs() < 0.001);
        assert!((db_to_linear(-60.0) - 0.001).abs() < 0.0001);
    }
}
