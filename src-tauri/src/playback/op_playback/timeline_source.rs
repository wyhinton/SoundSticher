//! # Timeline Source - Real-time Audio Playback Engine
//!
//! This module provides the `TimelineSource` - a Rodio-compatible audio source that renders
//! complex operation graphs in real-time with zero intermediate files or full buffer allocations.
//!
//! ## 🎵 What is TimelineSource?
//!
//! Think of TimelineSource as a **smart audio player** that can play multiple audio files
//! simultaneously, apply effects, and mix everything together - all happening **live** as
//! you listen, without creating any temporary files.
//!
//! Instead of:
//! 1. ❌ Pre-rendering entire timeline to a file
//! 2. ❌ Loading everything into memory
//! 3. ❌ Playing the rendered file
//!
//! We do:
//! 1. ✅ Stream small chunks of audio (512 samples at a time)
//! 2. ✅ Render operations **on-demand** as playback progresses
//! 3. ✅ Mix multiple operations together in real-time
//! 4. ✅ Support seeking, looping, and live updates
//!
//! ## 🔄 How Playback Works - The Big Picture
//!
//! ### Step 1: Frontend Request
//! ```
//! User clicks "Play" → Frontend calls backend → OpPlaybackService receives request
//! ```
//!
//! ### Step 2: Graph Building
//! ```
//! OpPlaybackService → PlaybackGraph → Timeline + OperationRegistry
//!                                  ↓
//!                              TimelineSource (this file!)
//! ```
//!
//! ### Step 3: Real-time Rendering (this is where TimelineSource shines!)
//! ```
//! Rodio requests samples → TimelineSource.next()
//!                       ↓
//!                    fill_buffer() - renders 512 samples
//!                       ↓
//!                    Timeline.get_active_events() - "what operations should play now?"
//!                       ↓
//!                    For each active operation:
//!                      - Operation.render_at() - "give me 512 samples starting at time X"
//!                      - Mix all operations together
//!                       ↓
//!                    Return mixed audio to Rodio → speakers! 🔊
//! ```
//!
//! ## 🧩 Key Components in the Playback Pipeline
//!
//! 1. **PlaybackGraph** (`timeline.rs`)
//!    - Contains the Timeline (when operations start/stop)
//!    - Contains the OperationRegistry (how to render each operation)
//!
//! 2. **Timeline** (`timeline.rs`)
//!    - Knows which operations should be playing at any given time
//!    - `get_active_events(position)` returns what's currently active
//!
//! 3. **Operations** (`operation.rs`)
//!    - SampleOp: plays audio files
//!    - MergeOp: combines multiple operations
//!    - Each has `render_at(time, buffer)` method
//!
//! 4. **TimelineSource** (this file!)
//!    - Bridges the gap between our operation system and Rodio
//!    - Implements `Iterator<Item=f32>` and `rodio::Source`
//!    - Renders operations in small chunks (512 samples = ~11ms at 44.1kHz)
//!
//! ## 🚀 Performance Features
//!
//! - **Block-based rendering**: Only renders 512 samples at a time (low latency)
//! - **Zero intermediate files**: Everything happens in memory, in real-time
//! - **Lazy evaluation**: Operations only run when their audio is actually needed
//! - **Memory efficient**: No full-timeline buffers, just small working buffers
//! - **Seekable**: Jump to any position instantly without re-rendering everything
//! - **Loopable**: Seamlessly restart from beginning when reaching the end
//!
//! ## 🎛️ Usage Examples
//!
//! ```rust
//! // Basic playback
//! let source = TimelineSource::new(graph, AudioSpec::cd_quality());
//! rodio_sink.append(source);
//!
//! // With looping
//! let source = TimelineSourceBuilder::new()
//!     .looping(true)
//!     .start_position_seconds(10.0)
//!     .build(graph);
//!
//! // Seeking during playback
//! source.seek_to_seconds(30.0);
//! ```
//!
//! ## 🔧 Internal Architecture
//!
//! The TimelineSource maintains several key pieces of state:
//! - **position**: Current playback position in samples
//! - **buffer**: Small internal buffer (512 samples × channels)
//! - **context**: Mixing context for combining operations
//! - **graph**: The PlaybackGraph containing timeline and operations
//!
//! The rendering loop works like this:
//! 1. Rodio asks for next sample via `Iterator::next()`
//! 2. If buffer is empty, call `fill_buffer()`
//! 3. `fill_buffer()` asks timeline "what's active at current position?"
//! 4. For each active operation, render 512 samples into scratch buffer
//! 5. Mix all operations together with their gain settings
//! 6. Store result in internal buffer
//! 7. Return samples one by one until buffer is empty
//! 8. Repeat!

use super::context::PlaybackContext;
use super::timeline::PlaybackGraph;
use super::types::{AudioSpec, SampleTime};
use rodio::Source;
use std::sync::Arc;
use std::time::Duration;
/// Block size for internal buffering (in frames)
const BLOCK_SIZE: usize = 512;

/// Rodio-compatible source that renders from a PlaybackGraph
pub struct TimelineSource {
    /// The playback graph (timeline + operations)
    graph: Arc<PlaybackGraph>,

    /// Current playback position in samples
    position: SampleTime,

    /// Audio specification
    spec: AudioSpec,

    /// Internal buffer for block-based rendering
    buffer: Vec<f32>,

    /// Current position within the internal buffer
    buffer_pos: usize,

    /// Number of valid samples in the buffer
    buffer_len: usize,

    /// Playback context for mixing
    context: PlaybackContext,

    /// Whether playback should loop
    loop_playback: bool,

    /// Whether playback is finished
    finished: bool,
}

impl TimelineSource {
    /// Create a new timeline source from a playback graph
    pub fn new(graph: Arc<PlaybackGraph>, spec: AudioSpec) -> Self {
        let buffer_size = BLOCK_SIZE * spec.channels as usize;
        Self {
            graph,
            position: SampleTime::new(0),
            spec,
            buffer: vec![0.0; buffer_size],
            buffer_pos: 0,
            buffer_len: 0,
            context: PlaybackContext::with_block_size(spec, BLOCK_SIZE),
            loop_playback: false,
            finished: false,
        }
    }

    /// Create a new timeline source with looping enabled
    pub fn new_looping(graph: Arc<PlaybackGraph>, spec: AudioSpec) -> Self {
        let mut source = Self::new(graph, spec);
        source.loop_playback = true;
        source
    }

    /// Set whether playback should loop
    pub fn set_looping(&mut self, loop_playback: bool) {
        self.loop_playback = loop_playback;
        if loop_playback {
            self.finished = false;
        }
    }

    /// Get the current playback position in samples
    pub fn position(&self) -> SampleTime {
        self.position
    }

    /// Get the current playback position in seconds
    pub fn position_seconds(&self) -> f64 {
        self.position.to_seconds(self.spec.sample_rate)
    }

    /// Seek to a specific position in samples
    pub fn seek(&mut self, position: SampleTime) {
        self.position = position;
        self.buffer_pos = 0;
        self.buffer_len = 0;
        self.finished = false;
    }

    /// Seek to a specific position in seconds
    pub fn seek_to_seconds(&mut self, seconds: f64) {
        self.seek(SampleTime::from_seconds(seconds, self.spec.sample_rate));
    }

    /// Check if playback is finished
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Get the total duration
    pub fn duration_samples(&self) -> SampleTime {
        self.graph.duration()
    }

    /// Fill the internal buffer by rendering from the graph
    fn fill_buffer(&mut self) {
        let duration = self.graph.duration();

        // TODO: Add proper timeline logging integration
        #[cfg(debug_assertions)]
        {
            println!(
                "Timeline: Filling buffer at position: {:.3}s",
                self.position.to_seconds(self.spec.sample_rate)
            );
        }

        // Check if we've reached the end
        if self.position >= duration {
            if self.loop_playback {
                self.position = SampleTime::new(0);
            } else {
                self.finished = true;
                self.buffer.fill(0.0);
                self.buffer_len = 0;
                return;
            }
        }

        // Calculate how many frames we can render
        let remaining_samples = duration.samples().saturating_sub(self.position.samples());
        let frames_to_render = (remaining_samples as usize).min(BLOCK_SIZE);

        if frames_to_render == 0 {
            self.finished = !self.loop_playback;
            self.buffer.fill(0.0);
            self.buffer_len = 0;
            return;
        }

        // Clear and prepare context
        self.context.clear_mix();

        // Get timeline and registry locks
        let timeline = self.graph.timeline.read().unwrap();
        let mut registry = self.graph.registry.write().unwrap();

        // Get active events at current position
        let active_events = timeline.get_active_events(self.position);

        // Render each active operation and accumulate
        for event in active_events {
            if let Some(op) = registry.get_mut(event.id) {
                // Calculate local time within the operation
                let local_time = event.to_local_time(self.position);

                // Render into scratch buffer and get the rendered sample count
                let _samples = frames_to_render * self.spec.channels as usize;
                let rendered = {
                    let scratch = self.context.scratch_buffer(frames_to_render);
                    scratch.fill(0.0);
                    op.render_at(local_time, scratch, &self.spec)
                };

                // Now accumulate scratch into mix (separate borrow)
                if let Ok(rendered_count) = rendered {
                    let gain = event.gain;
                    self.context.accumulate_scratch_to_mix(rendered_count, gain);
                }
            }
        }

        // Copy mix to internal buffer
        let samples = frames_to_render * self.spec.channels as usize;
        self.buffer[..samples].copy_from_slice(&self.context.mix_buffer()[..samples]);
        self.buffer_len = samples;
        self.buffer_pos = 0;

        // Advance position
        self.position = self.position.add_samples(frames_to_render as u64);
    }
}

impl Iterator for TimelineSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if we need to fill the buffer
        if self.buffer_pos >= self.buffer_len {
            if self.finished {
                return None;
            }
            self.fill_buffer();

            // Check again after filling
            if self.buffer_len == 0 {
                if self.finished {
                    return None;
                }
                // Return silence if no active operations
                return Some(0.0);
            }
        }

        let sample = self.buffer[self.buffer_pos];
        self.buffer_pos += 1;
        Some(sample)
    }
}

impl Source for TimelineSource {
    fn current_frame_len(&self) -> Option<usize> {
        // Return remaining frames in buffer, or None for infinite/streaming
        if self.finished {
            Some(0)
        } else if self.loop_playback {
            None // Infinite when looping
        } else {
            let duration = self.graph.duration();
            let remaining = duration.samples().saturating_sub(self.position.samples());
            Some(remaining as usize)
        }
    }

    fn channels(&self) -> u16 {
        self.spec.channels
    }

    fn sample_rate(&self) -> u32 {
        self.spec.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        if self.loop_playback {
            None
        } else {
            let duration_samples = self.graph.duration();
            Some(Duration::from_secs_f64(
                duration_samples.to_seconds(self.spec.sample_rate),
            ))
        }
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let target_seconds = pos.as_secs_f64();
        self.seek_to_seconds(target_seconds);
        Ok(())
    }
}

/// Builder for creating TimelineSource with common configurations
pub struct TimelineSourceBuilder {
    spec: AudioSpec,
    loop_playback: bool,
    start_position: SampleTime,
}

impl TimelineSourceBuilder {
    pub fn new() -> Self {
        Self {
            spec: AudioSpec::cd_quality(),
            loop_playback: false,
            start_position: SampleTime::new(0),
        }
    }

    pub fn spec(mut self, spec: AudioSpec) -> Self {
        self.spec = spec;
        self
    }

    pub fn sample_rate(mut self, sample_rate: u32) -> Self {
        self.spec.sample_rate = sample_rate;
        self
    }

    pub fn channels(mut self, channels: u16) -> Self {
        self.spec.channels = channels;
        self
    }

    pub fn looping(mut self, loop_playback: bool) -> Self {
        self.loop_playback = loop_playback;
        self
    }

    pub fn start_position(mut self, position: SampleTime) -> Self {
        self.start_position = position;
        self
    }

    pub fn start_position_seconds(mut self, seconds: f64) -> Self {
        self.start_position = SampleTime::from_seconds(seconds, self.spec.sample_rate);
        self
    }

    pub fn build(self, graph: Arc<PlaybackGraph>) -> TimelineSource {
        let mut source = if self.loop_playback {
            TimelineSource::new_looping(graph, self.spec)
        } else {
            TimelineSource::new(graph, self.spec)
        };

        if self.start_position.samples() > 0 {
            println!(
                "DEBUG: TimelineSource seeking to start position: {:.3}s",
                self.start_position.to_seconds(self.spec.sample_rate)
            );
            source.seek(self.start_position);
        }

        source
    }
}

impl Default for TimelineSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
