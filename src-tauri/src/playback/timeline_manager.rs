// TimelinePlaybackManager - Orchestrates timeline playback
//
// This module provides the central manager for timeline playback.
// It decides when to build sessions and where to store th em,
// delegating the actual building to OpPlaybackSessionBuilder.

use crate::log_info;
use crate::logging::{LogSystem, LoggingService};
use crate::op_playback_commands::{
    BuildGraphRequest, BuildGraphResponse, OpPlaybackState, PlaybackSession,
};
use crate::playback::session_builder::{
    OpPlaybackSessionBuilder, SessionBuildEvent, SessionBuildRequest,
};
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

/// Source type for timeline audio
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TimelineSource {
    /// Audio from an operation graph (the main use case)
    Operation {
        /// The build request for the operation graph
        request: BuildGraphRequest,
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
/// This manager:
/// - Orchestrates session building via OpPlaybackSessionBuilder
/// - Decides when/where to store sessions (OpPlaybackState)
/// - Routes playback commands to the appropriate handler based on source type
pub struct TimelinePlaybackManager {
    /// Session store (owns sessions and transport state)
    session_store: Arc<OpPlaybackState>,
    /// Session builder (pure, no side effects)
    builder: OpPlaybackSessionBuilder,
    /// Logging service
    logging_service: Arc<Mutex<LoggingService>>,
}

impl TimelinePlaybackManager {
    pub fn new(
        session_store: Arc<OpPlaybackState>,
        sample_cache: Arc<SampleCacheService>,
        logging_service: Arc<Mutex<LoggingService>>,
    ) -> Self {
        let builder = OpPlaybackSessionBuilder::new(sample_cache, Arc::clone(&logging_service));

        Self {
            session_store,
            builder,
            logging_service,
        }
    }

    /// Build a timeline from its source
    ///
    /// This method:
    /// - Determines the appropriate builder based on source type
    /// - Builds the session
    /// - Inserts it into the session store
    /// - Returns the build response
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
                &format!("Building timeline '{}' from source", timeline_id)
            );
        }

        match source {
            TimelineSource::Operation { request } => {
                self.build_operation_timeline(timeline_id, request, on_event)
            }
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

    /// Build a timeline from an operation graph request
    fn build_operation_timeline<F>(
        &self,
        timeline_id: TimelineId,
        request: BuildGraphRequest,
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
                    "Building operation timeline '{}' with {} operations",
                    timeline_id,
                    request.operations.len()
                )
            );
        }

        // Convert to internal request type
        let session_request: SessionBuildRequest = request.clone().into();
        let sample_rate = request.sample_rate.unwrap_or(44100);
        let channels = request.channels.unwrap_or(2);

        // Build the session (pure - no global state mutation)
        let result = self
            .builder
            .build(&timeline_id, session_request, |e| on_event(e.into()))?;

        // Create and store the session (this is the only place we mutate global state)
        let session = PlaybackSession::new(
            result.graph,
            result.spec,
            result.loop_playback,
            result.op_ids,
        );
        self.session_store
            .insert_session(timeline_id.clone(), session);

        if let Ok(logger) = self.logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "timeline_manager",
                &format!(
                    "Timeline '{}' stored in session store ({:.2}s duration)",
                    timeline_id, result.total_duration_seconds
                )
            );
        }

        Ok(BuildGraphResponse {
            operation_count: request.operations.len(),
            total_duration_seconds: result.total_duration_seconds,
            sample_rate,
            channels,
        })
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
    pub fn session_store(&self) -> &Arc<OpPlaybackState> {
        &self.session_store
    }
}
