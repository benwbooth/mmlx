//! `mmlx-clap`: CLAP export scaffold for mmlx instruments/effects.
//!
//! v1 is the Clack gain example renamed: it proves the cdylib entry,
//! descriptor, and buffer plumbing compile into a CLAP-compliant library.
//! Next: expose every instrument param (f64, no MIDI 7-bit limits) via the
//! params extension and render event streams inside `process`.

use clack_plugin::prelude::*;

pub struct MmlxGain;

impl Plugin for MmlxGain {
    type AudioProcessor<'a> = MmlxGainAudioProcessor;

    type Shared<'a> = ();
    type MainThread<'a> = ();
}

impl DefaultPluginFactory for MmlxGain {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("xyz.mmlx.gain", "mmlx Gain (scaffold)")
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<Self::Shared<'_>, PluginError> {
        Ok(())
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        _shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(())
    }
}

pub struct MmlxGainAudioProcessor;

impl<'a> PluginAudioProcessor<'a, (), ()> for MmlxGainAudioProcessor {
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &(),
        _shared: &'a (),
        _audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Self)
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        _events: Events,
    ) -> Result<ProcessStatus, PluginError> {
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
                            *output = input * 2.0
                        }
                    }
                    ChannelPair::InPlace(buf) => {
                        for sample in buf {
                            *sample *= 2.0
                        }
                    }
                }
            }
        }

        Ok(ProcessStatus::Continue)
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
}
