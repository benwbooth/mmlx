use super::{EventQueue, InstrumentsMap, SynthTime};
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::Stream;
use log::{debug, error, info, warn};
use mmlx_core::{Instrument, MusicalEventType};
use mmlx_fx::routing::{mix_segment, route_event, BusState};
use mmlx_synth::BasicSynth;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex}; // Import Stream

// Bus routing lives in mmlx_fx::routing (shared, headless-testable).

/// Sets up the audio host, device, stream, and shared state.
///
/// This function configures the audio system with a large buffer size (16384 samples)
/// to prioritize stability over latency, which helps prevent audio underruns
/// during playback of complex musical compositions.
///
/// The audio processing has been optimized to reduce allocations during the
/// real-time callback, using pre-allocated buffers for mixing and event processing.
///
/// Returns the shared state Arcs and the CPAL audio stream. The caller
/// is responsible for starting the stream with [`play_stream`] and
/// keeping it alive.
pub fn setup_audio() -> Result<(EventQueue, SynthTime, InstrumentsMap, Stream)> {
    info!("Setting up audio via setup_audio...");

    // --- Shared State Initialization ---
    let default_sample_rate = 44100; // Used for synth init if actual rate differs
    let event_queue: EventQueue = Arc::new(Mutex::new(VecDeque::new()));
    let synth_time: SynthTime = Arc::new(Mutex::new(0.0f32));
    let instruments_map_inner: HashMap<String, Arc<Mutex<dyn Instrument>>> = HashMap::from([(
        "basic_synth".to_string(),
        Arc::new(Mutex::new(BasicSynth::new(default_sample_rate))) as Arc<Mutex<dyn Instrument>>,
    )]);
    let instruments_map: InstrumentsMap = Arc::new(Mutex::new(instruments_map_inner));
    info!("Shared state initialized.");

    // --- CPAL Audio Setup ---
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("Failed to find default output device"))?;
    info!("Using default output device: {}", device.name()?);

    let desired_channels: u16 = 2; // Stereo
    let target_config = device
        .supported_output_configs()?
        .filter(|c| {
            c.channels() == desired_channels && c.sample_format() == cpal::SampleFormat::F32
        })
        .max_by_key(|c| c.max_sample_rate())
        .ok_or_else(|| anyhow::anyhow!("No supported f32 stereo output config found"))?
        .with_max_sample_rate(); // Use the highest rate found

    let mut config = target_config.config();

    // Configure a much larger buffer size to prevent underruns
    // Using a very large buffer (16384 samples) to prioritize stability over latency
    // This is approximately 370ms at 44.1kHz which should be acceptable for non-interactive music
    config.buffer_size = cpal::BufferSize::Fixed(16384);

    // On Linux, log additional information about the host
    #[cfg(target_os = "linux")]
    {
        info!("Running on Linux - host type: {:?}", host.id());
    }

    let actual_sample_rate = config.sample_rate.0;
    let channels = config.channels; // Should be 2
    info!(
        "Selected audio config: sample_rate={}, channels={}, format=f32",
        actual_sample_rate, channels
    );

    // ===== Global Low-Pass Filter to smooth clicks =====
    // One-pole LPF: y[n] = alpha * y[n-1] + (1-alpha) * x[n]
    let lp_cutoff = 4000.0_f32; // Cutoff frequency in Hz
    let lp_alpha = (-2.0 * std::f32::consts::PI * lp_cutoff / actual_sample_rate as f32).exp();
    let lp_beta = 1.0 - lp_alpha;
    // State for left/right channels
    let mut last_lp_l = 0.0_f32;
    let mut last_lp_r = 0.0_f32;

    if channels != desired_channels {
        return Err(anyhow::anyhow!(format!(
            "Failed to obtain stereo ({}ch) output config, got {}ch",
            desired_channels, channels
        )));
    }

    // --- Synth Logic (Data Callback) ---
    let eq_clone = event_queue.clone();
    let im_clone = instruments_map.clone();
    let st_clone = synth_time.clone();
    let bus_clone: Arc<Mutex<BusState>> = Arc::new(Mutex::new(BusState::new()));
    let mut samples_generated: u64 = 0;
    let mut last_callback_time = std::time::Instant::now();
    let mut callback_count = 0;
    let mut max_callback_time = std::time::Duration::from_secs(0);

    // Pre-allocate buffers to reduce allocations during callback
    let max_buffer_size = 16384;
    let mut pre_allocated_mix_buffer = vec![[0.0f32, 0.0f32]; max_buffer_size];
    // pre_allocated_drained_events removed; using local pre_block_events and mid_block_events instead
    let mut pre_allocated_instruments = Vec::with_capacity(8);

    // Enhanced error handler with more context
    let err_fn = |err| {
        error!("Audio stream error: {}", err);
        if let cpal::StreamError::BackendSpecific { err } = &err {
            error!("Backend error details: {:?}", err);
        }
    };

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // Measure callback timing to catch potential performance issues
            let callback_start = std::time::Instant::now();
            // We track time between callbacks but don't need to use it directly
            let _elapsed_since_last = callback_start.duration_since(last_callback_time);
            last_callback_time = callback_start;

            // Log if we detect potential timing issues (every 1000 callbacks)
            callback_count += 1;
            if callback_count % 1000 == 0 {
                if max_callback_time > std::time::Duration::from_millis(10) {
                    debug!(
                        "Audio callback stats - Max processing time: {:?}",
                        max_callback_time
                    );
                    max_callback_time = std::time::Duration::from_secs(0);
                }
            }

            let chunk_size_frames = data.len() / channels as usize;
            let current_chunk_start_time = samples_generated as f32 / actual_sample_rate as f32;
            let current_chunk_end_time =
                (samples_generated + chunk_size_frames as u64) as f32 / actual_sample_rate as f32;

            // Check if chunk size exceeds our pre-allocated buffer (unlikely but possible)
            if chunk_size_frames > max_buffer_size {
                warn!(
                    "Audio chunk size ({}) exceeds pre-allocated buffer size ({})",
                    chunk_size_frames, max_buffer_size
                );
            }

            // Drain events into pre-block and mid-block lists
            let mut pre_block_events: Vec<_> = Vec::new();
            let mut mid_block_events: Vec<_> = Vec::new();
            {
                let mut q = eq_clone.lock().unwrap();
                while let Some(evt) = q.front() {
                    if evt.time_seconds <= current_chunk_end_time {
                        let evt = q.pop_front().unwrap();
                        if evt.time_seconds <= current_chunk_start_time {
                            pre_block_events.push(evt);
                        } else {
                            mid_block_events.push(evt);
                        }
                    } else {
                        break;
                    }
                }
            }

            // Process pre-block events immediately and collect instruments
            pre_allocated_instruments.clear();
            let mut bus_state = bus_clone.lock().unwrap();
            {
                let im_guard = im_clone.lock().unwrap();
                // Handle parameter and note events before start of this block
                for event in &pre_block_events {
                    route_event(&mut bus_state, event);
                    if let MusicalEventType::Comment(s) = &event.event {
                        info!("[< {} >] @ {:.4}s", s, event.time_seconds);
                    } else if let Some(inst_arc) = im_guard.get(&event.instrument_name) {
                        let mut inst = inst_arc.lock().unwrap();
                        inst.process_event(&event, actual_sample_rate as usize);
                    }
                }
                // Collect active instruments with names for bus routing
                for (name, inst_arc) in im_guard.iter() {
                    pre_allocated_instruments.push((name.clone(), inst_arc.clone()));
                }
            }

            // Generate and Mix Samples, segmenting at mid-block events to schedule precisely
            // Clear mix buffer
            for frame in 0..chunk_size_frames {
                pre_allocated_mix_buffer[frame][0] = 0.0;
                pre_allocated_mix_buffer[frame][1] = 0.0;
            }

            if mid_block_events.is_empty() {
                // No mid-block events: fast path through the bus mixer
                mix_segment(
                    &pre_allocated_instruments,
                    &bus_state,
                    &mut pre_allocated_mix_buffer[..chunk_size_frames],
                    actual_sample_rate as usize,
                );
            } else {
                // Sort mid-block events by time
                mid_block_events.sort_by(|a, b| {
                    a.time_seconds
                        .partial_cmp(&b.time_seconds)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                // Mix in segments
                let mut prev_offset = 0;
                for event in &mid_block_events {
                    // Compute sample offset for event time
                    let offset_f =
                        (event.time_seconds - current_chunk_start_time) * actual_sample_rate as f32;
                    let mut offset = offset_f.floor() as usize;
                    if offset > chunk_size_frames {
                        offset = chunk_size_frames;
                    }
                    let seg_len = offset.saturating_sub(prev_offset);
                    if seg_len > 0 {
                        mix_segment(
                            &pre_allocated_instruments,
                            &bus_state,
                            &mut pre_allocated_mix_buffer[prev_offset..offset],
                            actual_sample_rate as usize,
                        );
                    }
                    // Process this mid-block event now
                    route_event(&mut bus_state, event);
                    if let MusicalEventType::Comment(s) = &event.event {
                        info!("[< {} >] @ {:.4}s", s, event.time_seconds);
                    } else if let Some(inst_arc) =
                        im_clone.lock().unwrap().get(&event.instrument_name)
                    {
                        let mut inst = inst_arc.lock().unwrap();
                        inst.process_event(&event, actual_sample_rate as usize);
                    }
                    prev_offset = offset;
                }
                // Final segment after last event
                let rem_len = chunk_size_frames.saturating_sub(prev_offset);
                if rem_len > 0 {
                    mix_segment(
                        &pre_allocated_instruments,
                        &bus_state,
                        &mut pre_allocated_mix_buffer[prev_offset..chunk_size_frames],
                        actual_sample_rate as usize,
                    );
                }
            }

            // Fill CPAL Buffer applying global low-pass filter per channel
            // One-pole LPF is applied sample-by-sample for left and right
            for frame_index in 0..chunk_size_frames {
                let buffer_index = frame_index * channels as usize;
                let x_l = pre_allocated_mix_buffer[frame_index][0];
                let x_r = pre_allocated_mix_buffer[frame_index][1];
                let y_l = lp_alpha * last_lp_l + lp_beta * x_l;
                let y_r = lp_alpha * last_lp_r + lp_beta * x_r;
                last_lp_l = y_l;
                last_lp_r = y_r;
                data[buffer_index] = y_l;
                data[buffer_index + 1] = y_r;
            }

            // Update Shared Synth Time
            samples_generated += chunk_size_frames as u64;
            let current_time_secs = samples_generated as f32 / actual_sample_rate as f32;
            {
                *st_clone.lock().unwrap() = current_time_secs;
            }

            // Measure callback completion time and update max if needed
            let callback_duration = callback_start.elapsed();
            if callback_duration > max_callback_time {
                max_callback_time = callback_duration;
            }

            // Log warning if callback takes too long (potential cause of underruns)
            let buffer_duration_ms =
                (chunk_size_frames as f64 / actual_sample_rate as f64) * 1000.0;
            if callback_duration.as_millis() as f64 > buffer_duration_ms * 0.75 {
                debug!(
                    "Audio callback taking {}ms (buffer: {:.2}ms) - potential underrun risk",
                    callback_duration.as_millis(),
                    buffer_duration_ms
                );
            }
        },
        err_fn,
        None,
    )?;

    info!("Audio stream built.");
    // Note: stream.play() is NOT called here. Caller must do it.

    Ok((event_queue, synth_time, instruments_map, stream))
}

/// Starts a stream from [`setup_audio`]. Lives here (not in callers) so
/// cpal stays behind the `audio` feature of this crate.
pub fn play_stream(stream: &Stream) -> Result<()> {
    use cpal::traits::StreamTrait;
    stream.play()?;
    Ok(())
}

// Queueing lives in the crate root (`mmlx_audio::queue_note`) so it works
// without the `audio` backend feature. This module only adds live output.
