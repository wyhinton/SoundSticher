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

impl PlayableOp for SampleOp {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_op_basic() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let spec = AudioSpec::new(44100, 1);
        let mut op = SampleOp::new(samples.clone(), spec);

        let mut out = vec![0.0f32; 10];
        let rendered = op.render_at(SampleTime::new(0), &mut out, &spec).unwrap();

        assert_eq!(rendered, 10);
        assert_eq!(out[0], 0.0);
        assert!((out[9] - 0.09).abs() < 0.001);
    }

    #[test]
    fn test_sample_op_seek() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let spec = AudioSpec::new(44100, 1);
        let mut op = SampleOp::new(samples, spec);

        let mut out = vec![0.0f32; 10];
        let rendered = op.render_at(SampleTime::new(50), &mut out, &spec).unwrap();

        assert_eq!(rendered, 10);
        assert_eq!(out[0], 50.0);
        assert_eq!(out[9], 59.0);
    }

    #[test]
    fn test_sample_op_past_end() {
        let samples: Vec<f32> = vec![1.0; 10];
        let spec = AudioSpec::new(44100, 1);
        let mut op = SampleOp::new(samples, spec);

        let mut out = vec![0.5f32; 10];
        let rendered = op.render_at(SampleTime::new(20), &mut out, &spec).unwrap();

        assert_eq!(rendered, 0);
        assert!(out.iter().all(|&s| s == 0.0)); // Should be filled with silence
    }

    #[test]
    fn test_sample_op_with_name() {
        let samples: Vec<f32> = vec![1.0; 10];
        let spec = AudioSpec::new(44100, 1);
        let op = SampleOp::with_name(samples, spec, "test_audio".to_string());

        assert_eq!(op.name(), Some("test_audio"));
    }

    #[test]
    fn test_duration() {
        let samples: Vec<f32> = vec![1.0; 88200]; // 2 seconds at 44100Hz mono
        let spec = AudioSpec::new(44100, 1);
        let op = SampleOp::new(samples, spec);

        assert_eq!(op.duration().unwrap().samples(), 88200);
        assert!((op.duration().unwrap().to_seconds(44100) - 2.0).abs() < 0.001);
    }
}
