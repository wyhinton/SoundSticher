// OpPlaybackSessionBuilder - Pure session builder (no global state mutation)
//
// This module provides a builder that constructs PlaybackSessions without
// touching any global state. The caller decides what to do with the result.

use crate::logging::{LogSystem, LoggingService};
use crate::playback::op_playback::{
    AudioSpec, PlayableOp, PlaybackGraph, PlaybackOpId, SampleTime,
};
use crate::playback_ops::merge_playback::MergePlaybackOp;
use crate::playback_ops::sample_playback::SamplePlayableOp;
use crate::sample_cache::SampleCacheService;
use crate::timeline_playback_commands::TimelineId;
use crate::{log_debug, log_info};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Events emitted during session building
#[derive(Clone, Debug)]
pub enum SessionBuildEvent {
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

/// Result of building a session
pub struct SessionBuildResult {
    /// The constructed playback graph
    pub graph: Arc<PlaybackGraph>,
    /// Audio specification
    pub spec: AudioSpec,
    /// Whether looping is enabled
    pub loop_playback: bool,
    /// Mapping of operation names to their IDs
    pub op_ids: HashMap<String, PlaybackOpId>,
    /// Total duration in seconds
    pub total_duration_seconds: f64,
}

/// The type of playback operation
#[derive(Debug, Clone, Default)]
pub enum OpType {
    #[default]
    Sample,
    Merge,
}

impl From<crate::op_playback_commands::OpType> for OpType {
    fn from(op_type: crate::op_playback_commands::OpType) -> Self {
        match op_type {
            crate::op_playback_commands::OpType::Sample => OpType::Sample,
            crate::op_playback_commands::OpType::Merge => OpType::Merge,
        }
    }
}

/// Child input for a merge operation
#[derive(Debug, Clone)]
pub struct MergeInput {
    pub file_path: Option<String>,
    pub samples: Option<Vec<f32>>,
    pub offset: f64,
    pub gain: Option<f32>,
}

impl From<crate::op_playback_commands::MergeInputRequest> for MergeInput {
    fn from(req: crate::op_playback_commands::MergeInputRequest) -> Self {
        Self {
            file_path: req.file_path,
            samples: req.samples,
            offset: req.offset,
            gain: req.gain,
        }
    }
}

/// Operation request for the builder
#[derive(Debug, Clone)]
pub struct OperationRequest {
    pub name: String,
    pub op_type: OpType,
    pub file_path: Option<String>,
    pub samples: Option<Vec<f32>>,
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub gain: Option<f32>,
    pub inputs: Option<Vec<MergeInput>>,
}

impl From<crate::op_playback_commands::AddOpRequest> for OperationRequest {
    fn from(req: crate::op_playback_commands::AddOpRequest) -> Self {
        Self {
            name: req.name,
            op_type: req.op_type.into(),
            file_path: req.file_path,
            samples: req.samples,
            start_time: req.start_time,
            end_time: req.end_time,
            gain: req.gain,
            inputs: req
                .inputs
                .map(|inputs| inputs.into_iter().map(|i| i.into()).collect()),
        }
    }
}

/// Request to build a session
#[derive(Debug, Clone)]
pub struct SessionBuildRequest {
    pub operations: Vec<OperationRequest>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub loop_playback: Option<bool>,
}

impl From<crate::op_playback_commands::BuildOpPlaybackGraphRequest> for SessionBuildRequest {
    fn from(req: crate::op_playback_commands::BuildOpPlaybackGraphRequest) -> Self {
        Self {
            operations: req.operations.into_iter().map(|op| op.into()).collect(),
            sample_rate: req.sample_rate,
            channels: req.channels,
            loop_playback: req.loop_playback,
        }
    }
}

/// Pure session builder - no global state mutation
///
/// This builder constructs a PlaybackSession from a request without
/// inserting it into any global state. The caller is responsible for
/// deciding what to do with the result (store it, discard it, etc.).
pub struct OpPlaybackSessionBuilder {
    sample_cache: Arc<SampleCacheService>,
    logging_service: Arc<Mutex<LoggingService>>,
}

impl OpPlaybackSessionBuilder {
    pub fn new(
        sample_cache: Arc<SampleCacheService>,
        logging_service: Arc<Mutex<LoggingService>>,
    ) -> Self {
        Self {
            sample_cache,
            logging_service,
        }
    }

    /// Build a session for the given timeline
    ///
    /// This method:
    /// - Builds a PlaybackGraph
    /// - Creates operation mappings
    /// - Emits progress events via the callback
    /// - Returns the session result (does NOT insert into global state)
    pub fn build<F>(
        &self,
        timeline_id: &TimelineId,
        request: SessionBuildRequest,
        on_event: F,
    ) -> Result<SessionBuildResult, String>
    where
        F: Fn(SessionBuildEvent),
    {
        if let Ok(logger) = self.logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "session_builder",
                &format!(
                    "Building session for timeline '{}' with {} operations",
                    timeline_id,
                    request.operations.len()
                )
            );
        }

        let total_graph_ops = self.count_operations(&request)?;

        // Emit started event
        on_event(SessionBuildEvent::Started {
            timeline_id: timeline_id.clone(),
            operation_count: total_graph_ops,
        });

        let sample_rate = request.sample_rate.unwrap_or(44100);
        let channels = request.channels.unwrap_or(2);
        let spec = AudioSpec::new(sample_rate, channels);
        let loop_playback = request.loop_playback.unwrap_or(true);

        // Create new graph
        let graph = Arc::new(PlaybackGraph::new(spec));
        let mut op_ids = HashMap::new();

        for (index, op_request) in request.operations.iter().enumerate() {
            let op = self.create_operation(op_request, spec, sample_rate, channels)?;

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
            on_event(SessionBuildEvent::Progress {
                timeline_id: timeline_id.clone(),
                operation_name: op_request.name.clone(),
                operation_index: index,
                total_operations: total_graph_ops,
                duration_seconds: op_duration_seconds,
            });

            if let Ok(logger) = self.logging_service.lock() {
                log_debug!(
                    logger,
                    LogSystem::Playback,
                    "session_builder",
                    &format!(
                        "Added operation '{}' to timeline '{}' (id={:?}, start={:.2}s, end={:.2}s, duration={:.2}s)",
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

        // Emit finished event
        on_event(SessionBuildEvent::Finished {
            timeline_id: timeline_id.clone(),
            operation_count: request.operations.len(),
            total_duration_seconds,
            sample_rate,
            channels,
        });

        if let Ok(logger) = self.logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Playback,
                "session_builder",
                &format!(
                    "Session built for timeline '{}': {} operations, {:.2}s total duration",
                    timeline_id, total_graph_ops, total_duration_seconds
                )
            );
        }

        Ok(SessionBuildResult {
            graph,
            spec,
            loop_playback,
            op_ids,
            total_duration_seconds,
        })
    }

    /// Create a playable operation from the request
    fn create_operation(
        &self,
        op_request: &OperationRequest,
        spec: AudioSpec,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Box<dyn PlayableOp>, String> {
        match op_request.op_type {
            OpType::Sample => {
                let samples_arc: Arc<Vec<f32>> = if let Some(ref samples) = op_request.samples {
                    Arc::new(samples.clone())
                } else if let Some(ref file_path) = op_request.file_path {
                    let path = std::path::PathBuf::from(file_path);
                    let buffer = self.sample_cache.get_or_load(path, sample_rate, channels)?;
                    Arc::clone(&buffer.data)
                } else {
                    return Err(format!(
                        "Sample operation '{}' must have either 'samples' or 'filePath'",
                        op_request.name
                    ));
                };

                Ok(Box::new(SamplePlayableOp::from_arc(samples_arc, spec)))
            }
            OpType::Merge => {
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
                    let samples_arc: Arc<Vec<f32>> = if let Some(ref samples) = input.samples {
                        Arc::new(samples.clone())
                    } else if let Some(ref file_path) = input.file_path {
                        let path = std::path::PathBuf::from(file_path);
                        let buffer = self.sample_cache.get_or_load(path, sample_rate, channels)?;
                        Arc::clone(&buffer.data)
                    } else {
                        return Err(format!(
                            "Merge input {} in operation '{}' must have either 'samples' or 'filePath'",
                            i, op_request.name
                        ));
                    };

                    let child_op = SamplePlayableOp::from_arc(samples_arc, spec);
                    let offset = SampleTime::from_seconds(input.offset, sample_rate);
                    builder = builder.add_input(Box::new(child_op), offset);
                }

                Ok(Box::new(builder.build()))
            }
        }
    }

    /// Count total operations including merge inputs
    fn count_operations(&self, request: &SessionBuildRequest) -> Result<usize, String> {
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

                    count += 1; // The merge op itself
                    count += inputs.len(); // Each input
                }
            }
        }

        Ok(count)
    }
}
