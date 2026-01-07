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

#[cfg(test)]
mod tests {
    use super::super::op_source::SamplePlayableOp;
    use super::*;

    #[test]
    fn test_context_creation() {
        let spec = AudioSpec::cd_quality();
        let ctx = PlaybackContext::new(spec);

        assert_eq!(ctx.spec().sample_rate, 44100);
        assert_eq!(ctx.spec().channels, 2);
        assert_eq!(ctx.block_size(), DEFAULT_BLOCK_SIZE);
    }

    #[test]
    fn test_render_single_op() {
        let spec = AudioSpec::new(44100, 1);
        let mut ctx = PlaybackContext::with_block_size(spec, 10);

        let samples: Vec<f32> = vec![0.5; 20];
        let mut op = SamplePlayableOp::new(samples, spec);

        ctx.clear_mix();
        let rendered = ctx
            .render_op_into_mix(&mut op, SampleTime::new(0), 10)
            .unwrap();

        assert_eq!(rendered, 10);
        assert!(ctx.mix_buffer()[..10]
            .iter()
            .all(|&s| (s - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_render_multiple_ops_accumulate() {
        let spec = AudioSpec::new(44100, 1);
        let mut ctx = PlaybackContext::with_block_size(spec, 10);

        let samples1: Vec<f32> = vec![0.3; 20];
        let samples2: Vec<f32> = vec![0.2; 20];
        let mut op1 = SamplePlayableOp::new(samples1, spec);
        let mut op2 = SamplePlayableOp::new(samples2, spec);

        let mut ops: Vec<&mut dyn PlayableOp> = vec![&mut op1, &mut op2];
        let rendered = ctx
            .render_ops_into_mix(&mut ops, SampleTime::new(0), 10)
            .unwrap();

        assert_eq!(rendered, 10);
        // Both ops should be accumulated: 0.3 + 0.2 = 0.5
        assert!(ctx.mix_buffer()[..10]
            .iter()
            .all(|&s| (s - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_apply_gain() {
        let spec = AudioSpec::new(44100, 1);
        let mut ctx = PlaybackContext::with_block_size(spec, 10);

        // Fill mix with 1.0
        ctx.mix[..10].fill(1.0);

        ctx.apply_gain(0.5, 10);

        assert!(ctx.mix_buffer()[..10]
            .iter()
            .all(|&s| (s - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_hard_clip() {
        let spec = AudioSpec::new(44100, 1);
        let mut ctx = PlaybackContext::with_block_size(spec, 4);

        ctx.mix[0] = 2.0;
        ctx.mix[1] = -2.0;
        ctx.mix[2] = 0.5;
        ctx.mix[3] = -0.5;

        ctx.hard_clip(4);

        assert_eq!(ctx.mix[0], 1.0);
        assert_eq!(ctx.mix[1], -1.0);
        assert_eq!(ctx.mix[2], 0.5);
        assert_eq!(ctx.mix[3], -0.5);
    }

    #[test]
    fn test_ensure_capacity() {
        let spec = AudioSpec::new(44100, 2);
        let mut ctx = PlaybackContext::with_block_size(spec, 10);

        assert_eq!(ctx.scratch.len(), 20); // 10 frames * 2 channels

        ctx.ensure_capacity(100);

        assert_eq!(ctx.scratch.len(), 200); // 100 frames * 2 channels
        assert_eq!(ctx.mix.len(), 200);
    }
}
