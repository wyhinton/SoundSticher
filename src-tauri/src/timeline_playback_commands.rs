// Timeline Playback Commands
//
// Architecture: AudioWorker — single dedicated audio thread
//
// PlaybackSession  → per-session metadata + start-position computation
// AudioWorker      → single thread that owns OutputStream + Sink, receives AudioCommands
// TimelinePlaybackController → sends commands to the worker via mpsc channel
// Tauri commands   → thin glue that constructs a controller and calls one method

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use rodio::{OutputStream, Sink};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

use crate::error::Error;
use crate::logging::{LogSystem, LoggingService};
use crate::op_playback_commands::{AudioSpecDebugInfo, BuildGraphResponse};
use crate::playback::op_playback::{SampleTime, TimelineSourceBuilder};
use crate::playback::timeline_manager::{TimelinePlaybackManager, TimelineSource};
use crate::playback::{AudioSpec, PlaybackGraph, PlaybackOpId, TimelinePlaybackEvent};
use crate::sample_cache::SampleCacheService;
use crate::{emit_logged, log_info, send_channel_event};

pub type TimelineId = String;

// ─── Playback transport state (published by worker, read by anyone) ────────

/// Mirrors the worker's internal state for external readers.
/// The worker is the *only* writer; everyone else reads.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Idle = 0,
    Playing = 1,
    Paused = 2,
}

impl From<u8> for TransportState {
    fn from(v: u8) -> Self {
        match v {
            1 => TransportState::Playing,
            2 => TransportState::Paused,
            _ => TransportState::Idle,
        }
    }
}

// ─── AudioCommand ───────────────────────────────────────────────────────────

/// Commands sent to the audio worker thread via an `mpsc` channel.
pub enum AudioCommand {
    /// Start playing a timeline from a position.
    Play {
        timeline_id: TimelineId,
        graph: Arc<PlaybackGraph>,
        spec: AudioSpec,
        loop_playback: bool,
        start_position: SampleTime,
    },
    /// Pause the currently active timeline.
    Pause,
    /// Resume the currently paused timeline.
    Resume,
    /// Stop playback entirely and reset.
    Stop,
    /// Seek to a position (seconds). If playing, restarts from that position.
    Seek {
        timeline_id: TimelineId,
        position_seconds: f64,
        graph: Arc<PlaybackGraph>,
        spec: AudioSpec,
        loop_playback: bool,
    },
    /// Set master volume (0.0–1.0+).
    SetVolume(f32),
    /// Shut down the worker thread.
    Shutdown,
}

// ─── PlaybackSession ────────────────────────────────────────────────────────

/// A playback session for a specific timeline.
///
/// Owns the graph and all per-timeline metadata (progress, seek position,
/// loop flag). Methods here are pure bookkeeping – they never touch the
/// audio sink or spawn threads.
pub struct PlaybackSession {
    /// The playback graph for this timeline
    pub graph: Arc<PlaybackGraph>,

    /// Audio specification
    pub spec: AudioSpec,

    /// Current playback progress (normalised 0.0–1.0)
    pub progress: Mutex<f32>,

    /// Current seek position in seconds
    pub seek_seconds: Mutex<f64>,

    /// Whether this timeline loops
    pub loop_playback: Mutex<bool>,

    /// Mapping of operation names to their IDs in this timeline's graph
    pub op_ids: HashMap<String, PlaybackOpId>,
}

impl PlaybackSession {
    pub fn new(
        graph: Arc<PlaybackGraph>,
        spec: AudioSpec,
        loop_playback: bool,
        op_ids: HashMap<String, PlaybackOpId>,
    ) -> Self {
        Self {
            graph,
            spec,
            progress: Mutex::new(0.0),
            seek_seconds: Mutex::new(0.0),
            loop_playback: Mutex::new(loop_playback),
            op_ids,
        }
    }

    // ── queries ──────────────────────────────────────────────────────────

    pub fn duration_seconds(&self) -> f64 {
        self.graph.duration().to_seconds(self.spec.sample_rate)
    }

    pub fn progress(&self) -> f32 {
        *self.progress.lock().unwrap()
    }

    pub fn seek_seconds(&self) -> f64 {
        *self.seek_seconds.lock().unwrap()
    }

    pub fn loop_enabled(&self) -> bool {
        *self.loop_playback.lock().unwrap()
    }

    pub fn op_names(&self) -> Vec<String> {
        self.op_ids.keys().cloned().collect()
    }

    // ── mutations (pure metadata — no audio side-effects) ────────────────

    /// Prepare metadata for a play-from-position.
    /// Returns the `SampleTime` the caller should use to start playback.
    pub fn prepare_play(&self, start_seconds: Option<f64>) -> SampleTime {
        let total = self.duration_seconds();
        match start_seconds {
            Some(start) => {
                *self.seek_seconds.lock().unwrap() = start;
                *self.progress.lock().unwrap() = if total > 0.0 {
                    (start / total).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                SampleTime::from_seconds(start, self.spec.sample_rate)
            }
            None => {
                let seek = self.seek_seconds();
                SampleTime::from_seconds(seek, self.spec.sample_rate)
            }
        }
    }

    /// Reset progress & seek to zero.
    pub fn reset(&self) {
        *self.progress.lock().unwrap() = 0.0;
        *self.seek_seconds.lock().unwrap() = 0.0;
    }

    pub fn seek(&self, seconds: f64) {
        let total = self.duration_seconds();
        *self.seek_seconds.lock().unwrap() = seconds;
        *self.progress.lock().unwrap() = if total > 0.0 {
            (seconds / total).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
    }

    pub fn set_loop(&self, enabled: bool) {
        *self.loop_playback.lock().unwrap() = enabled;
    }
}

// ─── AppTimelinePlaybackState ───────────────────────────────────────────────

/// Shared state for multi-timeline playback.
///
/// The audio worker thread is the single owner of OutputStream + Sink.
/// All transport commands go through `audio_tx`. The worker publishes
/// its transport state back into `transport_state` and `active_timeline`
/// so that readers can query without locking a channel.
pub struct AppTimelinePlaybackState {
    /// All timeline sessions (metadata only — no audio resources)
    pub(crate) sessions: DashMap<TimelineId, PlaybackSession>,

    /// Which timeline is currently audible (written by worker, read by anyone)
    pub(crate) active_timeline: RwLock<Option<TimelineId>>,

    /// Transport state published by the worker (Idle / Playing / Paused)
    pub(crate) transport_state: AtomicU8,

    /// Channel to send commands to the audio worker thread
    pub(crate) audio_tx: Mutex<Option<mpsc::Sender<AudioCommand>>>,

    // ── Legacy fields (used by op_playback_commands during migration) ────
    /// Whether audio is currently playing (LEGACY — read transport_state instead)
    pub(crate) is_playing: AtomicBool,
    /// Whether playback is paused (LEGACY — read transport_state instead)
    pub(crate) is_paused: AtomicBool,
    /// Single audio sink (LEGACY — the worker owns the real sink now)
    pub(crate) sink: Mutex<Option<Arc<Sink>>>,
}

impl AppTimelinePlaybackState {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            active_timeline: RwLock::new(None),
            transport_state: AtomicU8::new(TransportState::Idle as u8),
            audio_tx: Mutex::new(None),
            is_paused: AtomicBool::new(false),
            is_playing: AtomicBool::new(false),
            sink: Mutex::new(None),
        }
    }

    // ── transport state queries ──────────────────────────────────────────

    pub fn transport(&self) -> TransportState {
        TransportState::from(self.transport_state.load(Ordering::Acquire))
    }

    pub fn is_playing(&self) -> bool {
        self.transport() == TransportState::Playing
    }

    pub fn is_paused(&self) -> bool {
        self.transport() == TransportState::Paused
    }

    pub fn is_idle(&self) -> bool {
        self.transport() == TransportState::Idle
    }

    // ── session helpers ─────────────────────────────────────────────────

    pub fn get_session(
        &self,
        timeline_id: &TimelineId,
    ) -> Option<dashmap::mapref::one::Ref<'_, TimelineId, PlaybackSession>> {
        self.sessions.get(timeline_id)
    }

    pub fn insert_session(&self, timeline_id: TimelineId, session: PlaybackSession) {
        self.sessions.insert(timeline_id, session);
    }

    pub fn remove_session(
        &self,
        timeline_id: &TimelineId,
    ) -> Option<(TimelineId, PlaybackSession)> {
        self.sessions.remove(timeline_id)
    }

    pub fn get_active_timeline(&self) -> Option<TimelineId> {
        self.active_timeline.read().unwrap().clone()
    }

    pub fn set_active_timeline(&self, timeline_id: Option<TimelineId>) {
        *self.active_timeline.write().unwrap() = timeline_id;
    }

    pub fn clear_all(&self) {
        self.sessions.clear();
    }

    // ── worker channel ──────────────────────────────────────────────────

    /// Send a command to the audio worker. Spawns the worker if it isn't
    /// running yet (lazy init).
    fn send_cmd(&self, cmd: AudioCommand) -> Result<(), Error> {
        let guard = self.audio_tx.lock().unwrap();
        match guard.as_ref() {
            Some(tx) => tx
                .send(cmd)
                .map_err(|_| Error::PlaybackError("Audio worker channel disconnected".to_string())),
            None => Err(Error::PlaybackError("Audio worker not started".to_string())),
        }
    }

    // ── Legacy compatibility (for op_playback_commands during migration) ──

    /// Stop the current sink, reset legacy flags, and drop the sink reference.
    /// LEGACY: used only by op_playback_commands. New code uses AudioCommand::Stop.
    pub fn stop_current_playback(&self) {
        self.is_playing.store(false, Ordering::Relaxed);
        self.is_paused.store(false, Ordering::Relaxed);

        let mut sink = self.sink.lock().unwrap();
        if let Some(ref s) = *sink {
            s.stop();
            s.clear();
        }
        *sink = None;
    }

    /// Start the audio worker thread with a proper Arc reference.
    pub fn ensure_worker_arc(
        self: &Arc<Self>,
        app: AppHandle,
        logging: Arc<Mutex<LoggingService>>,
    ) {
        let mut guard = self.audio_tx.lock().unwrap();
        if guard.is_some() {
            return; // already running
        }

        let (tx, rx) = mpsc::channel::<AudioCommand>();
        *guard = Some(tx);
        drop(guard);

        let state = Arc::clone(self);
        thread::spawn(move || {
            audio_worker_loop(rx, state, app, logging);
        });
    }
}

impl Default for AppTimelinePlaybackState {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Audio Worker Loop ──────────────────────────────────────────────────────

/// The single audio thread. Owns the `OutputStream` (platform audio handle)
/// and `Sink`. Processes `AudioCommand`s from the channel and runs a
/// progress-reporting loop while audio is playing.
fn audio_worker_loop(
    rx: mpsc::Receiver<AudioCommand>,
    state: Arc<AppTimelinePlaybackState>,
    app: AppHandle,
    logging: Arc<Mutex<LoggingService>>,
) {
    // Create audio output — lives for the entire worker lifetime.
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("🔊 [audio_worker] FATAL: cannot create audio output: {e}");
            // Mark channel as dead so callers get an error
            *state.audio_tx.lock().unwrap() = None;
            return;
        }
    };

    let mut sink: Option<Sink> = None;

    // Progress tracking state
    let mut tracking_timeline: Option<TimelineId> = None;
    let mut tracking_start = Instant::now();
    let mut initial_seek: f32 = 0.0;
    let mut accumulated_before_pause: f32 = 0.0;
    let mut pause_start: Option<Instant> = None;

    /// Helper: stop current sink and reset tracking
    macro_rules! stop_and_reset {
        () => {
            if let Some(ref s) = sink {
                s.stop();
            }
            sink = None;
            state
                .transport_state
                .store(TransportState::Idle as u8, Ordering::Release);
            state.set_active_timeline(None);
            tracking_timeline = None;
            pause_start = None;
            accumulated_before_pause = 0.0;
            initial_seek = 0.0;
        };
    }

    loop {
        // If we're playing, use a non-blocking recv with a 16ms timeout
        // so we can update progress at ~60fps.
        // If idle/paused, block until a command arrives.
        let cmd = match state.transport() {
            TransportState::Playing => {
                match rx.recv_timeout(Duration::from_millis(16)) {
                    Ok(cmd) => Some(cmd),
                    Err(mpsc::RecvTimeoutError::Timeout) => None, // just update progress
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            TransportState::Paused => {
                // While paused, check less frequently but still respond to commands
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(cmd) => Some(cmd),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            TransportState::Idle => {
                // Block until command
                match rx.recv() {
                    Ok(cmd) => Some(cmd),
                    Err(_) => break, // channel closed
                }
            }
        };

        // ── Handle command ──────────────────────────────────────────────
        if let Some(cmd) = cmd {
            match cmd {
                AudioCommand::Play {
                    timeline_id,
                    graph,
                    spec,
                    loop_playback,
                    start_position,
                } => {
                    // Stop any existing playback
                    stop_and_reset!();

                    // Create new sink
                    let new_sink = match Sink::try_new(&stream_handle) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("🔊 [audio_worker] ERROR creating sink: {e}");
                            continue;
                        }
                    };

                    // Build source and append
                    let source = TimelineSourceBuilder::new()
                        .spec(spec)
                        .looping(loop_playback)
                        .start_position(start_position)
                        .build(graph);

                    new_sink.append(source);
                    new_sink.set_volume(1.0);
                    new_sink.play();

                    // Init progress tracking
                    initial_seek = start_position.to_seconds(spec.sample_rate) as f32;
                    accumulated_before_pause = 0.0;
                    tracking_start = Instant::now();
                    pause_start = None;
                    tracking_timeline = Some(timeline_id.clone());

                    sink = Some(new_sink);
                    state.set_active_timeline(Some(timeline_id));
                    state
                        .transport_state
                        .store(TransportState::Playing as u8, Ordering::Release);
                }

                AudioCommand::Pause => {
                    if let Some(ref s) = sink {
                        s.pause();
                        // Record accumulated time before pause
                        accumulated_before_pause += tracking_start.elapsed().as_secs_f32();
                        pause_start = Some(Instant::now());
                        state
                            .transport_state
                            .store(TransportState::Paused as u8, Ordering::Release);
                    }
                }

                AudioCommand::Resume => {
                    if let Some(ref s) = sink {
                        s.play();
                        pause_start = None;
                        tracking_start = Instant::now();
                        state
                            .transport_state
                            .store(TransportState::Playing as u8, Ordering::Release);
                    }
                }

                AudioCommand::Stop => {
                    // Snapshot final position before resetting
                    if let Some(ref tid) = tracking_timeline {
                        if let Some(session) = state.get_session(tid) {
                            session.reset();
                        }
                    }
                    stop_and_reset!();

                    emit_logged!(
                        app,
                        "op-timeline-progress",
                        serde_json::json!({ "timelineId": null, "progress": 0.0 })
                    );
                }

                AudioCommand::Seek {
                    timeline_id,
                    position_seconds,
                    graph,
                    spec,
                    loop_playback,
                } => {
                    let was_paused = state.is_paused();

                    // Stop current sink
                    if let Some(ref s) = sink {
                        s.stop();
                    }
                    sink = None;

                    // Create new sink from seek position
                    let new_sink = match Sink::try_new(&stream_handle) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("🔊 [audio_worker] ERROR creating sink on seek: {e}");
                            continue;
                        }
                    };

                    let seek_pos = SampleTime::from_seconds(position_seconds, spec.sample_rate);
                    let source = TimelineSourceBuilder::new()
                        .spec(spec)
                        .looping(loop_playback)
                        .start_position(seek_pos)
                        .build(graph);

                    new_sink.append(source);
                    new_sink.set_volume(1.0);

                    // Reset tracking
                    initial_seek = position_seconds as f32;
                    accumulated_before_pause = 0.0;
                    tracking_start = Instant::now();
                    tracking_timeline = Some(timeline_id.clone());

                    if was_paused {
                        new_sink.pause();
                        pause_start = Some(Instant::now());
                        state
                            .transport_state
                            .store(TransportState::Paused as u8, Ordering::Release);
                    } else {
                        new_sink.play();
                        pause_start = None;
                        state
                            .transport_state
                            .store(TransportState::Playing as u8, Ordering::Release);
                    }

                    sink = Some(new_sink);
                    state.set_active_timeline(Some(timeline_id));
                }

                AudioCommand::SetVolume(volume) => {
                    if let Some(ref s) = sink {
                        s.set_volume(volume);
                    }
                }

                AudioCommand::Shutdown => {
                    stop_and_reset!();
                    break;
                }
            }
        }

        // ── Progress update (only while playing) ────────────────────────
        // ── Progress update (only while playing) ────────────────────────
        if state.transport() == TransportState::Playing {
            // If nothing is being tracked, reset defensively
            let Some(ref tid) = tracking_timeline else {
                stop_and_reset!();
                continue;
            };

            // If the session no longer exists, it was removed externally.
            // Stop immediately and reset worker state.
            let Some(session) = state.get_session(tid) else {
                stop_and_reset!();

                emit_logged!(
                    app,
                    "op-timeline-progress",
                    serde_json::json!({ "timelineId": null, "progress": 0.0 })
                );

                continue;
            };

            // Check if sink finished naturally
            let is_empty = sink.as_ref().map(|s| s.empty()).unwrap_or(true);
            let loop_playback = session.loop_enabled();

            if is_empty && !loop_playback {
                // Natural end of playback
                *session.progress.lock().unwrap() = 1.0;

                emit_logged!(
                    app,
                    "op-timeline-progress",
                    serde_json::json!({ "timelineId": tid, "progress": 1.0 })
                );

                stop_and_reset!();
                continue;
            }

            // ── Compute progress ─────────────────────────────────────────
            let total_duration = session.duration_seconds();

            let elapsed = tracking_start.elapsed().as_secs_f32();
            let current_pos = initial_seek + accumulated_before_pause + elapsed;

            let progress = if total_duration > 0.0 {
                let td = total_duration as f32;

                if loop_playback {
                    (current_pos % td) / td
                } else {
                    (current_pos / td).min(1.0)
                }
            } else {
                0.0
            };

            // Update session metadata
            *session.progress.lock().unwrap() = progress;
            *session.seek_seconds.lock().unwrap() = current_pos as f64;

            emit_logged!(
                app,
                "op-timeline-progress",
                serde_json::json!({ "timelineId": tid, "progress": progress })
            );
        }
    }

    // Worker shutting down
    if let Ok(logger) = logging.lock() {
        log_info!(
            logger,
            LogSystem::Playback,
            "audio_worker",
            "Audio worker thread exiting"
        );
    }
}

// ─── TimelinePlaybackController ─────────────────────────────────────────────

/// Runtime controller: sends commands to the audio worker.
///
/// Every public method is self-contained — callers (Tauri commands) construct
/// one, call a single method, and let it drop.
pub struct TimelinePlaybackController {
    state: Arc<AppTimelinePlaybackState>,
    logging: Arc<Mutex<LoggingService>>,
    app: AppHandle,
}

impl TimelinePlaybackController {
    pub fn new(
        state: Arc<AppTimelinePlaybackState>,
        logging: Arc<Mutex<LoggingService>>,
        app: AppHandle,
    ) -> Self {
        // Ensure the audio worker is running
        state.ensure_worker_arc(app.clone(), Arc::clone(&logging));
        Self {
            state,
            logging,
            app,
        }
    }

    // ── play ─────────────────────────────────────────────────────────────

    /// Start playback of `timeline_id`, optionally from `start_seconds`.
    pub fn play(&self, timeline_id: TimelineId, start_seconds: Option<f64>) -> Result<(), Error> {
        // ── 0. Check if this timeline is already playing ─────────────────
        if let Some(ref active) = self.state.get_active_timeline() {
            if active == &timeline_id && self.state.is_playing() {
                return Err(Error::PlaybackError(format!(
                    "Timeline '{}' is already playing",
                    timeline_id
                )));
            }
        }

        // ── 1. Read session metadata ─────────────────────────────────────
        let session = self.state.get_session(&timeline_id).ok_or_else(|| {
            Error::PlaybackError(format!("No session for timeline '{timeline_id}'"))
        })?;

        let start_position = session.prepare_play(start_seconds);
        let loop_playback = session.loop_enabled();
        let spec = session.spec;
        let graph = Arc::clone(&session.graph);
        let total_duration = session.duration_seconds();
        let op_count = session.op_ids.len();
        let op_names = session.op_names();
        drop(session);

        // ── 2. Log ───────────────────────────────────────────────────────
        if let Ok(logger) = self.logging.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "play",
                &format!(
                    "Playing timeline '{}': {} ops [{}], {:.2}s, start={:.2}s, loop={}",
                    timeline_id,
                    op_count,
                    op_names.join(", "),
                    total_duration,
                    start_position.to_seconds(spec.sample_rate),
                    loop_playback
                )
            );
        }

        // ── 3. Send Play command to worker ───────────────────────────────
        self.state.send_cmd(AudioCommand::Play {
            timeline_id,
            graph,
            spec,
            loop_playback,
            start_position,
        })
    }

    // ── pause ────────────────────────────────────────────────────────────

    pub fn pause(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        if self.state.is_paused() {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is already paused",
                timeline_id
            )));
        }
        if !self.state.is_playing() {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not currently playing",
                timeline_id
            )));
        }
        if self.state.get_active_timeline().as_ref() != Some(timeline_id) {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not currently active",
                timeline_id
            )));
        }

        self.state.send_cmd(AudioCommand::Pause)?;

        // Log
        if let Some(session) = self.state.get_session(timeline_id) {
            let progress = session.progress();
            let pos = progress as f64 * session.duration_seconds();
            if let Ok(logger) = self.logging.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "pause",
                    &format!(
                        "Paused timeline '{}' at {:.2}s ({:.1}%)",
                        timeline_id,
                        pos,
                        progress * 100.0
                    )
                );
            }
            emit_logged!(
                self.app,
                "op-timeline-progress",
                serde_json::json!({ "timelineId": timeline_id, "progress": progress })
            );
        }

        Ok(())
    }

    // ── resume ───────────────────────────────────────────────────────────

    pub fn resume(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        if !self.state.is_paused() {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not paused",
                timeline_id
            )));
        }
        if self.state.get_active_timeline().as_ref() != Some(timeline_id) {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not currently active",
                timeline_id
            )));
        }

        self.state.send_cmd(AudioCommand::Resume)?;

        if let Some(session) = self.state.get_session(timeline_id) {
            let progress = session.progress();
            let pos = progress as f64 * session.duration_seconds();
            if let Ok(logger) = self.logging.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "resume",
                    &format!(
                        "Resumed timeline '{}' from {:.2}s ({:.1}%)",
                        timeline_id,
                        pos,
                        progress * 100.0
                    )
                );
            }
        }

        Ok(())
    }

    // ── stop ─────────────────────────────────────────────────────────────

    pub fn stop(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        let session = self.state.get_session(timeline_id).ok_or_else(|| {
            Error::PlaybackError(format!("No session for timeline '{timeline_id}'"))
        })?;

        if let Ok(logger) = self.logging.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "stop",
                &format!(
                    "Stopping timeline '{}' (was at {:.1}%)",
                    timeline_id,
                    session.progress() * 100.0
                )
            );
        }
        drop(session);

        self.state.send_cmd(AudioCommand::Stop)
    }

    // ── seek ─────────────────────────────────────────────────────────────

    pub fn seek(&self, timeline_id: &TimelineId, position_seconds: f64) -> Result<(), Error> {
        let session = self.state.get_session(timeline_id).ok_or_else(|| {
            Error::PlaybackError(format!("No session for timeline '{timeline_id}'"))
        })?;

        let total_duration = session.duration_seconds();
        let spec = session.spec;
        let graph = Arc::clone(&session.graph);
        let loop_playback = session.loop_enabled();

        session.seek(position_seconds);
        let progress = session.progress();
        drop(session);

        if let Ok(logger) = self.logging.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "seek",
                &format!(
                    "Seeking timeline '{}' to {:.2}s / {:.2}s ({:.1}%)",
                    timeline_id,
                    position_seconds,
                    total_duration,
                    progress * 100.0
                )
            );
        }

        emit_logged!(
            self.app,
            "op-timeline-progress",
            serde_json::json!({ "timelineId": timeline_id, "progress": progress })
        );

        // If actively playing/paused, send seek command to worker
        let is_active =
            self.state.get_active_timeline().as_ref() == Some(timeline_id) && !self.state.is_idle();

        if is_active {
            self.state.send_cmd(AudioCommand::Seek {
                timeline_id: timeline_id.clone(),
                position_seconds,
                graph,
                spec,
                loop_playback,
            })?;
        }

        Ok(())
    }

    // ── loop / volume / progress ─────────────────────────────────────────

    pub fn set_loop(&self, timeline_id: &TimelineId, loop_playback: bool) -> Result<(), Error> {
        let session = self.state.get_session(timeline_id).ok_or_else(|| {
            Error::PlaybackError(format!("No session for timeline '{timeline_id}'"))
        })?;
        session.set_loop(loop_playback);

        if let Ok(logger) = self.logging.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "set_loop",
                &format!("Loop={} for timeline '{}'", loop_playback, timeline_id)
            );
        }
        Ok(())
    }

    pub fn set_master_volume(&self, volume: f32) {
        let _ = self.state.send_cmd(AudioCommand::SetVolume(volume));
    }

    pub fn get_progress(&self, timeline_id: &TimelineId) -> Result<f32, Error> {
        let session = self.state.get_session(timeline_id).ok_or_else(|| {
            Error::PlaybackError(format!("No session for timeline '{timeline_id}'"))
        })?;
        Ok(session.progress())
    }

    // ── toggle ───────────────────────────────────────────────────────────

    pub fn toggle(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        let active_timeline = self.state.get_active_timeline();
        let is_this_timeline_active = active_timeline.as_ref() == Some(timeline_id);

        if is_this_timeline_active && self.state.is_playing() {
            self.pause(timeline_id)
        } else if is_this_timeline_active && self.state.is_paused() {
            self.resume(timeline_id)
        } else {
            self.play(timeline_id.clone(), None)
        }
    }

    // ── clear ────────────────────────────────────────────────────────────

    pub fn clear(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        // Stop if this is the active timeline
        if self.state.get_active_timeline().as_ref() == Some(timeline_id) {
            let _ = self.state.send_cmd(AudioCommand::Stop);
        }

        if let Some((_, session)) = self.state.remove_session(timeline_id) {
            if let Ok(logger) = self.logging.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "clear",
                    &format!(
                        "Cleared timeline '{}' ({} ops, {:.2}s)",
                        timeline_id,
                        session.op_ids.len(),
                        session.duration_seconds()
                    )
                );
            }
        }
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), Error> {
        let count = self.state.sessions.len();
        let _ = self.state.send_cmd(AudioCommand::Stop);
        self.state.clear_all();

        if let Ok(logger) = self.logging.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "clear_all",
                &format!("Cleared all {} timeline sessions", count)
            );
        }
        Ok(())
    }
}

// ─── Debug info ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSessionDebugInfo {
    pub duration_seconds: f64,
    pub progress: f32,
    pub seek_seconds: f32,
    pub loop_playback: bool,
    pub operation_names: Vec<String>,
    pub operation_count: usize,
    pub spec: AudioSpecDebugInfo,
}

// ─── Tauri commands (thin glue) ─────────────────────────────────────────────

#[tauri::command]
pub async fn timeline_build_playback(
    timeline_id: TimelineId,
    source: TimelineSource,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    on_event: Channel<TimelinePlaybackEvent>,
) -> Result<BuildGraphResponse, Error> {
    let state = Arc::clone(state.inner());
    let sample_cache = Arc::clone(sample_cache.inner());
    let logging_service = Arc::clone(logging_service.inner());

    tauri::async_runtime::spawn_blocking(move || {
        let manager = TimelinePlaybackManager::new(state, sample_cache, logging_service);
        manager.build_timeline(timeline_id, source, |event| {
            send_channel_event!(on_event, event);
        })
    })
    .await
    .map_err(|e| Error::PlaybackError(format!("Task panicked: {}", e)))?
    .map_err(Error::PlaybackError)
}

#[tauri::command]
pub fn timeline_play(
    timeline_id: TimelineId,
    start_seconds: Option<f64>,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).play(timeline_id, start_seconds)
}

#[tauri::command]
pub fn timeline_pause(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).pause(&timeline_id)
}

#[tauri::command]
pub fn timeline_resume(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).resume(&timeline_id)
}

#[tauri::command]
pub fn timeline_stop(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).stop(&timeline_id)
}

#[tauri::command]
pub fn timeline_seek(
    timeline_id: TimelineId,
    position_seconds: f64,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).seek(&timeline_id, position_seconds)
}

#[tauri::command]
pub fn timeline_set_loop(
    timeline_id: TimelineId,
    loop_playback: bool,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).set_loop(&timeline_id, loop_playback)
}

/// Set master playback volume (global, not per-timeline)
#[tauri::command]
pub fn timeline_set_volume(
    volume: f32,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).set_master_volume(volume);
    Ok(())
}

#[tauri::command]
pub fn timeline_get_progress(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<f32, Error> {
    make_controller(&state, &logging, app).get_progress(&timeline_id)
}

#[tauri::command]
pub fn timeline_clear(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).clear(&timeline_id)
}

#[tauri::command]
pub fn timeline_clear_all(
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).clear_all()
}

#[tauri::command]
pub fn timeline_toggle(
    timeline_id: TimelineId,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    make_controller(&state, &logging, app).toggle(&timeline_id)
}

// ─── Helper ─────────────────────────────────────────────────────────────────

fn make_controller(
    state: &State<'_, Arc<AppTimelinePlaybackState>>,
    logging: &State<'_, Arc<Mutex<LoggingService>>>,
    app: AppHandle,
) -> TimelinePlaybackController {
    TimelinePlaybackController::new(Arc::clone(state.inner()), Arc::clone(logging.inner()), app)
}

#[tauri::command]
pub fn get_app_playback_state(
    state: State<'_, Arc<AppTimelinePlaybackState>>,
) -> Result<AppPlaybackStateDebugInfo, String> {
    let active_timeline = state.get_active_timeline();
    let transport = state.transport();

    // BTreeMap guarantees deterministic ordering by key
    let mut sessions_info: BTreeMap<String, PlaybackSessionDebugInfo> = BTreeMap::new();

    for entry in state.sessions.iter() {
        let timeline_id = entry.key().clone();
        let session = entry.value();

        let operation_names: Vec<String> = session.op_ids.keys().cloned().collect();
        let operation_count = operation_names.len();

        let duration_seconds = session.duration_seconds();
        let progress = *session.progress.lock().unwrap();
        let seek_seconds = *session.seek_seconds.lock().unwrap() as f32;
        let loop_playback = *session.loop_playback.lock().unwrap();
        let spec = session.spec.into();

        sessions_info.insert(
            timeline_id,
            PlaybackSessionDebugInfo {
                duration_seconds,
                progress,
                seek_seconds,
                loop_playback,
                operation_names,
                operation_count,
                spec,
            },
        );
    }

    let total_sessions = sessions_info.len();

    Ok(AppPlaybackStateDebugInfo {
        sessions: sessions_info,
        active_timeline,
        is_playing: transport == TransportState::Playing,
        is_paused: transport == TransportState::Paused,
        total_sessions,
    })
}

/// Serializable representation of the playback state for debugging
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPlaybackStateDebugInfo {
    pub sessions: BTreeMap<TimelineId, PlaybackSessionDebugInfo>,
    pub active_timeline: Option<TimelineId>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub total_sessions: usize,
}

#[tauri::command]
pub fn op_timeline_sync_full(
    timeline_ids: Vec<TimelineId>,
    state: State<'_, Arc<AppTimelinePlaybackState>>,
) -> Result<(), String> {
    let desired: HashSet<TimelineId> = timeline_ids.into_iter().collect();

    // Current session keys
    let existing: HashSet<TimelineId> = state.sessions.iter().map(|e| e.key().clone()).collect();

    // Compute diff
    let to_add = desired.difference(&existing);
    let to_remove = existing.difference(&desired);

    // 1️⃣ Remove stale sessions
    for timeline_id in to_remove {
        state.remove_session(timeline_id);

        // If this was active, stop transport
        if state.get_active_timeline().as_ref() == Some(timeline_id) {
            state.set_active_timeline(None);

            // Tell worker to stop
            if let Err(e) = state.send_cmd(AudioCommand::Stop) {
                eprintln!("Failed to stop worker during sync: {e}");
            }
        }
    }

    // 2️⃣ Insert placeholders for new timelines
    for timeline_id in to_add {
        // Placeholder session (no graph yet)
        let empty_graph = Arc::new(PlaybackGraph::new(AudioSpec::cd_quality()));
        let spec = AudioSpec::default();

        let session = PlaybackSession::new(empty_graph, spec, false, HashMap::new());

        state.insert_session(timeline_id.clone(), session);
    }

    Ok(())
}
