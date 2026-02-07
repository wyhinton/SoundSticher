// TimelinePlaybackManager - Timeline-centric orchestration layer
//
// This module is the **single choke point** for timeline lifecycle:
//   - Figures out WHAT to build (source type → builder selection)
//   - Calls the appropriate builder (via TimelinePlaybackBuilder trait)
//   - Registers the session in AppTimelinePlaybackState
//   - Handles replacement / coexistence
//   - Emits high-level lifecycle events
//
// Builders are pure: they return a PlaybackSession without touching
// global state.  Only this manager decides where sessions live.

use crate::log_info;
use crate::logging::{LogSystem, LoggingService};
use crate::op_playback_commands::{AppTimelinePlaybackState, BuildGraphResponse, PlaybackSession};
use crate::playback::builder::{
    BuildContext, BuildPlaybackRequest, BuildResult, OpGraphPlaybackBuilder, TimelinePlaybackBuilder,
};
use crate::playback::session_builder::SessionBuildEvent;
use crate::sample_cache::SampleCacheService;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub type TimelineId = String;

/// Information about a timeline for playback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineInfo {
    /// Unique identifier for this timeline
    pub id: TimelineId,
    /// The source of audio for this timeline
    pub source: TimelineSource,
}

/// Source type for timeline audio.
///
/// Each variant maps 1:1 to a builder via `TimelinePlaybackManager::select_builder`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TimelineSource {
    /// Audio from an operation graph (the main use case)
    Operation {
        /// The build request for the operation graph
        request: crate::op_playback_commands::BuildOpPlaybackGraphRequest,
    },

    /// Audio from a single file (future)
    #[serde(rename = "audioFile")]
    AudioFile { file_path: String },

    /// Live audio input (future)
    #[serde(rename = "liveInput")]
    LiveInput { device_id: String },
}

/// Events emitted during timeline playback management
#[derive(Clone, Debug, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum TimelinePlaybackEvent {
    /// Session build started
    BuildStarted {
        timeline_id: TimelineId,
        operation_count: usize,
    },
    /// Session build progress
    BuildProgress {
        timeline_id: TimelineId,
        operation_name: String,
        operation_index: usize,
        total_operations: usize,
        duration_seconds: f64,
    },
    /// Session build finished
    BuildFinished {
        timeline_id: TimelineId,
        operation_count: usize,
        total_duration_seconds: f64,
        sample_rate: u32,
        channels: u16,
    },
    /// Error during build
    BuildError {
        timeline_id: TimelineId,
        error: String,
    },
}

impl From<SessionBuildEvent> for TimelinePlaybackEvent {
    fn from(event: SessionBuildEvent) -> Self {
        match event {
            SessionBuildEvent::Started {
                timeline_id,
                operation_count,
            } => TimelinePlaybackEvent::BuildStarted {
                timeline_id,
                operation_count,
            },
            SessionBuildEvent::Progress {
                timeline_id,
                operation_name,
                operation_index,
                total_operations,
                duration_seconds,
            } => TimelinePlaybackEvent::BuildProgress {
                timeline_id,
                operation_name,
                operation_index,
                total_operations,
                duration_seconds,
            },
            SessionBuildEvent::Finished {
                timeline_id,
                operation_count,
                total_duration_seconds,
                sample_rate,
                channels,
            } => TimelinePlaybackEvent::BuildFinished {
                timeline_id,
                operation_count,
                total_duration_seconds,
                sample_rate,
                channels,
            },
        }
    }
}

/// Central manager for timeline playback
///
/// This manager is the **single canonical path** for timeline lifecycle:
/// - Selects the appropriate builder based on source type
/// - Delegates construction (pure, no side effects)
/// - Owns session registration (the ONLY place sessions are inserted)
/// - Provides a clear choke point for logging, metrics, and cleanup
pub struct TimelinePlaybackManager {
    /// Session store (owns sessions and transport state)
    session_store: Arc<AppTimelinePlaybackState>,
    /// Build context (shared services for all builders)
    build_ctx: BuildContext,
    /// Logging service (separate ref for manager-level logging)
    logging_service: Arc<Mutex<LoggingService>>,
}

impl TimelinePlaybackManager {
    pub fn new(
        session_store: Arc<AppTimelinePlaybackState>,
        sample_cache: Arc<SampleCacheService>,
        logging_service: Arc<Mutex<LoggingService>>,
    ) -> Self {
        let build_ctx = BuildContext {
            sample_cache,
            logging_service: Arc::clone(&logging_service),
        };

        Self {
            session_store,
            build_ctx,
            logging_service,
        }
    }

    /// Build a timeline from its source.
    ///
    /// This method:
    /// 1. Selects the appropriate builder based on source type
    /// 2. Converts the source into a `BuildPlaybackRequest`
    /// 3. Calls the builder (pure — no state mutation)
    /// 4. Registers the resulting session (the ONLY place this happens)
    /// 5. Returns the build response metadata
    pub fn build_timeline<F>(
        &self,
        timeline_id: TimelineId,
        source: TimelineSource,
        on_event: F,
    ) -> Result<BuildGraphResponse, String>
    where
        F: Fn(TimelinePlaybackEvent),
    {
        if let Ok(logger) = self.logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "timeline_manager",
                &format!(
                    "Building timeline '{}' from {:?} source",
                    timeline_id,
                    source_type_name(&source)
                )
            );
        }

        // 1. Select builder + convert source to request
        let (builder, request) = self.select_builder_and_request(&source)?;

        // 2. Build the session (pure — no state mutation)
        let result = builder.build(&timeline_id, &request, &self.build_ctx, &|e| {
            on_event(e.into())
        })?;

        let response = BuildGraphResponse {
            operation_count: result.operation_count,
            total_duration_seconds: result.total_duration_seconds,
            sample_rate: result.sample_rate,
            channels: result.channels,
        };

        // 3. Register the session (the ONLY place sessions are inserted)
        self.register_session(timeline_id.clone(), result);

        if let Ok(logger) = self.logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "timeline_manager",
                &format!(
                    "Timeline '{}' registered ({:.2}s duration, {} ops)",
                    timeline_id, response.total_duration_seconds, response.operation_count
                )
            );
        }

        Ok(response)
    }

    /// Select the right builder and convert the source into a builder request.
    ///
    /// This is the dispatch table — add new source types here.
    fn select_builder_and_request(
        &self,
        source: &TimelineSource,
    ) -> Result<(Box<dyn TimelinePlaybackBuilder>, BuildPlaybackRequest), String> {
        match source {
            TimelineSource::Operation { request } => Ok((
                Box::new(OpGraphPlaybackBuilder),
                BuildPlaybackRequest::OpGraph(request.clone()),
            )),
            TimelineSource::AudioFile { file_path } => Err(format!(
                "AudioFile timeline source not yet implemented (file: {})",
                file_path
            )),
            TimelineSource::LiveInput { device_id } => Err(format!(
                "LiveInput timeline source not yet implemented (device: {})",
                device_id
            )),
        }
    }

    /// Register a built session into the session store.
    ///
    /// This is the SINGLE place where sessions enter `AppTimelinePlaybackState`.
    /// Future enhancements (replacement policies, cleanup hooks, metrics) go here.
    fn register_session(&self, timeline_id: TimelineId, result: BuildResult) {
        // If there was an existing session for this timeline, log replacement
        if self.session_store.get_session(&timeline_id).is_some() {
            if let Ok(logger) = self.logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Playback,
                    "timeline_manager",
                    &format!(
                        "Replacing existing session for timeline '{}'",
                        timeline_id
                    )
                );
            }
        }

        self.session_store
            .insert_session(timeline_id, result.session);
    }

    /// Check if a timeline exists in the session store
    pub fn has_timeline(&self, timeline_id: &TimelineId) -> bool {
        self.session_store.get_session(timeline_id).is_some()
    }

    /// Remove a timeline from the session store
    pub fn remove_timeline(&self, timeline_id: &TimelineId) -> bool {
        self.session_store.remove_session(timeline_id).is_some()
    }

    /// Get reference to the session store for playback control
    pub fn session_store(&self) -> &Arc<AppTimelinePlaybackState> {
        &self.session_store
    }

    /// Get reference to the build context (useful for offline renders, tests, etc.)
    pub fn build_context(&self) -> &BuildContext {
        &self.build_ctx
    }
}

/// Helper to get a human-readable source type name for logging
fn source_type_name(source: &TimelineSource) -> &'static str {
    match source {
        TimelineSource::Operation { .. } => "Operation",
        TimelineSource::AudioFile { .. } => "AudioFile",
        TimelineSource::LiveInput { .. } => "LiveInput",
    }
}
