// Timeline Playback Commands
//
// Architecture: Option C – Hybrid
//
// PlaybackSession  → per-session metadata + start-position computation
// TimelinePlaybackController → multi-session orchestration, global sink, playback threads
// Tauri commands   → thin glue that constructs a controller and calls one method
//
// The old op_playback_* commands remain as legacy wrappers; new work goes through
// the controller.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
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
/// Fields are `pub(crate)` so that both the controller (in this module) and
/// the legacy `op_playback_commands` module can access them during the
/// migration period.
pub struct AppTimelinePlaybackState {
    /// All timeline sessions
    pub(crate) sessions: DashMap<TimelineId, PlaybackSession>,

    /// Which timeline is currently audible (only one at a time — hardware constraint)
    pub(crate) active_timeline: RwLock<Option<TimelineId>>,

    /// Single audio sink
    pub(crate) sink: Mutex<Option<Arc<Sink>>>,

    /// Whether audio is currently playing
    pub(crate) is_playing: AtomicBool,

    /// Whether playback is paused
    pub(crate) is_paused: AtomicBool,
}

impl AppTimelinePlaybackState {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            active_timeline: RwLock::new(None),
            sink: Mutex::new(None),
            is_paused: AtomicBool::new(false),
            is_playing: AtomicBool::new(false),
        }
    }

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

    /// Stop the current sink, reset flags, and drop the sink reference.
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
}

impl Default for AppTimelinePlaybackState {
    fn default() -> Self {
        Self::new()
    }
}

// ─── TimelinePlaybackController ─────────────────────────────────────────────

/// Runtime controller: multi-session orchestration and global sink management.
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
        Self {
            state,
            logging,
            app,
        }
    }

    // ── play ─────────────────────────────────────────────────────────────

    /// Start playback of `timeline_id`, optionally from `start_seconds`.
    ///
    /// This is the full replacement for the old `op_playback_play`:
    /// it computes the start position, stops any current playback,
    /// spawns the playback thread, and runs the progress loop.
    pub fn play(&self, timeline_id: TimelineId, start_seconds: Option<f64>) -> Result<(), Error> {
        // ── 0. Check if this timeline is already playing ─────────────────
        if let Some(active) = self.state.get_active_timeline() {
            if active == timeline_id
                && self.state.is_playing.load(Ordering::Relaxed)
                && !self.state.is_paused.load(Ordering::Relaxed)
            {
                return Err(Error::PlaybackError(format!(
                    "Timeline '{}' is already playing",
                    timeline_id
                )));
            }
        }

        // ── 1. Read session metadata (immutable snapshot) ────────────────
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
        drop(session); // release DashMap ref before locking sink

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

        // ── 3. Stop whatever is currently playing ────────────────────────
        self.state.stop_current_playback();

        // ── 4. Update global state ───────────────────────────────────────
        self.state.set_active_timeline(Some(timeline_id.clone()));
        self.state.is_playing.store(true, Ordering::Relaxed);
        self.state.is_paused.store(false, Ordering::Relaxed);

        // ── 5. Spawn playback thread ─────────────────────────────────────
        let state = Arc::clone(&self.state);
        let app = self.app.clone();
        let tid = timeline_id.clone();

        thread::spawn(move || {
            start_playback_from_position(
                state,
                app,
                tid,
                graph,
                spec,
                loop_playback,
                start_position,
                false, // not paused
            );
        });

        Ok(())
    }

    // ── pause ────────────────────────────────────────────────────────────

    /// Pause the currently active timeline.
    /// Pause a specific timeline.
    pub fn pause(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        // Check if already paused
        if self.state.is_paused.load(Ordering::Relaxed) {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is already paused",
                timeline_id
            )));
        }

        // Check if not playing
        if !self.state.is_playing.load(Ordering::Relaxed) {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not currently playing",
                timeline_id
            )));
        }

        // Check if this is the active timeline
        if let Some(active) = self.state.get_active_timeline() {
            if &active != timeline_id {
                return Err(Error::PlaybackError(format!(
                    "Timeline '{}' is not currently active",
                    timeline_id
                )));
            }
        } else {
            return Err(Error::PlaybackError(
                "No active timeline to pause".to_string(),
            ));
        }

        let sink = self.state.sink.lock().unwrap();
        if let Some(ref s) = *sink {
            s.pause();
            self.state.is_paused.store(true, Ordering::Relaxed);
            self.state.is_playing.store(false, Ordering::Relaxed);
        }
        drop(sink);

        // Log + emit progress snapshot
        if let Some(session) = self.state.get_session(&timeline_id) {
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

    /// Resume the currently active timeline.
    /// Resume a specific timeline.
    pub fn resume(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        // Check if not paused
        if !self.state.is_paused.load(Ordering::Relaxed) {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not paused",
                timeline_id
            )));
        }

        // Check if not playing
        if !self.state.is_playing.load(Ordering::Relaxed) {
            return Err(Error::PlaybackError(format!(
                "Timeline '{}' is not currently playing",
                timeline_id
            )));
        }

        // Check if this is the active timeline
        if let Some(active) = self.state.get_active_timeline() {
            if &active != timeline_id {
                return Err(Error::PlaybackError(format!(
                    "Timeline '{}' is not currently active",
                    timeline_id
                )));
            }
        } else {
            return Err(Error::PlaybackError(
                "No active timeline to resume".to_string(),
            ));
        }

        let sink = self.state.sink.lock().unwrap();
        match *sink {
            Some(ref s) => {
                s.play();
                self.state.is_paused.store(false, Ordering::Relaxed);
            }
            None => {
                return Err(Error::PlaybackError(
                    "No active playback to resume".to_string(),
                ))
            }
        }
        drop(sink);

        if let Some(session) = self.state.get_session(&timeline_id) {
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

    /// Stop a specific timeline and reset its progress to zero.
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

        session.reset();
        drop(session);

        self.state.stop_current_playback();
        self.state.set_active_timeline(None);

        emit_logged!(
            self.app,
            "op-timeline-progress",
            serde_json::json!({ "timelineId": null, "progress": 0.0 })
        );

        Ok(())
    }

    // ── seek ─────────────────────────────────────────────────────────────

    /// Seek to `position_seconds` within `timeline_id`.
    ///
    /// If the timeline is actively playing, the audio is restarted from the
    /// new position (since rodio's `try_seek` is not always supported).
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

        // If this timeline is actively playing, restart from new position
        let is_active = self.state.get_active_timeline().as_ref() == Some(timeline_id)
            && self.state.is_playing.load(Ordering::Relaxed);

        if is_active {
            let seek_duration = Duration::from_secs_f64(position_seconds);

            // Try native seek first
            let sink = self.state.sink.lock().unwrap();
            let seek_ok = sink
                .as_ref()
                .and_then(|s| s.try_seek(seek_duration).ok())
                .is_some();
            drop(sink);

            if !seek_ok {
                // Restart from new position
                let was_paused = self.state.is_paused.load(Ordering::Relaxed);
                self.state.stop_current_playback();
                self.state.is_playing.store(true, Ordering::Relaxed);
                self.state.is_paused.store(was_paused, Ordering::Relaxed);

                let state = Arc::clone(&self.state);
                let app = self.app.clone();
                let tid = timeline_id.clone();
                let seek_pos = SampleTime::from_seconds(position_seconds, spec.sample_rate);

                thread::spawn(move || {
                    start_playback_from_position(
                        state,
                        app,
                        tid,
                        graph,
                        spec,
                        loop_playback,
                        seek_pos,
                        was_paused,
                    );
                });
            }
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
        let sink = self.state.sink.lock().unwrap();
        if let Some(ref s) = *sink {
            s.set_volume(volume);
        }
    }

    pub fn get_progress(&self, timeline_id: &TimelineId) -> Result<f32, Error> {
        let session = self.state.get_session(timeline_id).ok_or_else(|| {
            Error::PlaybackError(format!("No session for timeline '{timeline_id}'"))
        })?;
        Ok(session.progress())
    }

    // ── clear ────────────────────────────────────────────────────────────

    pub fn clear(&self, timeline_id: &TimelineId) -> Result<(), Error> {
        // Stop if this is the active timeline
        if self.state.get_active_timeline().as_ref() == Some(timeline_id) {
            self.state.stop_current_playback();
            self.state.set_active_timeline(None);
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
        self.state.stop_current_playback();
        self.state.set_active_timeline(None);
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

// ─── Playback engine (private) ──────────────────────────────────────────────

/// Spawn an audio output on the current thread, feed it the graph from
/// `position`, and run a progress loop until playback ends or is stopped.
///
/// **Must be called on a dedicated thread** – `OutputStream` is `!Send` and
/// must stay alive for the entire duration of playback.
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
    // Create audio output — must live on THIS thread
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("🔊 [start_playback] ERROR creating audio output: {e}");
            state.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    let sink = match Sink::try_new(&stream_handle) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("🔊 [start_playback] ERROR creating sink: {e}");
            state.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    // Build the timeline audio source
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

    // Store sink so pause/resume/stop/volume can reach it
    *state.sink.lock().unwrap() = Some(Arc::clone(&sink));

    // Run the progress loop (blocks until done)
    run_progress_loop(&state, &timeline_id, &app);

    // OutputStream drops here → audio stops
}

/// Progress tracking loop.  Blocks the current thread until playback ends.
fn run_progress_loop(
    state: &Arc<AppTimelinePlaybackState>,
    timeline_id: &TimelineId,
    app: &AppHandle,
) {
    // Capture the initial seek offset once
    let initial_seek: f32 = state
        .get_session(timeline_id)
        .map(|s| s.seek_seconds() as f32)
        .unwrap_or(0.0);

    let mut tracking_start = Instant::now();
    let mut pause_start: Option<Instant> = None;
    let mut total_pause_duration = Duration::ZERO;
    let mut accumulated_before_pause: f32 = 0.0;

    loop {
        if !state.is_playing.load(Ordering::Relaxed) {
            break;
        }
        if state.get_active_timeline().as_ref() != Some(timeline_id) {
            break;
        }

        let (total_duration, loop_playback) = {
            let Some(session) = state.get_session(timeline_id) else {
                break;
            };
            (session.duration_seconds(), session.loop_enabled())
        };

        // Check sink empty
        {
            let sg = state.sink.lock().unwrap();
            let should_break = match *sg {
                Some(ref s) => s.empty() && !loop_playback,
                None => true,
            };
            if should_break {
                break;
            }
        }

        // ── paused ──────────────────────────────────────────────────────
        if state.is_paused.load(Ordering::Relaxed) {
            if pause_start.is_none() {
                accumulated_before_pause += tracking_start.elapsed().as_secs_f32();
                pause_start = Some(Instant::now());
            }
            thread::sleep(Duration::from_millis(50));
            continue;
        } else if let Some(ps) = pause_start.take() {
            total_pause_duration += ps.elapsed();

            // Detect seek-while-paused
            if let Some(session) = state.get_session(timeline_id) {
                let current_seek = session.seek_seconds() as f32;
                let expected = initial_seek + accumulated_before_pause;
                if (current_seek - expected).abs() > 0.01 {
                    accumulated_before_pause = current_seek - initial_seek;
                }
            }
            tracking_start = Instant::now();
        }

        // ── compute progress ────────────────────────────────────────────
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

        // Update session
        if let Some(session) = state.get_session(timeline_id) {
            *session.progress.lock().unwrap() = progress;
            *session.seek_seconds.lock().unwrap() = current_pos as f64;
        }

        emit_logged!(
            app,
            "op-timeline-progress",
            serde_json::json!({ "timelineId": timeline_id, "progress": progress })
        );

        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    // Snapshot final position
    let final_pos =
        initial_seek + accumulated_before_pause + tracking_start.elapsed().as_secs_f32();
    if let Some(session) = state.get_session(timeline_id) {
        *session.seek_seconds.lock().unwrap() = final_pos as f64;
    }
    state.is_playing.store(false, Ordering::Relaxed);
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
    let is_playing = state.is_playing.load(Ordering::Relaxed);
    let is_paused = state.is_paused.load(Ordering::Relaxed);

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
        is_playing,
        is_paused,
        total_sessions,
    })
}
/// Serializable representation of OpPlaybackState for debugging
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPlaybackStateDebugInfo {
    pub sessions: BTreeMap<TimelineId, PlaybackSessionDebugInfo>,
    pub active_timeline: Option<TimelineId>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub total_sessions: usize,
}
