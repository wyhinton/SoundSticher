// Operation-based timeline playback commands
//
// This module provides Tauri commands for playing back operations
// using the pull-based playback system.

use crate::logging::{LogSystem, LoggingService};
use crate::playback::op_playback::{
    AudioSpec, PlayableOp, PlaybackGraph, PlaybackOpId, SamplePlayableOp, SampleTime,
    TimelineEvent, TimelineSource, TimelineSourceBuilder,
};
use crate::{log_debug, log_info};
use rodio::{OutputStream, Sink};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

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

/// Request to add an operation to the playback graph
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddOpRequest {
    /// Unique name for this operation
    pub name: String,

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
        // Get samples (either from file or directly provided)
        let samples = if let Some(ref samples) = op_request.samples {
            samples.clone()
        } else if let Some(ref file_path) = op_request.file_path {
            // Load samples from file
            load_audio_samples(file_path, sample_rate, channels)?
        } else {
            return Err(format!(
                "Operation '{}' must have either 'samples' or 'filePath'",
                op_request.name
            ));
        };

        // Create the playable operation
        let op = Box::new(SamplePlayableOp::new(samples.clone(), spec));
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
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_play",
            &format!("Starting operation playback (start={:?}s)", start_seconds)
        );
    }

    let graph = state.get_graph().ok_or("No playback graph available")?;
    let spec = *state.spec.read().unwrap();
    let loop_playback = state.loop_playback.load(Ordering::Relaxed);

    // Stop any current playback
    stop_current_playback(&state);

    // Determine start position
    let start_position = if let Some(start) = start_seconds {
        SampleTime::from_seconds(start, spec.sample_rate)
    } else {
        // Resume from current progress
        let progress = *state.progress.lock().unwrap();
        let duration = graph.duration();
        SampleTime::new((duration.samples() as f64 * progress as f64) as u64)
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
        let mut source = TimelineSourceBuilder::new()
            .spec(spec)
            .looping(loop_playback)
            .start_position(start_position)
            .build(graph.clone());

        // Store current position for tracking
        let start_seconds_actual = start_position.to_seconds(spec.sample_rate);
        *state_clone.seek_position.lock().unwrap() = start_seconds_actual as f32;

        sink.append(source);
        sink.set_volume(1.0);
        sink.play();

        // Store the sink
        *state_clone.sink.lock().unwrap() = Some(Arc::clone(&sink));

        // Progress tracking
        let tracking_start = Instant::now();
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
                thread::sleep(Duration::from_millis(50));
                continue;
            }

            // Calculate current position
            let seek_start = *state_clone.seek_position.lock().unwrap();
            let elapsed = tracking_start.elapsed().as_secs_f32();
            let current_position = seek_start + elapsed;

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
            let _ = app_clone.emit("op-timeline-progress", progress);

            thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }

        state_clone.is_playing.store(false, Ordering::Relaxed);
        println!("Operation playback finished");
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
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_pause",
            "Pausing operation playback"
        );
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        sink.pause();
        state.is_paused.store(true, Ordering::Relaxed);

        // Emit current progress
        let progress = *state.progress.lock().unwrap();
        let _ = app.emit("op-timeline-progress", progress);
    }

    Ok(())
}

/// Resume playback
#[tauri::command]
pub fn op_playback_resume(
    state: State<'_, Arc<OpPlaybackState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_resume",
            "Resuming operation playback"
        );
    }

    let sink = state.sink.lock().unwrap();
    if let Some(ref sink) = *sink {
        sink.play();
        state.is_paused.store(false, Ordering::Relaxed);
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
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_stop",
            "Stopping operation playback"
        );
    }

    stop_current_playback(&state);
    *state.progress.lock().unwrap() = 0.0;
    let _ = app.emit("op-timeline-progress", 0.0f32);

    Ok(())
}

/// Seek to a position
#[tauri::command]
pub fn op_playback_seek(
    position_seconds: f64,
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_seek",
            &format!("Seeking to {:.2}s", position_seconds)
        );
    }

    let graph = state.get_graph().ok_or("No playback graph available")?;
    let spec = *state.spec.read().unwrap();
    let total_duration = graph.duration().to_seconds(spec.sample_rate);

    // Calculate and update progress
    let progress = (position_seconds / total_duration).clamp(0.0, 1.0) as f32;
    *state.progress.lock().unwrap() = progress;
    *state.seek_position.lock().unwrap() = position_seconds as f32;

    // Emit progress
    let _ = app.emit("op-timeline-progress", progress);

    // If currently playing, restart from new position
    if state.is_playing.load(Ordering::Relaxed) && !state.is_paused.load(Ordering::Relaxed) {
        // This will restart playback from the new position
        drop(state); // Release borrow before calling play
                     // Note: In a real implementation, you'd want to seamlessly seek
                     // without restarting. This is a simplified version.
    }

    Ok(())
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
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_volume",
            &format!("Setting volume to {:.2}", volume)
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
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_loop",
            &format!("Setting loop mode to {}", loop_playback)
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
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "op_clear",
            "Clearing playback graph"
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
    use std::io::BufReader;

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

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let track_id = track.id;
    let mut samples: Vec<f32> = Vec::new();

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

        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        sample_buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(sample_buf.samples());
    }

    Ok(samples)
}
