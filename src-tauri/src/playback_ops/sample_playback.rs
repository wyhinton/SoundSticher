use std::sync::Arc;

use crate::playback::{AudioSpec, PlayableOp, PlaybackResult, SampleTime};

pub struct SamplePlayableOp {
    /// The audio samples (interleaved for multi-channel)
    samples: Arc<Vec<f32>>,

    /// Audio specification
    spec: AudioSpec,

    /// Total duration in samples (per channel)
    duration_samples: u64,
}

impl SamplePlayableOp {
    /// Create a new sample-based playable operation
    pub fn new(samples: Vec<f32>, spec: AudioSpec) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples: Arc::new(samples),
            spec,
            duration_samples,
        }
    }

    /// Create from an existing Arc<Vec<f32>> to share ownership
    pub fn from_arc(samples: Arc<Vec<f32>>, spec: AudioSpec) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples,
            spec,
            duration_samples,
        }
    }

    /// Get a reference to the underlying samples
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

impl PlayableOp for SamplePlayableOp {
    fn render_at(
        &mut self,
        t: SampleTime,
        out: &mut [f32],
        _spec: &AudioSpec,
    ) -> PlaybackResult<usize> {
        let channels = self.spec.channels as usize;
        let start_sample = t.samples() as usize;
        let start_idx = start_sample * channels;

        if start_idx >= self.samples.len() {
            // Past the end - fill with silence
            out.fill(0.0);
            return Ok(0);
        }

        let available = self.samples.len() - start_idx;
        let to_copy = available.min(out.len());

        out[..to_copy].copy_from_slice(&self.samples[start_idx..start_idx + to_copy]);

        // Fill remaining with silence if we reached the end
        if to_copy < out.len() {
            out[to_copy..].fill(0.0);
        }

        Ok(to_copy)
    }

    fn duration(&self) -> Option<SampleTime> {
        Some(SampleTime::new(self.duration_samples))
    }

    fn spec(&self) -> AudioSpec {
        self.spec
    }
}
