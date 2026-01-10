// Playback context - manages scratch buffers and mixing
//
// The PlaybackContext is responsible for:
// - Maintaining scratch buffers for intermediate processing
// - Mixing multiple active operations together
// - Managing memory efficiently without per-callback allocations

use super::op_source::PlayableOp;
use super::types::{AudioSpec, PlaybackResult, SampleTime};

/// Default block size for rendering (in frames, not samples)
pub const DEFAULT_BLOCK_SIZE: usize = 512;

/// Maximum number of operations that can be mixed simultaneously
pub const MAX_CONCURRENT_OPS: usize = 32;

/// Playback context that manages scratch buffers and mixing.
///
/// This is designed to be reused across audio callbacks without
/// heap allocation. The scratch buffers are pre-allocated and
/// reused for each render pass.
pub struct PlaybackContext {
    /// Scratch buffer for individual operation rendering
    scratch: Vec<f32>,

    /// Mix buffer for accumulating multiple operations
    mix: Vec<f32>,

    /// Audio specification
    spec: AudioSpec,

    /// Block size in frames (samples per channel)
    block_size: usize,
}

impl PlaybackContext {
    /// Create a new playback context with the given specification
    pub fn new(spec: AudioSpec) -> Self {
        Self::with_block_size(spec, DEFAULT_BLOCK_SIZE)
    }

    /// Create a new playback context with a custom block size
    pub fn with_block_size(spec: AudioSpec, block_size: usize) -> Self {
        let buffer_size = block_size * spec.channels as usize;
        Self {
            scratch: vec![0.0; buffer_size],
            mix: vec![0.0; buffer_size],
            spec,
            block_size,
        }
    }

    /// Get the audio specification
    pub fn spec(&self) -> AudioSpec {
        self.spec
    }

    /// Get the block size in frames
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Get the block size in samples (frames * channels)
    pub fn block_samples(&self) -> usize {
        self.block_size * self.spec.channels as usize
    }

    /// Resize the internal buffers if needed
    pub fn ensure_capacity(&mut self, frames: usize) {
        let needed = frames * self.spec.channels as usize;
        if self.scratch.len() < needed {
            self.scratch.resize(needed, 0.0);
            self.mix.resize(needed, 0.0);
            self.block_size = frames;
        }
    }

    /// Clear the mix buffer
    pub fn clear_mix(&mut self) {
        self.mix.fill(0.0);
    }

    /// Clear the scratch buffer
    pub fn clear_scratch(&mut self) {
        self.scratch.fill(0.0);
    }

    /// Get a mutable reference to the scratch buffer for a given frame count
    pub fn scratch_buffer(&mut self, frames: usize) -> &mut [f32] {
        let samples = frames * self.spec.channels as usize;
        self.ensure_capacity(frames);
        &mut self.scratch[..samples]
    }

    /// Get a reference to the mix buffer
    pub fn mix_buffer(&self) -> &[f32] {
        &self.mix
    }

    /// Get a mutable reference to the mix buffer
    pub fn mix_buffer_mut(&mut self) -> &mut [f32] {
        &mut self.mix
    }

    /// Accumulate the scratch buffer into the mix buffer with a gain factor
    ///
    /// This method copies samples from scratch to mix with gain applied.
    /// Use this after rendering into scratch_buffer to avoid borrow conflicts.
    pub fn accumulate_scratch_to_mix(&mut self, samples: usize, gain: f32) {
        let samples = samples.min(self.scratch.len()).min(self.mix.len());
        for i in 0..samples {
            self.mix[i] += self.scratch[i] * gain;
        }
    }

    /// Render a single operation into the mix buffer at the given time
    ///
    /// Returns the number of samples rendered
    pub fn render_op_into_mix(
        &mut self,
        op: &mut dyn PlayableOp,
        t: SampleTime,
        frames: usize,
    ) -> PlaybackResult<usize> {
        let samples = frames * self.spec.channels as usize;
        self.ensure_capacity(frames);

        // Clear scratch and render operation into it
        self.scratch[..samples].fill(0.0);
        let rendered = op.render_at(t, &mut self.scratch[..samples], &self.spec)?;

        // Accumulate into mix
        for i in 0..rendered {
            self.mix[i] += self.scratch[i];
        }

        Ok(rendered)
    }

    /// Render multiple operations into the mix buffer at the given time
    ///
    /// Each operation in the slice is rendered and accumulated into the mix.
    pub fn render_ops_into_mix(
        &mut self,
        ops: &mut [&mut dyn PlayableOp],
        t: SampleTime,
        frames: usize,
    ) -> PlaybackResult<usize> {
        let samples = frames * self.spec.channels as usize;
        self.ensure_capacity(frames);
        self.clear_mix();

        let mut max_rendered = 0;

        for op in ops.iter_mut() {
            // Clear scratch and render operation into it
            self.scratch[..samples].fill(0.0);
            let rendered = op.render_at(t, &mut self.scratch[..samples], &self.spec)?;
            max_rendered = max_rendered.max(rendered);

            // Accumulate into mix
            for i in 0..rendered {
                self.mix[i] += self.scratch[i];
            }
        }

        Ok(max_rendered)
    }

    /// Apply a gain to the mix buffer
    pub fn apply_gain(&mut self, gain: f32, samples: usize) {
        for sample in self.mix[..samples].iter_mut() {
            *sample *= gain;
        }
    }

    /// Soft clip the mix buffer to prevent harsh clipping
    pub fn soft_clip(&mut self, samples: usize) {
        for sample in self.mix[..samples].iter_mut() {
            // Simple soft clipper: tanh-like curve
            if *sample > 1.0 {
                *sample = 1.0 - 1.0 / (*sample + 1.0);
            } else if *sample < -1.0 {
                *sample = -1.0 + 1.0 / (-*sample + 1.0);
            }
        }
    }

    /// Hard clip the mix buffer
    pub fn hard_clip(&mut self, samples: usize) {
        for sample in self.mix[..samples].iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    /// Copy mix buffer to output buffer
    pub fn copy_to_output(&self, output: &mut [f32], samples: usize) {
        let to_copy = samples.min(output.len()).min(self.mix.len());
        output[..to_copy].copy_from_slice(&self.mix[..to_copy]);
    }
}
