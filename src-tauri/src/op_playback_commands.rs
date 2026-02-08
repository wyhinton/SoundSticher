// Multi-timeline operation playback commands (Model A)
//
// This module provides Tauri commands for managing multiple timeline sessions,
// where each timeline has its own playback graph, progress, and state.

use crate::logging::{LogSystem, LoggingService};
use crate::playback::op_playback::{
    AudioSpec, PlayableOp, PlaybackGraph, SampleTime, TimelineSourceBuilder,
};
use crate::playback_ops::merge_playback::MergePlaybackOp;
use crate::playback_ops::sample_playback::SamplePlayableOp;
use crate::sample_cache::SampleCacheService;
use crate::timeline_playback_commands::{
    AppTimelinePlaybackState, PlaybackSession, PlaybackSessionDebugInfo, TimelineId,
};
use crate::{emit_logged, log_debug, log_info, send_channel_event};
use rodio::{OutputStream, Sink};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

/// Serializable audio spec for debugging
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSpecDebugInfo {
    pub sample_rate: u32,
    pub channels: u16,
}

impl From<AudioSpec> for AudioSpecDebugInfo {
    fn from(spec: AudioSpec) -> Self {
        Self {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        }
    }
}

/// Events emitted during playback graph building
#[derive(Clone, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum OpPlaybackBuildGraphEvent {
    Started {
        timeline_id: TimelineId,
        operation_count: usize,
    },
    Progress {
        timeline_id: TimelineId,
        operation_name: String,
        operation_index: usize,
        total_operations: usize,
        duration_seconds: f64,
    },
    Finished {
        timeline_id: TimelineId,
        operation_count: usize,
        total_duration_seconds: f64,
        sample_rate: u32,
        channels: u16,
    },
}

/// The type of playback operation
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OpType {
    /// A simple sample-based operation (default)
    #[default]
    Sample,
    /// A merge operation that combines multiple inputs
    Merge,
}

/// Child input for a merge operation
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeInputRequest {
    /// File path to load samples from
    pub file_path: Option<String>,

    /// Pre-loaded samples (f32, interleaved)
    pub samples: Option<Vec<f32>>,

    /// Offset time in seconds within the merge operation
    pub offset: f64,

    /// Gain for this input (0.0 to 1.0+)
    pub gain: Option<f32>,
}

/// Request to add an operation to the playback graph
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOpRequest {
    /// Unique name for this operation
    pub name: String,

    /// Type of operation (sample or merge)
    #[serde(default)]
    pub op_type: OpType,

    /// File path to load samples from (for sample-based ops)
    pub file_path: Option<String>,

    /// Pre-loaded samples (f32, interleaved)
    pub samples: Option<Vec<f32>>,

    /// Start time in seconds on the timeline
    pub start_time: f64,

    /// End time in seconds on the timeline (if None, uses operation duration)
    pub end_time: Option<f64>,

    /// Gain for this operation (0.0 to 1.0+)
    pub gain: Option<f32>,

    /// Child inputs for merge operations
    pub inputs: Option<Vec<MergeInputRequest>>,
}

/// Response after adding an operation
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOpResponse {
    pub name: String,
    pub op_id: u64,
    pub duration_seconds: f64,
}

/// Request to build a playback graph from multiple operations
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOpPlaybackGraphRequest {
    /// Operations to add to the graph
    pub operations: Vec<AddOpRequest>,

    /// Sample rate for playback
    pub sample_rate: Option<u32>,

    /// Number of channels
    pub channels: Option<u16>,

    /// Whether to loop playback
    pub loop_playback: Option<bool>,
}

/// Response after building a graph
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGraphResponse {
    pub operation_count: usize,
    pub total_duration_seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Build a playback graph for a specific timeline
///
/// This command delegates to `TimelinePlaybackManager` which:
/// 1. Selects the appropriate builder (OpGraphPlaybackBuilder)
/// 2. Builds the session (pure — no state mutation)
/// 3. Registers the session in `AppTimelinePlaybackState`
///
/// The builder is reusable and side-effect free. Only the manager
/// decides where sessions live.
#[tauri::command]
pub async fn op_playback_build_graph(
    timeline_id: TimelineId,
    request: BuildOpPlaybackGraphRequest,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    on_event: Channel<OpPlaybackBuildGraphEvent>,
) -> Result<BuildGraphResponse, String> {
    use crate::playback::timeline_manager::{TimelinePlaybackManager, TimelineSource};

    let state = state.inner().clone();
    let sample_cache = sample_cache.inner().clone();
    let logging_service = logging_service.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let manager = TimelinePlaybackManager::new(state, sample_cache, logging_service);
        let source = TimelineSource::Operation { request };

        manager.build_timeline(timeline_id, source, |e| {
            // Convert TimelinePlaybackEvent → OpPlaybackBuildGraphEvent for backward compat
            let op_event = match e {
                crate::playback::timeline_manager::TimelinePlaybackEvent::BuildStarted {
                    timeline_id,
                    operation_count,
                } => OpPlaybackBuildGraphEvent::Started {
                    timeline_id,
                    operation_count,
                },
                crate::playback::timeline_manager::TimelinePlaybackEvent::BuildProgress {
                    timeline_id,
                    operation_name,
                    operation_index,
                    total_operations,
                    duration_seconds,
                } => OpPlaybackBuildGraphEvent::Progress {
                    timeline_id,
                    operation_name,
                    operation_index,
                    total_operations,
                    duration_seconds,
                },
                crate::playback::timeline_manager::TimelinePlaybackEvent::BuildFinished {
                    timeline_id,
                    operation_count,
                    total_duration_seconds,
                    sample_rate,
                    channels,
                } => OpPlaybackBuildGraphEvent::Finished {
                    timeline_id,
                    operation_count,
                    total_duration_seconds,
                    sample_rate,
                    channels,
                },
                crate::playback::timeline_manager::TimelinePlaybackEvent::BuildError { .. } => {
                    return; // Errors are propagated via Result, not events
                }
            };
            send_channel_event!(on_event, op_event);
        })
    })
    .await
    .map_err(|e| format!("Failed to execute build graph task: {}", e))?
}

/// Build a playback graph from the request (legacy version without cache)
#[tauri::command]
pub async fn op_playback_build_graph_legacy(
    timeline_id: TimelineId,
    request: BuildOpPlaybackGraphRequest,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    _sample_cache: State<'_, Arc<SampleCacheService>>, // Not used in legacy mode
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    on_event: Channel<OpPlaybackBuildGraphEvent>,
) -> Result<BuildGraphResponse, String> {
    let state = state.inner().clone();
    let logging_service = logging_service.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_build_graph_legacy",
                &format!(
                    "Building playback graph (LEGACY - no cache) for timeline '{}' with {} operations",
                    timeline_id, request.operations.len()
                )
            );
        }
        let total_graph_ops = count_graph_ops(&request)?;
        // Emit started event
        send_channel_event!(
            on_event,
            OpPlaybackBuildGraphEvent::Started {
                timeline_id: timeline_id.clone(),
                operation_count: total_graph_ops
            }
        );

        let sample_rate = request.sample_rate.unwrap_or(44100);
        let channels = request.channels.unwrap_or(2);
        let spec = AudioSpec::new(sample_rate, channels);
        let loop_playback = request.loop_playback.unwrap_or(true);

        // Create new graph
        let graph = Arc::new(PlaybackGraph::new(spec));
        let mut op_ids = HashMap::new();

        for (index, op_request) in request.operations.iter().enumerate() {
            // Create the playable operation based on type
            let op: Box<dyn PlayableOp> = match op_request.op_type {
                OpType::Sample => {
                    // Get samples (either from file or directly provided)
                    let samples = if let Some(ref samples) = op_request.samples {
                        samples.clone()
                    } else if let Some(ref file_path) = op_request.file_path {
                        // Load samples from file directly (no cache)
                        load_audio_samples(file_path, sample_rate, channels)?
                    } else {
                        return Err(format!(
                            "Sample operation '{}' must have either 'samples' or 'filePath'",
                            op_request.name
                        ));
                    };

                    Box::new(SamplePlayableOp::new(samples, spec))
                }
                OpType::Merge => {
                    // Build a merge operation from child inputs
                    let inputs = op_request.inputs.as_ref().ok_or_else(|| {
                        format!(
                            "Merge operation '{}' must have 'inputs' array",
                            op_request.name
                        )
                    })?;

                    if inputs.is_empty() {
                        return Err(format!(
                            "Merge operation '{}' must have at least one input",
                            op_request.name
                        ));
                    }

                    let mut builder = MergePlaybackOp::builder(spec);

                    for (i, input) in inputs.iter().enumerate() {
                        // Get samples for this input
                        let samples = if let Some(ref samples) = input.samples {
                            samples.clone()
                        } else if let Some(ref file_path) = input.file_path {
                            // Load samples from file directly (no cache)
                            load_audio_samples(file_path, sample_rate, channels)?
                        } else {
                            return Err(format!(
                                "Merge input {} in operation '{}' must have either 'samples' or 'filePath'",
                                i, op_request.name
                            ));
                        };

                        let child_op = SamplePlayableOp::new(samples, spec);

                        let offset = SampleTime::from_seconds(input.offset, sample_rate);
                        builder = builder.add_input(Box::new(child_op), offset);
                    }

                    Box::new(builder.build())
                }
            };

            let op_duration = op.duration().unwrap_or(SampleTime::new(0));
            let op_duration_seconds = op_duration.to_seconds(sample_rate);

            // Calculate timeline positions
            let start = SampleTime::from_seconds(op_request.start_time, sample_rate);
            let end = if let Some(end_time) = op_request.end_time {
                SampleTime::from_seconds(end_time, sample_rate)
            } else {
                start + op_duration
            };

            // Schedule the operation
            let op_id = graph.schedule_op(op, start, end).map_err(|e| {
                format!(
                    "Failed to schedule operation '{}': {:?}",
                    op_request.name, e
                )
            })?;

            // Apply gain if specified
            if let Some(gain) = op_request.gain {
                graph.timeline.write().unwrap().set_gain(op_id, gain);
            }

            op_ids.insert(op_request.name.clone(), op_id);

            // Emit progress event
            send_channel_event!(
                on_event,
                OpPlaybackBuildGraphEvent::Progress {
                    timeline_id: timeline_id.clone(),
                    operation_name: op_request.name.clone(),
                    operation_index: index,
                    total_operations: total_graph_ops,
                    duration_seconds: op_duration_seconds,
                }
            );

            if let Ok(logger) = logging_service.lock() {
                log_debug!(
                    logger,
                    LogSystem::Playback,
                    "op_build_graph",
                    &format!(
                        "Added operation '{}' to timeline '{}' (id={:?}, start={:.2}s, end={:.2}s, duration={:.2}s) [LEGACY - NO CACHE]",
                        op_request.name,
                        timeline_id,
                        op_id,
                        op_request.start_time,
                        end.to_seconds(sample_rate),
                        op_duration_seconds
                    )
                );
            }
        }

        let total_duration = graph.duration();
        let total_duration_seconds = total_duration.to_seconds(sample_rate);

        // Create and store the session
        let session = PlaybackSession::new(graph, spec, loop_playback, op_ids);
        state.insert_session(timeline_id.clone(), session);

        // Emit finished event
        send_channel_event!(
            on_event,
            OpPlaybackBuildGraphEvent::Finished {
                timeline_id: timeline_id.clone(),
                operation_count: request.operations.len(),
                total_duration_seconds,
                sample_rate,
                channels,
            }
        );

        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_build_graph",
                &format!(
                    "Timeline '{}' built successfully (LEGACY - NO CACHE): {} operations, {:.2}s total duration",
                    timeline_id, total_graph_ops, total_duration_seconds
                )
            );
        }

        Ok(BuildGraphResponse {
            operation_count: request.operations.len(),
            total_duration_seconds,
            sample_rate,
            channels,
        })
    })
    .await
    .map_err(|e| format!("Failed to execute build graph task: {}", e))?
}

fn load_audio_samples(
    file_path: &str,
    target_sample_rate: u32,
    target_channels: u16,
) -> Result<Vec<f32>, String> {
    use std::fs::File;

    // Use symphonia for audio decoding
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(file_path).extension() {
        hint.with_extension(&ext.to_string_lossy());
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Failed to probe file: {}", e))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No audio track found")?;

    // Get source audio properties
    let source_sample_rate = track
        .codec_params
        .sample_rate
        .ok_or("No sample rate found in audio file")?;

    let source_channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1) as u16;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let track_id = track.id;
    let mut samples: Vec<f32> = Vec::new();

    // Decode all samples first
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(format!("Error reading packet: {}", e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(e) => {
                eprintln!("Error decoding packet: {}", e);
                continue;
            }
        };

        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        sample_buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(sample_buf.samples());
    }

    // Apply channel conversion if needed
    let samples = if source_channels != target_channels {
        eprintln!(
            "Converting audio channels: {} -> {} channels for file: {}",
            source_channels, target_channels, file_path
        );
        convert_channels(&samples, source_channels, target_channels)?
    } else {
        samples
    };

    // Apply sample rate conversion if needed
    let samples = if source_sample_rate != target_sample_rate {
        eprintln!(
            "Resampling audio: {}Hz -> {}Hz for file: {}",
            source_sample_rate, target_sample_rate, file_path
        );
        resample_audio(
            &samples,
            source_sample_rate,
            target_sample_rate,
            target_channels,
        )?
    } else {
        samples
    };

    eprintln!(
        "Loaded audio file: {} ({}Hz, {} channels, {} samples)",
        file_path,
        target_sample_rate,
        target_channels,
        samples.len()
    );

    Ok(samples)
}

/// Convert between different channel counts
fn convert_channels(
    samples: &[f32],
    source_channels: u16,
    target_channels: u16,
) -> Result<Vec<f32>, String> {
    if source_channels == target_channels {
        return Ok(samples.to_vec());
    }

    let frames = samples.len() / source_channels as usize;
    let mut output = Vec::with_capacity(frames * target_channels as usize);

    for frame_idx in 0..frames {
        let source_frame_start = frame_idx * source_channels as usize;

        match (source_channels, target_channels) {
            // Mono to Stereo: duplicate the mono channel
            (1, 2) => {
                let mono_sample = samples[source_frame_start];
                output.push(mono_sample); // Left
                output.push(mono_sample); // Right
            }
            // Stereo to Mono: average left and right channels
            (2, 1) => {
                let left = samples[source_frame_start];
                let right = samples[source_frame_start + 1];
                let mono = (left + right) * 0.5;
                output.push(mono);
            }
            // Multi-channel to Stereo: downmix by averaging all channels
            (src, 2) if src > 2 => {
                let mut sum = 0.0;
                for ch in 0..src {
                    sum += samples[source_frame_start + ch as usize];
                }
                let avg = sum / src as f32;
                output.push(avg); // Left
                output.push(avg); // Right
            }
            // Multi-channel to Mono: downmix by averaging all channels
            (src, 1) if src > 1 => {
                let mut sum = 0.0;
                for ch in 0..src {
                    sum += samples[source_frame_start + ch as usize];
                }
                let avg = sum / src as f32;
                output.push(avg);
            }
            // Unsupported conversion
            (src, tgt) => {
                return Err(format!(
                    "Unsupported channel conversion: {} -> {} channels",
                    src, tgt
                ));
            }
        }
    }

    Ok(output)
}

/// Simple linear interpolation resampler
fn resample_audio(
    samples: &[f32],
    source_rate: u32,
    target_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, String> {
    if source_rate == target_rate {
        return Ok(samples.to_vec());
    }

    let source_frames = samples.len() / channels as usize;
    let ratio = target_rate as f64 / source_rate as f64;
    let target_frames = (source_frames as f64 * ratio).ceil() as usize;

    let mut output = Vec::with_capacity(target_frames * channels as usize);

    for target_frame in 0..target_frames {
        let source_pos = target_frame as f64 / ratio;
        let source_frame = source_pos.floor() as usize;
        let frac = source_pos - source_frame as f64;

        for ch in 0..channels {
            let ch_idx = ch as usize;

            // Get current sample
            let current_idx = source_frame * channels as usize + ch_idx;
            let current_sample = if current_idx < samples.len() {
                samples[current_idx]
            } else {
                0.0 // Pad with silence if beyond end
            };

            // Get next sample for interpolation
            let next_idx = (source_frame + 1) * channels as usize + ch_idx;
            let next_sample = if next_idx < samples.len() {
                samples[next_idx]
            } else {
                current_sample // Use current if no next sample
            };

            // Linear interpolation
            let interpolated = current_sample + (next_sample - current_sample) * frac as f32;
            output.push(interpolated);
        }
    }

    Ok(output)
}
/// Start playback of a specific timeline
#[tauri::command]
pub fn op_playback_play(
    timeline_id: TimelineId,
    start_seconds: Option<f64>,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    eprintln!(
        "🎬 [op_playback_play] ENTER timeline='{}', start_seconds={:?}",
        timeline_id, start_seconds
    );

    let session = state
        .get_session(&timeline_id)
        .ok_or(format!("Timeline '{}' not found", timeline_id))?;

    let loop_playback = *session.loop_playback.lock().unwrap();
    let spec = session.spec;
    let graph = session.graph.clone();
    let total_duration = session.duration_seconds();
    let op_count = session.op_ids.len();
    let op_names: Vec<String> = session.op_ids.keys().cloned().collect();

    // Diagnostic: check if graph actually has content
    let graph_duration_samples = graph.duration().samples();
    let graph_is_empty = graph.is_empty();
    let timeline_events_count = graph.timeline.read().unwrap().len();
    let registry_ops_count = graph.registry.read().unwrap().len();
    eprintln!(
        "🎬 [op_playback_play] Graph diagnostics: duration_samples={}, duration_secs={:.3}, is_empty={}, timeline_events={}, registry_ops={}, session_op_count={}, op_names=[{}]",
        graph_duration_samples, total_duration, graph_is_empty, timeline_events_count, registry_ops_count, op_count, op_names.join(", ")
    );

    drop(session); // Release the reference

    if graph_is_empty || graph_duration_samples == 0 {
        eprintln!("🎬 [op_playback_play] WARNING: Graph is empty or has zero duration! No audio will play.");
    }

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_play",
            &format!(
                "Starting playback for timeline '{}': {} operations [{}], {:.2}s duration, start={:?}s, loop={}",
                timeline_id, op_count, op_names.join(", "), total_duration, start_seconds, loop_playback
            )
        );
    }

    // Stop any current playback
    eprintln!("🎬 [op_playback_play] Stopping current playback...");
    stop_current_playback(&state);
    eprintln!(
        "🎬 [op_playback_play] Current playback stopped. is_playing={}, is_paused={}",
        state.is_playing.load(Ordering::Relaxed),
        state.is_paused.load(Ordering::Relaxed)
    );

    // Determine start position
    let session_ref = state
        .get_session(&timeline_id)
        .ok_or(format!("Timeline '{}' not found", timeline_id))?;

    let start_position = if let Some(start) = start_seconds {
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_play",
                &format!("Using explicit start position: {:.2}s", start)
            );
        }
        *session_ref.seek_seconds.lock().unwrap() = start;
        *session_ref.progress.lock().unwrap() = (start / total_duration).clamp(0.0, 1.0) as f32;
        SampleTime::from_seconds(start, spec.sample_rate)
    } else {
        // Resume from current progress
        let seek_seconds = *session_ref.seek_seconds.lock().unwrap() as f64;
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_play",
                &format!("Resuming from current position: {:.2}s", seek_seconds)
            );
        }
        SampleTime::from_seconds(seek_seconds, spec.sample_rate)
    };
    drop(session_ref);

    eprintln!(
        "🎬 [op_playback_play] Start position: {:?} samples ({:.3}s)",
        start_position,
        start_position.to_seconds(spec.sample_rate)
    );

    // Set this timeline as active
    state.set_active_timeline(Some(timeline_id.clone()));
    state.is_playing.store(true, Ordering::Relaxed);
    state.is_paused.store(false, Ordering::Relaxed);

    eprintln!("🎬 [op_playback_play] State set: active_timeline='{}', is_playing=true, is_paused=false. Spawning playback thread...", timeline_id);

    // Clone what we need for the playback thread
    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    let timeline_id_clone = timeline_id.clone();

    thread::spawn(move || {
        eprintln!(
            "🎬 [op_playback_play] Playback thread started for timeline '{}'",
            timeline_id_clone
        );
        start_playback_from_position(
            state_clone,
            app_clone,
            timeline_id_clone.clone(),
            graph,
            spec,
            loop_playback,
            start_position,
            false, // not paused
        );
        eprintln!("🎬 [op_playback_play] Playback thread for '{}' returned from start_playback_from_position", timeline_id_clone);
    });

    eprintln!(
        "🎬 [op_playback_play] EXIT - playback thread spawned for timeline '{}'",
        timeline_id
    );
    Ok(())
}

/// Pause playback of the currently active timeline
#[tauri::command]
pub fn op_playback_pause(
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let active_timeline = state.get_active_timeline();

    if let Some(timeline_id) = active_timeline {
        if let Some(session) = state.get_session(&timeline_id) {
            let progress = *session.progress.lock().unwrap();
            let total_duration = session.duration_seconds();
            let current_position = progress as f64 * total_duration;
            let op_names: Vec<String> = session.op_ids.keys().cloned().collect();
            drop(session);

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "op_pause",
                    &format!(
                        "Pausing timeline '{}' [{}] at {:.2}s (progress: {:.1}%)",
                        timeline_id,
                        op_names.join(", "),
                        current_position,
                        progress * 100.0
                    )
                );
            }
        }
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        sink.pause();
        state.is_paused.store(true, Ordering::Relaxed);

        // Emit current progress for the active timeline
        if let Some(timeline_id) = state.get_active_timeline() {
            if let Some(session) = state.get_session(&timeline_id) {
                let progress = *session.progress.lock().unwrap();
                emit_logged!(
                    app,
                    "op-timeline-progress",
                    serde_json::json!({
                        "timelineId": timeline_id,
                        "progress": progress
                    })
                );
            }
        }
    }

    Ok(())
}

/// Resume playback of the currently active timeline
#[tauri::command]
pub fn op_playback_resume(
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let active_timeline = state.get_active_timeline();

    if let Some(timeline_id) = active_timeline {
        if let Some(session) = state.get_session(&timeline_id) {
            let progress = *session.progress.lock().unwrap();
            let total_duration = session.duration_seconds();
            let current_position = progress as f64 * total_duration;
            let op_names: Vec<String> = session.op_ids.keys().cloned().collect();
            drop(session);

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "op_resume",
                    &format!(
                        "Resuming timeline '{}' [{}] from {:.2}s (progress: {:.1}%)",
                        timeline_id,
                        op_names.join(", "),
                        current_position,
                        progress * 100.0
                    )
                );
            }
        }
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_resume",
                "Sink found, calling sink.play()"
            );
        }

        sink.play();
        state.is_paused.store(false, Ordering::Relaxed);

        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_resume",
                "Resume completed - sink playing, is_paused=false"
            );
        }
    } else {
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_resume",
                "No sink found - cannot resume"
            );
        }
        return Err("No active playback to resume".to_string());
    }

    Ok(())
}

/// Stop playback and reset progress for all timelines
#[tauri::command]
pub fn op_playback_stop(
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let active_timeline = state.get_active_timeline();

    if let Some(timeline_id) = active_timeline {
        if let Some(session) = state.get_session(&timeline_id) {
            let progress = *session.progress.lock().unwrap();
            let op_names: Vec<String> = session.op_ids.keys().cloned().collect();

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "op_stop",
                    &format!(
                        "Stopping timeline '{}' [{}] (was at {:.1}%)",
                        timeline_id,
                        op_names.join(", "),
                        progress * 100.0
                    )
                );
            }

            // Reset progress for this timeline
            *session.progress.lock().unwrap() = 0.0;
            *session.seek_seconds.lock().unwrap() = 0.0;
        }
    }

    stop_current_playback(&state);
    state.set_active_timeline(None);

    emit_logged!(
        app,
        "op-timeline-progress",
        serde_json::json!({
            "timelineId": null,
            "progress": 0.0
        })
    );

    Ok(())
}
/// Seek to a position in a specific timeline
#[tauri::command]
pub fn op_playback_seek(
    timeline_id: TimelineId,
    position_seconds: f64,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let session = state
        .get_session(&timeline_id)
        .ok_or(format!("Timeline '{}' not found", timeline_id))?;

    let total_duration = session.duration_seconds();
    let op_names: Vec<String> = session.op_ids.keys().cloned().collect();
    let spec = session.spec;
    let graph = session.graph.clone();
    let loop_playback = *session.loop_playback.lock().unwrap();

    // Calculate and update progress
    let progress = (position_seconds / total_duration).clamp(0.0, 1.0) as f32;

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_seek",
            &format!(
                "Seeking to {:.2}s / {:.2}s ({:.1}%) in timeline '{}' [{}]",
                position_seconds,
                total_duration,
                progress * 100.0,
                timeline_id,
                op_names.join(", ")
            )
        );
    }

    *session.progress.lock().unwrap() = progress;
    *session.seek_seconds.lock().unwrap() = position_seconds;
    drop(session);

    // Emit progress
    emit_logged!(
        app,
        "op-timeline-progress",
        serde_json::json!({
            "timelineId": timeline_id,
            "progress": progress
        })
    );

    // If this is the currently active timeline and playback is active (playing or paused),
    // we need to handle seeking. When paused, we still restart playback so that when
    // the user resumes, it plays from the new seek position.
    let is_active_and_playing = state.get_active_timeline().as_ref() == Some(&timeline_id)
        && state.is_playing.load(Ordering::Relaxed);

    if is_active_and_playing {
        let seek_duration = Duration::from_secs_f64(position_seconds);

        // Try to seek on the current sink's source
        let sink = state.sink.lock().unwrap();
        let seek_supported = if let Some(ref sink) = *sink {
            match sink.try_seek(seek_duration) {
                Ok(_) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Playback,
                            "op_seek",
                            &format!(
                                "Successfully seeked timeline '{}' to {:.2}s using try_seek",
                                timeline_id, position_seconds
                            )
                        );
                    }
                    true
                }
                Err(e) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Playback,
                            "op_seek",
                            &format!(
                                "Seek not supported by source ({}). Restarting timeline '{}' playback from {:.2}s.",
                                e, timeline_id, position_seconds
                            )
                        );
                    }
                    false
                }
            }
        } else {
            false
        };

        // If seeking wasn't supported, restart playback from new position
        if !seek_supported {
            drop(sink); // Release the lock

            let was_paused = state.is_paused.load(Ordering::Relaxed);
            stop_current_playback(&state);

            state.is_playing.store(true, Ordering::Relaxed);
            state.is_paused.store(was_paused, Ordering::Relaxed);

            let state_clone = state.inner().clone();
            let app_clone = app.clone();
            let seek_position_time = SampleTime::from_seconds(position_seconds, spec.sample_rate);

            thread::spawn(move || {
                start_playback_from_position(
                    state_clone,
                    app_clone,
                    timeline_id,
                    graph,
                    spec,
                    loop_playback,
                    seek_position_time,
                    was_paused,
                );
            });
        }
    }

    Ok(())
}

/// Helper function to start playback from a specific position
///
/// IMPORTANT: The OutputStream MUST stay alive for the entire duration of playback.
/// If it is dropped, rodio immediately stops all audio output. Because OutputStream
/// is NOT Send (cannot be moved between threads), we keep it alive on the CURRENT
/// thread and run the progress loop here as well. The caller is responsible for
/// calling this from a dedicated thread (not the main thread).
#[allow(clippy::too_many_arguments)]
fn start_playback_from_position(
    state: Arc<AppTimelinePlaybackState>,
    app: AppHandle,
    timeline_id: TimelineId,
    graph: Arc<PlaybackGraph>,
    spec: AudioSpec,
    loop_playback: bool,
    position: SampleTime,
    was_paused: bool,
) {
    eprintln!(
        "🔊 [start_playback] ENTER timeline='{}', position={} samples ({:.3}s), loop={}, paused={}, spec={}Hz/{}ch",
        timeline_id, position.samples(), position.to_seconds(spec.sample_rate), loop_playback, was_paused, spec.sample_rate, spec.channels
    );

    // Diagnostic: verify graph has content at the start position
    {
        let timeline_lock = graph.timeline.read().unwrap();
        let active_events = timeline_lock.get_active_events(position);
        let total_events = timeline_lock.len();
        let duration = timeline_lock.duration();
        eprintln!(
            "🔊 [start_playback] Graph state: total_events={}, active_events_at_position={}, total_duration={} samples ({:.3}s)",
            total_events, active_events.len(), duration.samples(), duration.to_seconds(spec.sample_rate)
        );
        for (i, evt) in active_events.iter().enumerate() {
            eprintln!(
                "🔊 [start_playback]   Active event[{}]: id={:?}, start={}, end={}, gain={:.2}, muted={}, solo={}",
                i, evt.id, evt.start.samples(), evt.end.samples(), evt.gain, evt.muted, evt.solo
            );
        }
        if active_events.is_empty() && total_events > 0 {
            eprintln!("🔊 [start_playback] WARNING: No active events at start position! Listing all events:");
            for (i, evt) in timeline_lock.events().iter().enumerate() {
                eprintln!(
                    "🔊 [start_playback]   Event[{}]: id={:?}, start={} ({:.3}s), end={} ({:.3}s), gain={:.2}, muted={}",
                    i, evt.id, evt.start.samples(), evt.start.to_seconds(spec.sample_rate),
                    evt.end.samples(), evt.end.to_seconds(spec.sample_rate), evt.gain, evt.muted
                );
            }
        }
    }

    // Create audio output - OutputStream is NOT Send, so it must stay on THIS thread
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(output) => {
            eprintln!(
                "🔊 [start_playback] OutputStream created successfully on thread {:?}",
                thread::current().id()
            );
            output
        }
        Err(e) => {
            eprintln!(
                "🔊 [start_playback] ERROR creating audio output stream: {}",
                e
            );
            state.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    let sink = match Sink::try_new(&stream_handle) {
        Ok(sink) => {
            eprintln!("🔊 [start_playback] Sink created successfully");
            Arc::new(sink)
        }
        Err(e) => {
            eprintln!("🔊 [start_playback] ERROR creating sink: {}", e);
            state.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    // Create timeline source from position
    let source = TimelineSourceBuilder::new()
        .spec(spec)
        .looping(loop_playback)
        .start_position(position)
        .build(graph.clone());

    eprintln!(
        "🔊 [start_playback] TimelineSource built: position={:.3}s, finished={}, duration_samples={}",
        source.position_seconds(), source.is_finished(), source.duration_samples().samples()
    );

    // Diagnostic: peek at the first few samples to verify we're producing audio data
    {
        let mut test_source = TimelineSourceBuilder::new()
            .spec(spec)
            .looping(loop_playback)
            .start_position(position)
            .build(graph.clone());

        let mut sample_count = 0;
        let mut non_zero_count = 0;
        let mut max_abs_sample: f32 = 0.0;
        let peek_count = 1024; // Check first 1024 samples

        for _ in 0..peek_count {
            if let Some(sample) = test_source.next() {
                sample_count += 1;
                if sample.abs() > 0.0001 {
                    non_zero_count += 1;
                }
                if sample.abs() > max_abs_sample {
                    max_abs_sample = sample.abs();
                }
            } else {
                break;
            }
        }
        eprintln!(
            "🔊 [start_playback] Sample peek: checked {} samples, {} non-zero, max_abs={:.6}",
            sample_count, non_zero_count, max_abs_sample
        );
        if non_zero_count == 0 {
            eprintln!("🔊 [start_playback] ⚠️ WARNING: All peeked samples are ZERO/SILENT! Audio source may be empty or broken.");
        }
    }

    sink.append(source);
    sink.set_volume(1.0);

    if was_paused {
        eprintln!("🔊 [start_playback] Starting in PAUSED state");
        sink.pause();
    } else {
        eprintln!("🔊 [start_playback] Starting PLAYBACK - calling sink.play()");
        sink.play();
    }

    eprintln!(
        "🔊 [start_playback] Sink state after start: empty={}, volume={:.2}, is_paused={}",
        sink.empty(),
        sink.volume(),
        sink.is_paused()
    );

    // Store the sink in shared state so other commands (pause/resume/stop) can control it
    *state.sink.lock().unwrap() = Some(Arc::clone(&sink));

    eprintln!(
        "🔊 [start_playback] Sink stored in state. Running progress loop on THIS thread (OutputStream stays alive here)..."
    );

    // Run the progress loop on THIS thread (not a new one!)
    // This is critical because OutputStream is NOT Send and must remain on the thread
    // where it was created. This thread will block until playback is done.
    let graph_duration_seconds = graph.duration().to_seconds(spec.sample_rate);
    eprintln!(
        "🔊 [start_playback] About to run progress loop with: total_duration={:.3}s, start_position={:.3}s",
        graph_duration_seconds, position.to_seconds(spec.sample_rate)
    );
    run_progress_loop(&state, &timeline_id, &app);

    eprintln!("🔊 [start_playback] EXIT - progress loop ended for timeline '{}'. OutputStream will be dropped now.", timeline_id);
    // `_stream` (OutputStream) is dropped here when this function returns
}

/// Run the progress tracking loop for a timeline (blocks the current thread)
///
/// This runs on the same thread as the OutputStream to keep it alive.
///
/// IMPORTANT: Progress is calculated as:
///   current_position = initial_seek_offset + wall_clock_elapsed
///   progress = current_position / total_duration
///
/// We capture the initial seek offset ONCE at loop start and never write it back
/// during iteration. Only `progress` is updated in the session so that pause/stop
/// can read the current progress. When pausing, we snapshot the current position
/// so that resume can pick up from there.
fn run_progress_loop(
    state: &Arc<AppTimelinePlaybackState>,
    timeline_id: &TimelineId,
    app: &AppHandle,
) {
    eprintln!(
        "🔊 [progress_loop] Started for timeline '{}'. OutputStream is alive on thread {:?}.",
        timeline_id,
        thread::current().id()
    );

    // Capture the initial seek offset ONCE - this is the position we started from
    let initial_seek_offset: f32 = if let Some(session) = state.get_session(timeline_id) {
        *session.seek_seconds.lock().unwrap() as f32
    } else {
        eprintln!(
            "🔊 [progress_loop] session not found at start for timeline '{}'",
            timeline_id
        );
        return;
    };

    eprintln!(
        "🔊 [progress_loop] Initial seek offset: {:.3}s",
        initial_seek_offset
    );

    let mut tracking_start = Instant::now();
    let mut pause_start: Option<Instant> = None;
    let mut total_pause_duration = Duration::from_secs(0);
    // Accumulated playing time (excluding pauses) since tracking_start was last set.
    // When we resume from pause, we snapshot the accumulated position into accumulated_before_pause
    // and reset tracking_start.
    let mut accumulated_before_pause: f32 = 0.0;
    let mut loop_iteration: u64 = 0;
    let mut last_logged_iteration: u64 = 0;
    let mut first_progress_update = true;

    loop {
        loop_iteration += 1;

        // Log every ~1 second (60 iterations at 16ms sleep)
        if loop_iteration % 60 == 1 {
            let sink_state = state.sink.lock().unwrap();
            let sink_info = if let Some(ref s) = *sink_state {
                format!(
                    "empty={}, volume={:.2}, paused={}",
                    s.empty(),
                    s.volume(),
                    s.is_paused()
                )
            } else {
                "NONE (sink removed from state!)".to_string()
            };
            drop(sink_state);
            eprintln!(
                "🔊 [progress_loop] iter={} timeline='{}': is_playing={}, is_paused={}, sink=[{}]",
                loop_iteration,
                timeline_id,
                state.is_playing.load(Ordering::Relaxed),
                state.is_paused.load(Ordering::Relaxed),
                sink_info
            );
        }

        // Check if we should stop
        if !state.is_playing.load(Ordering::Relaxed) {
            eprintln!(
                "🔊 [progress_loop] is_playing=false, breaking for timeline '{}' at iter={}",
                timeline_id, loop_iteration
            );
            break;
        }

        // Check if this timeline is still the active one
        if state.get_active_timeline().as_ref() != Some(timeline_id) {
            eprintln!("🔊 [progress_loop] timeline '{}' is no longer active (active={:?}), breaking at iter={}", 
                timeline_id, state.get_active_timeline(), loop_iteration);
            break;
        }

        // Get session info
        let session = if let Some(s) = state.get_session(timeline_id) {
            s
        } else {
            eprintln!(
                "🔊 [progress_loop] session not found for timeline '{}', breaking at iter={}",
                timeline_id, loop_iteration
            );
            break;
        };

        let total_duration_seconds = session.duration_seconds();
        let loop_playback = *session.loop_playback.lock().unwrap();
        drop(session);

        // Check if sink is empty and not looping
        {
            let sink_guard = state.sink.lock().unwrap();
            let should_break = match *sink_guard {
                Some(ref sink) => {
                    if sink.empty() && !loop_playback {
                        eprintln!("🔊 [progress_loop] Sink empty and not looping for timeline '{}', breaking at iter={}", timeline_id, loop_iteration);
                        true
                    } else {
                        false
                    }
                }
                None => {
                    eprintln!("🔊 [progress_loop] Sink is NONE in state for timeline '{}', breaking at iter={}", timeline_id, loop_iteration);
                    true
                }
            };
            drop(sink_guard);
            if should_break {
                break;
            }
        }

        if state.is_paused.load(Ordering::Relaxed) {
            // Mark pause start if we just entered pause state
            if pause_start.is_none() {
                // Snapshot how much playing time we've accumulated before this pause
                let elapsed_this_segment = tracking_start.elapsed().as_secs_f32();
                accumulated_before_pause += elapsed_this_segment;
                eprintln!(
                    "🔊 [progress_loop] PAUSED at iter={}, elapsed_this_segment={:.3}s, total_accumulated={:.3}s",
                    loop_iteration, elapsed_this_segment, accumulated_before_pause
                );
                pause_start = Some(Instant::now());
            }
            thread::sleep(Duration::from_millis(50));
            continue;
        } else if let Some(pause_started_at) = pause_start.take() {
            // We just resumed from pause - reset the wall clock
            let pause_duration = pause_started_at.elapsed();
            total_pause_duration += pause_duration;

            // Check if seek happened while paused - if seek_seconds changed, use it
            if let Some(session) = state.get_session(timeline_id) {
                let current_seek_seconds = *session.seek_seconds.lock().unwrap() as f32;
                let expected_position = initial_seek_offset + accumulated_before_pause;

                // If seek_seconds differs significantly from our tracked position, a seek happened
                if (current_seek_seconds - expected_position).abs() > 0.01 {
                    eprintln!(
                        "🔊 [progress_loop] SEEK DETECTED during pause: expected={:.3}s, actual={:.3}s, adjusting accumulated_before_pause",
                        expected_position, current_seek_seconds
                    );
                    // Update accumulated_before_pause to reflect the seek position
                    // The new position is: initial_seek_offset + new_accumulated = current_seek_seconds
                    // So: new_accumulated = current_seek_seconds - initial_seek_offset
                    accumulated_before_pause = current_seek_seconds - initial_seek_offset;
                }
            }

            eprintln!(
                "🔊 [progress_loop] RESUMED at iter={}, pause_duration={:.3}s, total_pause_duration={:.3}s, accumulated_before_pause={:.3}s",
                loop_iteration, pause_duration.as_secs_f32(), total_pause_duration.as_secs_f32(), accumulated_before_pause
            );
            // Reset tracking_start so elapsed() measures from resume point
            tracking_start = Instant::now();
            first_progress_update = true;
        }

        // Calculate current position:
        //   current_position = initial_seek_offset + accumulated_playing_time
        // where accumulated_playing_time = time_accumulated_before_pauses + time_since_last_resume
        let elapsed_this_segment = tracking_start.elapsed().as_secs_f32();
        let total_playing_time = accumulated_before_pause + elapsed_this_segment;
        let current_position = initial_seek_offset + total_playing_time;

        // Calculate progress (handle looping)
        let progress = if total_duration_seconds > 0.0 {
            if loop_playback {
                (current_position % total_duration_seconds as f32) / total_duration_seconds as f32
            } else {
                (current_position / total_duration_seconds as f32).min(1.0)
            }
        } else {
            0.0
        };

        // Log detailed progress info periodically or on first update
        if first_progress_update || loop_iteration - last_logged_iteration >= 60 {
            eprintln!(
                "🔊 [progress_loop] PROGRESS @ iter={}: \n  \
                initial_seek={:.3}s, accumulated_before_pause={:.3}s, elapsed_this_segment={:.3}s, \n  \
                total_playing_time={:.3}s, current_position={:.3}s, \n  \
                total_duration={:.3}s, progress={:.4} ({:.1}%)",
                loop_iteration,
                initial_seek_offset, accumulated_before_pause, elapsed_this_segment,
                total_playing_time, current_position,
                total_duration_seconds, progress, progress * 100.0,
            );

            last_logged_iteration = loop_iteration;
            first_progress_update = false;
        }

        // Update session state:
        // - progress: for UI display and pause/stop to read
        // - seek_seconds: so that if playback is stopped and restarted, it can resume from here
        if let Some(session) = state.get_session(timeline_id) {
            *session.progress.lock().unwrap() = progress;
            *session.seek_seconds.lock().unwrap() = current_position as f64;
        }

        emit_logged!(
            app,
            "op-timeline-progress",
            serde_json::json!({
                "timelineId": timeline_id,
                "progress": progress
            })
        );

        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    // On exit, store the final position so stop/pause can report it correctly
    let final_elapsed = tracking_start.elapsed().as_secs_f32();
    let final_position = initial_seek_offset + accumulated_before_pause + final_elapsed;
    eprintln!(
        "🔊 [progress_loop] Ended for timeline '{}' at iter={}. Final position: {:.3}s. Total pause duration: {:.3}s",
        timeline_id, loop_iteration, final_position, total_pause_duration.as_secs_f32()
    );

    if let Some(session) = state.get_session(timeline_id) {
        *session.seek_seconds.lock().unwrap() = final_position as f64;
    }

    state.is_playing.store(false, Ordering::Relaxed);
}

/// Get current playback progress for a specific timeline
#[tauri::command]
pub fn op_playback_get_progress(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
) -> Result<f32, String> {
    let session = state
        .get_session(&timeline_id)
        .ok_or(format!("Timeline '{}' not found", timeline_id))?;

    let progress = *session.progress.lock().unwrap();
    Ok(progress)
}

/// Set playback volume for the currently active audio sink
#[tauri::command]
pub fn op_playback_set_volume(
    volume: f32,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let active_timeline = state.get_active_timeline();

    if let Some(timeline_id) = active_timeline {
        if let Some(session) = state.get_session(&timeline_id) {
            let op_names: Vec<String> = session.op_ids.keys().cloned().collect();

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "op_volume",
                    &format!(
                        "Setting volume to {:.2} for timeline '{}' [{}]",
                        volume,
                        timeline_id,
                        op_names.join(", ")
                    )
                );
            }
        }
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        sink.set_volume(volume);
    }

    Ok(())
}

/// Set loop playback mode for a specific timeline
#[tauri::command]
pub fn op_playback_set_loop(
    timeline_id: TimelineId,
    loop_playback: bool,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let session = state
        .sessions
        .get(&timeline_id)
        .ok_or(format!("Timeline '{}' not found", timeline_id))?;

    let op_names: Vec<String> = session.op_ids.keys().cloned().collect();

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_loop",
            &format!(
                "Setting loop mode to {} for timeline '{}' [{}]",
                loop_playback,
                timeline_id,
                op_names.join(", ")
            )
        );
    }

    *session.loop_playback.lock().unwrap() = loop_playback;
    Ok(())
}

/// Remove a specific timeline's playback session
#[tauri::command]
pub fn op_playback_clear_timeline(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    // Stop playback if this timeline is currently active
    if state.get_active_timeline().as_ref() == Some(&timeline_id) {
        stop_current_playback(&state);
        state.set_active_timeline(None);
    }

    if let Some((_, session)) = state.remove_session(&timeline_id) {
        let op_names: Vec<String> = session.op_ids.keys().cloned().collect();
        let total_duration = session.duration_seconds();

        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_clear_timeline",
                &format!(
                    "Cleared timeline '{}' ({} operations [{}], {:.2}s duration)",
                    timeline_id,
                    session.op_ids.len(),
                    op_names.join(", "),
                    total_duration
                )
            );
        }
    } else {
        return Err(format!("Timeline '{}' not found", timeline_id));
    }

    Ok(())
}

/// Clear all timeline playback sessions
#[tauri::command]
pub fn op_playback_clear_all_timelines(
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let timeline_count = state.sessions.len();

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_clear_all",
            &format!("Clearing all {} timeline sessions", timeline_count)
        );
    }

    stop_current_playback(&state);
    state.set_active_timeline(None);
    state.sessions.clear();

    Ok(())
}

// Helper functions

fn stop_current_playback(state: &AppTimelinePlaybackState) {
    eprintln!(
        "🛑 [stop_current_playback] ENTER: is_playing={}, is_paused={}, has_sink={}",
        state.is_playing.load(Ordering::Relaxed),
        state.is_paused.load(Ordering::Relaxed),
        state.sink.lock().unwrap().is_some()
    );
    state.is_playing.store(false, Ordering::Relaxed);
    state.is_paused.store(false, Ordering::Relaxed);

    let mut sink = state.sink.lock().unwrap();
    if let Some(ref s) = *sink {
        eprintln!(
            "🛑 [stop_current_playback] Stopping and clearing sink (empty={}, volume={:.2})",
            s.empty(),
            s.volume()
        );
        s.stop();
        s.clear();
    } else {
        eprintln!("🛑 [stop_current_playback] No sink to stop");
    }
    *sink = None;
    eprintln!("🛑 [stop_current_playback] EXIT");
}

fn count_graph_ops(request: &BuildOpPlaybackGraphRequest) -> Result<usize, String> {
    let mut count = 0;

    for op in &request.operations {
        match op.op_type {
            OpType::Sample => {
                count += 1;
            }
            OpType::Merge => {
                let inputs = op
                    .inputs
                    .as_ref()
                    .ok_or_else(|| format!("Merge operation '{}' must have inputs", op.name))?;

                if inputs.is_empty() {
                    return Err(format!(
                        "Merge operation '{}' must have at least one input",
                        op.name
                    ));
                }

                // The merge op itself
                count += 1;

                // Each input becomes its own SamplePlayableOp
                count += inputs.len();
            }
        }
    }

    Ok(count)
}
