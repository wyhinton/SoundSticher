// TimelinePlaybackBuilder - Trait-based builder abstraction
//
// This module defines the `TimelinePlaybackBuilder` trait that decouples
// session construction from session registration. Builders are pure:
// they return a `PlaybackSession` without touching global state.
//
// The `TimelinePlaybackManager` is the only consumer that decides
// where sessions live and when they replace existing ones.

use crate::logging::LoggingService;
use crate::playback::session_builder::SessionBuildEvent;
use crate::sample_cache::SampleCacheService;
use crate::timeline_playback_commands::PlaybackSession;
use crate::timeline_playback_commands::TimelineId;
use std::sync::{Arc, Mutex};

/// Bundles shared dependencies needed by all builders.
///
/// Passed into `TimelinePlaybackBuilder::build()` so that builders
/// don't need to store or clone these services themselves.
pub struct BuildContext {
    pub sample_cache: Arc<SampleCacheService>,
    pub logging_service: Arc<Mutex<LoggingService>>,
}

/// The source-specific payload a builder needs to construct a session.
///
/// Each variant maps 1:1 to a `TimelineSource` variant, but carries
/// only the data the builder requires (no Tauri types, no state refs).
#[derive(Debug, Clone)]
pub enum BuildPlaybackRequest {
    /// Build from an operation graph
    OpGraph(crate::op_playback_commands::BuildOpPlaybackGraphRequest),

    /// Build from a single audio file (future)
    AudioFile { file_path: String },

    /// Build from a live input device (future)
    LiveInput { device_id: String },
}

/// Trait for building `PlaybackSession`s without side effects.
///
/// Implementors:
/// - Parse source-specific input
/// - Load/decode audio data
/// - Construct a `PlaybackGraph`
/// - Return a fully formed `PlaybackSession`
///
/// They do **not**:
/// - Insert sessions into `AppTimelinePlaybackState`
/// - Emit Tauri events (they call a generic callback instead)
/// - Touch any global mutable state
pub trait TimelinePlaybackBuilder: Send + Sync {
    /// Build a `PlaybackSession` for the given timeline.
    ///
    /// # Arguments
    /// * `timeline_id`  – Logical ID of the timeline being built
    /// * `request`      – Source-specific build payload
    /// * `ctx`          – Shared services (cache, logging)
    /// * `on_event`     – Progress callback (builder → manager → Tauri channel)
    ///
    /// # Returns
    /// A fully constructed `PlaybackSession` ready for registration,
    /// along with a `BuildResult` carrying metadata for the caller.
    fn build(
        &self,
        timeline_id: &TimelineId,
        request: &BuildPlaybackRequest,
        ctx: &BuildContext,
        on_event: &dyn Fn(SessionBuildEvent),
    ) -> Result<BuildResult, String>;
}

/// Metadata returned alongside the `PlaybackSession`.
pub struct BuildResult {
    /// The constructed session, ready for insertion into state
    pub session: PlaybackSession,
    /// Number of operations in the graph
    pub operation_count: usize,
    /// Total duration in seconds
    pub total_duration_seconds: f64,
    /// Sample rate used
    pub sample_rate: u32,
    /// Channel count used
    pub channels: u16,
}

// ─── OpGraphPlaybackBuilder ───────────────────────────────────────────

use crate::playback::session_builder::{OpPlaybackSessionBuilder, SessionBuildRequest};

/// Builds a `PlaybackSession` from an operation-graph request.
///
/// This is the "main" builder — it wraps `OpPlaybackSessionBuilder`
/// (which does the heavy lifting) and converts the result into a
/// `PlaybackSession` without touching global state.
pub struct OpGraphPlaybackBuilder;

impl TimelinePlaybackBuilder for OpGraphPlaybackBuilder {
    fn build(
        &self,
        timeline_id: &TimelineId,
        request: &BuildPlaybackRequest,
        ctx: &BuildContext,
        on_event: &dyn Fn(SessionBuildEvent),
    ) -> Result<BuildResult, String> {
        let graph_request = match request {
            BuildPlaybackRequest::OpGraph(req) => req,
            other => {
                return Err(format!(
                    "OpGraphPlaybackBuilder received unsupported request type: {:?}",
                    std::mem::discriminant(other)
                ))
            }
        };

        let sample_rate = graph_request.sample_rate.unwrap_or(44100);
        let channels = graph_request.channels.unwrap_or(2);

        // Convert to the internal session builder request
        let session_request: SessionBuildRequest = graph_request.clone().into();

        // Delegate to the pure session builder
        let session_builder = OpPlaybackSessionBuilder::new(
            Arc::clone(&ctx.sample_cache),
            Arc::clone(&ctx.logging_service),
        );

        let result = session_builder.build(timeline_id, session_request, on_event)?;

        // Wrap in a PlaybackSession (no state mutation)
        let session = PlaybackSession::new(
            result.graph,
            result.spec,
            result.loop_playback,
            result.op_ids,
        );

        Ok(BuildResult {
            session,
            operation_count: graph_request.operations.len(),
            total_duration_seconds: result.total_duration_seconds,
            sample_rate,
            channels,
        })
    }
}
