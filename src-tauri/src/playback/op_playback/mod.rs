// Operation-based playback system
//
// This module implements a pull-based, time-addressable playback system
// where operations produce samples on-demand rather than pre-rendering.
//
// Core concepts:
// - PlayableOp: Operations that can produce samples at a given time
// - PlaybackTimeline: Schedules when operations are active
// - TimelineSource: Rodio Source that pulls from the timeline
// - PlaybackContext: Manages mixing and scratch buffers

pub mod context;
pub mod op_source;
pub mod timeline;
pub mod timeline_source;
pub mod types;

pub use op_source::*;
pub use timeline::*;
pub use timeline_source::*;
pub use types::*;
