// Operation-based timeline playback commands
//
// This module provides Tauri commands for playing back operations
// using the pull-based playback system.

use crate::logging::{LogSystem, LoggingService};
use crate::playback::op_playback::{
    AudioSpec, PlayableOp, PlaybackGraph, PlaybackOpId, SampleTime, TimelineSourceBuilder,
};
use crate::playback_ops::merge_playback::MergePlaybackOp;
use crate::playback_ops::sample_playback::SamplePlayableOp;
use crate::sample_cache::SampleCacheService;
use crate::{emit_logged, log_debug, log_info, send_channel_event};
use rodio::{OutputStream, Sink};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

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
        operation_count: usize,
    },
    Progress {
        operation_name: String,
        operation_index: usize,
        total_operations: usize,
        duration_seconds: f64,
    },
    Finished {
        operation_count: usize,
        total_duration_seconds: f64,
        sample_rate: u32,
        channels: u16,
    },
}

/// State for operation-based playback
pub struct OpPlaybackState {
    /// The current playback graph
    graph: RwLock<Option<Arc<PlaybackGraph>>>,

    /// Current audio sink
    sink: Mutex<Option<Arc<Sink>>>,

    /// Current playback position (normalized 0.0-1.0)
    progress: Mutex<f32>,

    /// Seek position in seconds (set when seeking during playback)
    seek_position: Mutex<f32>,

    /// Whether playback is paused
    is_paused: AtomicBool,

    /// Whether playback is active
    is_playing: AtomicBool,

    /// Whether to loop playback
    loop_playback: AtomicBool,

    /// Sample rate for playback
    sample_rate: AtomicU64,

    /// Mapping of operation names to their IDs in the current graph
    op_id_map: RwLock<HashMap<String, PlaybackOpId>>,

    /// Audio specification
    spec: RwLock<AudioSpec>,
}

impl OpPlaybackState {
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(None),
            sink: Mutex::new(None),
            progress: Mutex::new(0.0),
            seek_position: Mutex::new(0.0),
            is_paused: AtomicBool::new(false),
            is_playing: AtomicBool::new(false),
            loop_playback: AtomicBool::new(true),
            sample_rate: AtomicU64::new(44100),
            op_id_map: RwLock::new(HashMap::new()),
            spec: RwLock::new(AudioSpec::cd_quality()),
        }
    }

    pub fn set_graph(&self, graph: Arc<PlaybackGraph>) {
        *self.graph.write().unwrap() = Some(graph);
    }

    pub fn get_graph(&self) -> Option<Arc<PlaybackGraph>> {
        self.graph.read().unwrap().clone()
    }

    pub fn clear_graph(&self) {
        *self.graph.write().unwrap() = None;
        self.op_id_map.write().unwrap().clear();
    }
}

impl Default for OpPlaybackState {
    fn default() -> Self {
        Self::new()
    }
}

/// The type of playback operation
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OpType {
    /// A simple sample-based operation (default)
    #[default]
    Sample,
    /// A merge operation that combines multiple inputs
    Merge,
}

/// Child input for a merge operation
#[derive(Debug, Clone, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGraphRequest {
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

/// Build a playback graph from the request
#[tauri::command]
pub async fn op_playback_build_graph(
    request: BuildGraphRequest,
    state: State<'_, Arc<OpPlaybackState>>,
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    on_event: Channel<OpPlaybackBuildGraphEvent>,
) -> Result<BuildGraphResponse, String> {
    let state = state.inner().clone();
    let sample_cache = sample_cache.inner().clone();
    let logging_service = logging_service.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_build_graph",
                &format!(
                    "Building playback graph with {} operations",
                    request.operations.len()
                )
            );
        }
        let total_graph_ops = count_graph_ops(&request)?;
        // Emit started event
        send_channel_event!(
            on_event,
            OpPlaybackBuildGraphEvent::Started {
                operation_count: total_graph_ops
            }
        );

        // Stop any current playback
        stop_current_playback(&state);

        let sample_rate = request.sample_rate.unwrap_or(44100);
        let channels = request.channels.unwrap_or(2);
        let spec = AudioSpec::new(sample_rate, channels);

        // Update state
        *state.spec.write().unwrap() = spec;
        state
            .sample_rate
            .store(sample_rate as u64, Ordering::Relaxed);
        state
            .loop_playback
            .store(request.loop_playback.unwrap_or(true), Ordering::Relaxed);

        // Create new graph
        let graph = Arc::new(PlaybackGraph::new(spec));
        let mut op_id_map = HashMap::new();

        for (index, op_request) in request.operations.iter().enumerate() {
            // Create the playable operation based on type
            let op: Box<dyn PlayableOp> = match op_request.op_type {
                OpType::Sample => {
                    // Get samples (either from file or directly provided)
                    let samples = if let Some(ref samples) = op_request.samples {
                        samples.clone()
                    } else if let Some(ref file_path) = op_request.file_path {
                        // Load samples from file using cache
                        let path = std::path::PathBuf::from(file_path);
                        let buffer = sample_cache.get_or_load(path, sample_rate, channels)?;
                        buffer.data.clone()
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
                            let path = std::path::PathBuf::from(file_path);
                            let buffer = sample_cache.get_or_load(path, sample_rate, channels)?;
                            buffer.data.clone()
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

            op_id_map.insert(op_request.name.clone(), op_id);

            // Emit progress event
            send_channel_event!(
                on_event,
                OpPlaybackBuildGraphEvent::Progress {
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
                        "Added operation '{}' (id={:?}, start={:.2}s, end={:.2}s, duration={:.2}s)",
                        op_request.name,
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

        // Store the graph and ID map
        *state.op_id_map.write().unwrap() = op_id_map;
        state.set_graph(graph);

        // Emit finished event
        send_channel_event!(
            on_event,
            OpPlaybackBuildGraphEvent::Finished {
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
                    "Graph built successfully: {} operations, {:.2}s total duration",
                    total_graph_ops,
                    total_duration_seconds
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

/// Start playback of the current graph
#[tauri::command]
pub fn op_playback_play(
    start_seconds: Option<f64>,
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let graph = state.get_graph().ok_or("No playback graph available")?;
    let spec = *state.spec.read().unwrap();
    let loop_playback = state.loop_playback.load(Ordering::Relaxed);

    // Get operation info for logging
    let op_id_map = state.op_id_map.read().unwrap();
    let op_count = op_id_map.len();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);
    let total_duration = graph.duration().to_seconds(spec.sample_rate);

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_play",
            &format!(
                "Starting operation playback: {} operations [{}], {:.2}s duration, start={:?}s, loop={}",
                op_count, op_names.join(", "), total_duration, start_seconds, loop_playback
            )
        );
    }

    // Stop any current playback
    stop_current_playback(&state);

    // Determine start position
    let start_position = if let Some(start) = start_seconds {
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_play",
                &format!("Using explicit start position: {:.2}s", start)
            );
        }
        SampleTime::from_seconds(start, spec.sample_rate)
    } else {
        // Resume from current progress
        let progress = *state.progress.lock().unwrap();
        let duration = graph.duration();
        let calculated_samples = (duration.samples() as f64 * progress as f64) as u64;
        let calculated_seconds = SampleTime::new(calculated_samples).to_seconds(spec.sample_rate);

        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "op_play",
                &format!(
                    "Resuming from current progress: {:.1}% -> {:.2}s ({} samples)",
                    progress * 100.0,
                    calculated_seconds,
                    calculated_samples
                )
            );
        }

        SampleTime::new(calculated_samples)
    };

    // Clone what we need for the playback thread
    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    let total_duration = graph.duration();

    state.is_playing.store(true, Ordering::Relaxed);
    state.is_paused.store(false, Ordering::Relaxed);

    thread::spawn(move || {
        // Create audio output
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Error creating audio output stream: {}", e);
                state_clone.is_playing.store(false, Ordering::Relaxed);
                return;
            }
        };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(sink) => Arc::new(sink),
            Err(e) => {
                eprintln!("Error creating sink: {}", e);
                state_clone.is_playing.store(false, Ordering::Relaxed);
                return;
            }
        };

        // Create timeline source
        let source = TimelineSourceBuilder::new()
            .spec(spec)
            .looping(loop_playback)
            .start_position(start_position)
            .build(graph.clone()); // Store current position for tracking
        let start_seconds_actual = start_position.to_seconds(spec.sample_rate);
        *state_clone.seek_position.lock().unwrap() = start_seconds_actual as f32;

        sink.append(source);
        sink.set_volume(1.0);
        sink.play();

        // Store the sink
        *state_clone.sink.lock().unwrap() = Some(Arc::clone(&sink));

        // Progress tracking
        let mut tracking_start = Instant::now();
        let mut pause_start: Option<Instant> = None;
        let mut total_pause_duration = Duration::from_secs(0);
        let total_duration_seconds = total_duration.to_seconds(spec.sample_rate);

        loop {
            // Check if we should stop
            if !state_clone.is_playing.load(Ordering::Relaxed) {
                break;
            }

            if sink.empty() && !loop_playback {
                break;
            }
            if state_clone.is_paused.load(Ordering::Relaxed) {
                // Mark pause start if we just entered pause state
                if pause_start.is_none() {
                    pause_start = Some(Instant::now());
                    println!("DEBUG: Entering pause state in playback loop");
                }
                thread::sleep(Duration::from_millis(50));
                continue;
            } else if let Some(pause_started_at) = pause_start.take() {
                // We just resumed from pause
                let pause_duration = pause_started_at.elapsed();
                total_pause_duration += pause_duration;
                println!(
                    "DEBUG: Resuming from pause - pause lasted {:.2}s, total pause time: {:.2}s",
                    pause_duration.as_secs_f32(),
                    total_pause_duration.as_secs_f32()
                );
                // Reset tracking_start to now so we measure from resume point, not from initial play start
                tracking_start = Instant::now();
            }

            // Calculate current position (excluding time spent paused)
            let seek_start = *state_clone.seek_position.lock().unwrap();
            let total_elapsed = tracking_start.elapsed();
            let current_position = seek_start + total_elapsed.as_secs_f32();

            // Calculate progress (handle looping)
            let progress = if total_duration_seconds > 0.0 {
                if loop_playback {
                    (current_position % total_duration_seconds as f32)
                        / total_duration_seconds as f32
                } else {
                    (current_position / total_duration_seconds as f32).min(1.0)
                }
            } else {
                0.0
            };

            // Update state and emit progress
            *state_clone.progress.lock().unwrap() = progress;
            emit_logged!(app_clone, "op-timeline-progress", progress);

            thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }
        state_clone.is_playing.store(false, Ordering::Relaxed);
        println!("DEBUG: Operation playback finished");
    });

    Ok(())
}

/// Pause playback
#[tauri::command]
pub fn op_playback_pause(
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let progress = *state.progress.lock().unwrap();
    let spec = *state.spec.read().unwrap();
    let graph = state.get_graph();

    let current_position = if let Some(ref g) = graph {
        let total_duration = g.duration().to_seconds(spec.sample_rate);
        progress as f64 * total_duration
    } else {
        0.0
    };

    let op_id_map = state.op_id_map.read().unwrap();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_pause",
            &format!(
                "Pausing operation playback [{}] at {:.2}s (progress: {:.1}%)",
                op_names.join(", "),
                current_position,
                progress * 100.0
            )
        );
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        sink.pause();
        state.is_paused.store(true, Ordering::Relaxed);

        // Emit current progress
        let progress = *state.progress.lock().unwrap();
        emit_logged!(app, "op-timeline-progress", progress);
    }

    Ok(())
}

/// Resume playback
#[tauri::command]
pub fn op_playback_resume(
    state: State<'_, Arc<OpPlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let progress = *state.progress.lock().unwrap();
    let spec = *state.spec.read().unwrap();
    let graph = state.get_graph();

    let current_position = if let Some(ref g) = graph {
        let total_duration = g.duration().to_seconds(spec.sample_rate);
        progress as f64 * total_duration
    } else {
        0.0
    };

    let op_id_map = state.op_id_map.read().unwrap();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_resume",
            &format!(
                "Resuming operation playback [{}] from {:.2}s (progress: {:.1}%)",
                op_names.join(", "),
                current_position,
                progress * 100.0
            )
        );
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

/// Stop playback
#[tauri::command]
pub fn op_playback_stop(
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let progress = *state.progress.lock().unwrap();
    let op_id_map = state.op_id_map.read().unwrap();
    let op_count = op_id_map.len();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_stop",
            &format!(
                "Stopping operation playback ({} operations [{}], was at {:.1}%)",
                op_count,
                op_names.join(", "),
                progress * 100.0
            )
        );
    }

    stop_current_playback(&state);
    *state.progress.lock().unwrap() = 0.0;
    emit_logged!(app, "op-timeline-progress", 0.0f32);

    Ok(())
}
/// Seek to a position in the current playback graph
#[tauri::command]
pub fn op_playback_seek(
    position_seconds: f64,
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let graph = state.get_graph().ok_or("No playback graph available")?;
    let spec = *state.spec.read().unwrap();
    let total_duration = graph.duration().to_seconds(spec.sample_rate);

    let op_id_map = state.op_id_map.read().unwrap();
    let op_count = op_id_map.len();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    // Calculate and update progress
    let progress = (position_seconds / total_duration).clamp(0.0, 1.0) as f32;

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_seek",
            &format!(
                "Seeking to {:.2}s / {:.2}s ({:.1}%) in {} operations [{}]",
                position_seconds,
                total_duration,
                progress * 100.0,
                op_count,
                op_names.join(", ")
            )
        );
    }

    *state.progress.lock().unwrap() = progress;
    *state.seek_position.lock().unwrap() = position_seconds as f32;

    // Emit progress
    emit_logged!(app, "op-timeline-progress", progress);

    // If currently playing, try to seek on the active source
    if state.is_playing.load(Ordering::Relaxed) {
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
                                "Successfully seeked to {:.2}s using try_seek",
                                position_seconds
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
                                "Seek not supported by source ({}). Restarting playback from {:.2}s.",
                                e, position_seconds
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
            let loop_playback = state.loop_playback.load(Ordering::Relaxed);
            let seek_position_time = SampleTime::from_seconds(position_seconds, spec.sample_rate);

            thread::spawn(move || {
                start_playback_from_position(
                    state_clone,
                    app_clone,
                    graph.clone(),
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
fn start_playback_from_position(
    state: Arc<OpPlaybackState>,
    app: AppHandle,
    graph: Arc<PlaybackGraph>,
    spec: AudioSpec,
    loop_playback: bool,
    position: SampleTime,
    was_paused: bool,
) {
    // Create audio output
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Error creating audio output stream: {}", e);
            state.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    let sink = match Sink::try_new(&stream_handle) {
        Ok(sink) => Arc::new(sink),
        Err(e) => {
            eprintln!("Error creating sink: {}", e);
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

    sink.append(source);
    sink.set_volume(1.0);

    if was_paused {
        sink.pause();
    } else {
        sink.play();
    }

    *state.sink.lock().unwrap() = Some(Arc::clone(&sink));

    // Progress tracking loop
    let tracking_start = Instant::now();
    let mut pause_start: Option<Instant> = None;
    let mut total_pause_duration = Duration::from_secs(0);
    let total_duration_seconds = graph.duration().to_seconds(spec.sample_rate);

    loop {
        if !state.is_playing.load(Ordering::Relaxed) {
            break;
        }

        if sink.empty() && !loop_playback {
            break;
        }

        if state.is_paused.load(Ordering::Relaxed) {
            if pause_start.is_none() {
                pause_start = Some(Instant::now());
            }
            thread::sleep(Duration::from_millis(50));
            continue;
        } else if let Some(pause_started_at) = pause_start.take() {
            total_pause_duration += pause_started_at.elapsed();
        }

        let seek_start = *state.seek_position.lock().unwrap();
        let total_elapsed = tracking_start.elapsed();
        let active_elapsed = total_elapsed - total_pause_duration;
        let current_position = seek_start + active_elapsed.as_secs_f32();

        let progress = if total_duration_seconds > 0.0 {
            if loop_playback {
                (current_position % total_duration_seconds as f32) / total_duration_seconds as f32
            } else {
                (current_position / total_duration_seconds as f32).min(1.0)
            }
        } else {
            0.0
        };

        *state.progress.lock().unwrap() = progress;
        emit_logged!(app, "op-timeline-progress", progress);

        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    state.is_playing.store(false, Ordering::Relaxed);
}

/// Get current playback progress
#[tauri::command]
pub fn op_playback_get_progress(state: State<'_, Arc<OpPlaybackState>>) -> f32 {
    *state.progress.lock().unwrap()
}

/// Set playback volume
#[tauri::command]
pub fn op_playback_set_volume(
    volume: f32,
    state: State<'_, Arc<OpPlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let op_id_map = state.op_id_map.read().unwrap();
    let op_count = op_id_map.len();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_volume",
            &format!(
                "Setting volume to {:.2} ({} operations [{}])",
                volume,
                op_count,
                op_names.join(", ")
            )
        );
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        sink.set_volume(volume);
    }

    Ok(())
}

/// Set loop playback mode
#[tauri::command]
pub fn op_playback_set_loop(
    loop_playback: bool,
    state: State<'_, Arc<OpPlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let op_id_map = state.op_id_map.read().unwrap();
    let op_count = op_id_map.len();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_loop",
            &format!(
                "Setting loop mode to {} ({} operations [{}])",
                loop_playback,
                op_count,
                op_names.join(", ")
            )
        );
    }

    state.loop_playback.store(loop_playback, Ordering::Relaxed);
    Ok(())
}

/// Clear the current playback graph
#[tauri::command]
pub fn op_playback_clear_graph(
    state: State<'_, Arc<OpPlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    let op_id_map = state.op_id_map.read().unwrap();
    let op_count = op_id_map.len();
    let op_names: Vec<String> = op_id_map.keys().cloned().collect();
    drop(op_id_map);

    let spec = *state.spec.read().unwrap();
    let total_duration = if let Some(ref graph) = state.get_graph() {
        graph.duration().to_seconds(spec.sample_rate)
    } else {
        0.0
    };

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_clear",
            &format!(
                "Clearing playback graph ({} operations [{}], {:.2}s duration)",
                op_count,
                op_names.join(", "),
                total_duration
            )
        );
    }

    stop_current_playback(&state);
    state.clear_graph();
    *state.progress.lock().unwrap() = 0.0;

    Ok(())
}

// Helper functions

fn stop_current_playback(state: &OpPlaybackState) {
    state.is_playing.store(false, Ordering::Relaxed);
    state.is_paused.store(false, Ordering::Relaxed);

    let mut sink = state.sink.lock().unwrap();
    if let Some(ref s) = *sink {
        s.stop();
        s.clear();
    }
    *sink = None;
}

fn count_graph_ops(request: &BuildGraphRequest) -> Result<usize, String> {
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
