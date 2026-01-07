// Operation source trait - defines how operations produce samples
//
// This is the core abstraction for the pull-based playback system.
// Operations implement PlayableOp to provide samples at a given time.

use super::types::{AudioSpec, PlaybackError, PlaybackResult, SampleTime};
use std::sync::Arc;

/// Trait that operations must implement to be playable in the timeline.
///
/// This is a pull-based interface: the playback engine calls `render_at`
/// to get samples for a specific time range, rather than the operation
/// pushing samples to a buffer.
///
/// # Design Principles
///
/// - **Time-addressable**: Given any time `t`, the operation can produce
///   the samples that would be playing at that time.
///
/// - **Stateless or minimally stateful**: Operations should ideally be
///   pure functions of time. State (like oscillator phase) should be
///   derivable from time where possible.
///
/// - **Composable**: Operations can wrap other operations to create
///   effect chains and complex processing graphs.
///
/// - **No allocation per call**: The output buffer is provided by the
///   caller, so no heap allocation is needed per render call.
///
/// # Example
///
/// ```ignore

pub trait PlayableOp: Send + Sync {
    /// Fill the output buffer with samples starting at absolute time `t`.
    ///
    /// # Arguments
    /// * `t` - The absolute time in samples to start rendering from
    /// * `out` - The output buffer to fill with interleaved samples
    /// * `spec` - The audio format specification
    ///
    /// # Returns
    /// The number of samples actually written to the buffer.
    /// If this is less than `out.len()`, the operation has reached its end.
    fn render_at(
        &mut self,
        t: SampleTime,
        out: &mut [f32],
        spec: &AudioSpec,
    ) -> PlaybackResult<usize>;

    /// Get the total duration of this operation in samples.
    /// Returns `None` for infinite/generative sources.
    fn duration(&self) -> Option<SampleTime>;

    /// Get the audio specification for this operation.
    fn spec(&self) -> AudioSpec;

    /// Check if this operation is finished at the given time.
    fn is_finished_at(&self, t: SampleTime) -> bool {
        match self.duration() {
            Some(duration) => t >= duration,
            None => false, // Infinite sources are never finished
        }
    }

    /// Seek to a specific position. Default implementation is no-op
    /// for stateless operations.
    fn seek(&mut self, _t: SampleTime) -> PlaybackResult<()> {
        Ok(())
    }

    /// Reset the operation to its initial state.
    fn reset(&mut self) -> PlaybackResult<()> {
        self.seek(SampleTime::new(0))
    }
}

/// A simple sample-based playable operation that wraps pre-loaded audio data.
///
/// This is the most basic implementation of PlayableOp - it simply reads
/// from a buffer of samples at the requested time.

/// A playable operation wrapper that applies a gain adjustment
pub struct GainOp<T: PlayableOp> {
    inner: T,
    gain: f32,
}

impl<T: PlayableOp> GainOp<T> {
    pub fn new(inner: T, gain: f32) -> Self {
        Self { inner, gain }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }
}

impl<T: PlayableOp> PlayableOp for GainOp<T> {
    fn render_at(
        &mut self,
        t: SampleTime,
        out: &mut [f32],
        spec: &AudioSpec,
    ) -> PlaybackResult<usize> {
        let rendered = self.inner.render_at(t, out, spec)?;

        // Apply gain to rendered samples
        for sample in out[..rendered].iter_mut() {
            *sample *= self.gain;
        }

        Ok(rendered)
    }

    fn duration(&self) -> Option<SampleTime> {
        self.inner.duration()
    }

    fn spec(&self) -> AudioSpec {
        self.inner.spec()
    }

    fn seek(&mut self, t: SampleTime) -> PlaybackResult<()> {
        self.inner.seek(t)
    }

    fn reset(&mut self) -> PlaybackResult<()> {
        self.inner.reset()
    }
}

/// Box wrapper for dynamic dispatch of PlayableOp
pub type BoxedPlayableOp = Box<dyn PlayableOp>;

#[cfg(test)]
mod tests {
    use crate::ops::sample::SamplePlayableOp;

    use super::*;

    #[test]
    fn test_sample_playable_op_basic() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let spec = AudioSpec::new(44100, 1);
        let mut op = SamplePlayableOp::new(samples.clone(), spec);

        let mut out = vec![0.0f32; 10];
        let rendered = op.render_at(SampleTime::new(0), &mut out, &spec).unwrap();

        assert_eq!(rendered, 10);
        assert_eq!(out[0], 0.0);
        assert!((out[9] - 0.09).abs() < 0.001);
    }

    #[test]
    fn test_sample_playable_op_seek() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let spec = AudioSpec::new(44100, 1);
        let mut op = SamplePlayableOp::new(samples, spec);

        let mut out = vec![0.0f32; 10];
        let rendered = op.render_at(SampleTime::new(50), &mut out, &spec).unwrap();

        assert_eq!(rendered, 10);
        assert_eq!(out[0], 50.0);
        assert_eq!(out[9], 59.0);
    }

    #[test]
    fn test_sample_playable_op_past_end() {
        let samples: Vec<f32> = vec![1.0; 10];
        let spec = AudioSpec::new(44100, 1);
        let mut op = SamplePlayableOp::new(samples, spec);

        let mut out = vec![0.5f32; 10];
        let rendered = op.render_at(SampleTime::new(20), &mut out, &spec).unwrap();

        assert_eq!(rendered, 0);
        assert!(out.iter().all(|&s| s == 0.0)); // Should be filled with silence
    }

    #[test]
    fn test_gain_op() {
        let samples: Vec<f32> = vec![1.0; 10];
        let spec = AudioSpec::new(44100, 1);
        let inner = SamplePlayableOp::new(samples, spec);
        let mut op = GainOp::new(inner, 0.5);

        let mut out = vec![0.0f32; 10];
        let rendered = op.render_at(SampleTime::new(0), &mut out, &spec).unwrap();

        assert_eq!(rendered, 10);
        assert!(out.iter().all(|&s| (s - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_duration() {
        let samples: Vec<f32> = vec![1.0; 88200]; // 2 seconds at 44100Hz mono
        let spec = AudioSpec::new(44100, 1);
        let op = SamplePlayableOp::new(samples, spec);

        assert_eq!(op.duration().unwrap().samples(), 88200);
        assert!((op.duration().unwrap().to_seconds(44100) - 2.0).abs() < 0.001);
    }
}
