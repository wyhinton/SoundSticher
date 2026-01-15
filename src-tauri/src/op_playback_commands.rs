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
use crate::{emit_logged, log_debug, log_info};
use rodio::{OutputStream, Sink};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

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
pub fn op_playback_build_graph(
    request: BuildGraphRequest,
    state: State<'_, Arc<OpPlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<BuildGraphResponse, String> {
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

    for op_request in &request.operations {
        // Create the playable operation based on type
        let op: Box<dyn PlayableOp> = match op_request.op_type {
            OpType::Sample => {
                // Get samples (either from file or directly provided)
                let samples = if let Some(ref samples) = op_request.samples {
                    samples.clone()
                } else if let Some(ref file_path) = op_request.file_path {
                    // Load samples from file
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
                        load_audio_samples(file_path, sample_rate, channels)?
                    } else {
                        return Err(format!(
                            "Merge input {} in operation '{}' must have either 'samples' or 'filePath'",
                            i, op_request.name
                        ));
                    };

                    // TODO: Per-input gain could be supported by wrapping in a GainOp
                    // For now, gain is applied at the merge operation level
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

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_build_graph",
            &format!(
                "Graph built successfully: {} operations, {:.2}s total duration",
                request.operations.len(),
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
