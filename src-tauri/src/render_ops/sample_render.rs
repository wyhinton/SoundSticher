// Sample-based playable operation
//
// This operation simply reads from a pre-loaded buffer of audio samples.

use crate::playback::op_playback::{AudioSpec, PlayableOp, PlaybackResult, SampleTime};
use std::sync::Arc;

/// A simple sample-based playable operation that wraps pre-loaded audio data.
///
/// This is the most basic implementation of PlayableOp - it simply reads
/// from a buffer of samples at the requested time.
pub struct SampleOp {
    /// The audio samples (interleaved for multi-channel)
    samples: Arc<Vec<f32>>,

    /// Audio specification
    spec: AudioSpec,

    /// Total duration in samples (per channel)
    duration_samples: u64,

    /// Optional name for logging/debugging
    name: Option<String>,
}

impl SampleOp {
    /// Create a new sample-based playable operation
    pub fn new(samples: Vec<f32>, spec: AudioSpec) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples: Arc::new(samples),
            spec,
            duration_samples,
            name: None,
        }
    }

    /// Create a new sample operation with a name
    pub fn with_name(samples: Vec<f32>, spec: AudioSpec, name: String) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples: Arc::new(samples),
            spec,
            duration_samples,
            name: Some(name),
        }
    }

    /// Create from an existing Arc<Vec<f32>> to share ownership
    pub fn from_arc(samples: Arc<Vec<f32>>, spec: AudioSpec) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples,
            spec,
            duration_samples,
            name: None,
        }
    }

    /// Create from an existing Arc<Vec<f32>> with a name
    pub fn from_arc_with_name(samples: Arc<Vec<f32>>, spec: AudioSpec, name: String) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples,
            spec,
            duration_samples,
            name: Some(name),
        }
    }

    /// Get a reference to the underlying samples
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Get the name of this operation, if set
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Set the name of this operation
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}
