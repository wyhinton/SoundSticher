// Operation source trait - defines how operations produce samples
//
// This is the core abstraction for the pull-based playback system.
// Operations implement PlayableOp to provide samples at a given time.

use super::types::{AudioSpec, PlaybackResult, SampleTime};

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
