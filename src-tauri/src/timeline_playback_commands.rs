// Timeline Playback Commands - Thin Tauri command adapters
//
// These commands are thin adapters that delegate to TimelinePlaybackManager.
// They handle:
// - Tauri command boundary
// - Async/threading
// - Event emission
// - Error mapping
//
// They do NOT handle:
// - Graph construction (delegated to OpPlaybackSessionBuilder)
// - Session management (delegated to TimelinePlaybackManager)
// - Domain logic

use std::sync::{Arc, Mutex};

use tauri::ipc::Channel;
use tauri::AppHandle;
use tauri::State;

use crate::logging::LoggingService;
use crate::op_playback_commands::{BuildGraphRequest, BuildGraphResponse, OpPlaybackState};
use crate::playback::timeline_manager::{
    TimelinePlaybackEvent, TimelinePlaybackManager, TimelineSource,
};
use crate::sample_cache::SampleCacheService;
use crate::send_channel_event;

pub type TimelineId = String;

/// Build a timeline's playback graph
///
/// This command:
/// 1. Creates a TimelinePlaybackManager
/// 2. Delegates building to the manager
/// 3. The manager uses OpPlaybackSessionBuilder for pure graph construction
/// 4. The manager inserts the session into OpPlaybackState
#[tauri::command]
pub async fn timeline_build_playback(
    timeline_id: TimelineId,
    source: TimelineSource,
    state: State<'_, Arc<OpPlaybackState>>,
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    on_event: Channel<TimelinePlaybackEvent>,
) -> Result<BuildGraphResponse, String> {
    let state = state.inner().clone();
    let sample_cache = sample_cache.inner().clone();
    let logging_service = logging_service.inner().clone();

    tauri::async_runtime::spawn_blocking(move || {
        let manager = TimelinePlaybackManager::new(state, sample_cache, logging_service);

        manager.build_timeline(timeline_id, source, |e| {
            let _ = send_channel_event!(on_event, e);
        })
    })
    .await
    .map_err(|e| format!("Failed to execute build timeline task: {}", e))?
}

/// Build a timeline using legacy BuildGraphRequest format
///
/// This is a convenience wrapper for building operation-based timelines
/// using the familiar BuildGraphRequest structure.
#[tauri::command]
pub async fn timeline_build_from_request(
    timeline_id: TimelineId,
    request: BuildGraphRequest,
    state: State<'_, Arc<OpPlaybackState>>,
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    on_event: Channel<TimelinePlaybackEvent>,
) -> Result<BuildGraphResponse, String> {
    // Wrap the request in a TimelineSource::Operation
    let source = TimelineSource::Operation { request };

    timeline_build_playback(
        timeline_id,
        source,
        state,
        sample_cache,
        logging_service,
        on_event,
    )
    .await
}

/// Play a timeline
///
/// Delegates to the existing op_playback_play for operation-based timelines.
/// In the future, this will handle different source types.
#[tauri::command]
pub fn timeline_play(
    timeline_id: TimelineId,
    start_seconds: Option<f64>,
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    // For now, delegate to the existing op_playback_play
    // In the future, we could check the timeline's source type
    crate::op_playback_commands::op_playback_play(timeline_id, start_seconds, state, app, logging)
}

/// Pause the currently active timeline
#[tauri::command]
pub fn timeline_pause(
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_pause(state, app, logging)
}

/// Resume the currently active timeline
#[tauri::command]
pub fn timeline_resume(
    state: State<'_, Arc<OpPlaybackState>>,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_resume(state, logging)
}

/// Stop playback and reset
#[tauri::command]
pub fn timeline_stop(
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_stop(state, app, logging)
}

/// Seek to a position in a timeline
#[tauri::command]
pub fn timeline_seek(
    timeline_id: TimelineId,
    position_seconds: f64,
    state: State<'_, Arc<OpPlaybackState>>,
    app: AppHandle,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_seek(
        timeline_id,
        position_seconds,
        state,
        app,
        logging,
    )
}

/// Set loop mode for a timeline
#[tauri::command]
pub fn timeline_set_loop(
    timeline_id: TimelineId,
    loop_playback: bool,
    state: State<'_, Arc<OpPlaybackState>>,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_set_loop(timeline_id, loop_playback, state, logging)
}

/// Set playback volume
#[tauri::command]
pub fn timeline_set_volume(
    volume: f32,
    state: State<'_, Arc<OpPlaybackState>>,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_set_volume(volume, state, logging)
}

/// Get playback progress for a timeline
#[tauri::command]
pub fn timeline_get_progress(
    timeline_id: TimelineId,
    state: State<'_, Arc<OpPlaybackState>>,
) -> Result<f32, String> {
    crate::op_playback_commands::op_playback_get_progress(timeline_id, state)
}

/// Clear a specific timeline
#[tauri::command]
pub fn timeline_clear(
    timeline_id: TimelineId,
    state: State<'_, Arc<OpPlaybackState>>,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_clear_timeline(timeline_id, state, logging)
}

/// Clear all timelines
#[tauri::command]
pub fn timeline_clear_all(
    state: State<'_, Arc<OpPlaybackState>>,
    logging: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), String> {
    crate::op_playback_commands::op_playback_clear_all_timelines(state, logging)
}
