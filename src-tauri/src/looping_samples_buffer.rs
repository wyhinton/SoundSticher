use rodio::Source;
use std::sync::Arc;
use std::time::Duration;

use crate::epr;

/// Custom looping source for raw audio samples that continuously loops through the provided samples
pub struct LoopingSamplesBuffer {
    samples: Arc<Vec<i16>>,
    channels: u16,
    sample_rate: u32,
    current_index: usize,
    loop_infinite: bool,
    finished: bool,
}

impl LoopingSamplesBuffer {
    /// Create a new looping samples buffer
    ///
    /// # Arguments
    /// * `channels` - Number of audio channels (e.g., 1 for mono, 2 for stereo)
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100)
    /// * `samples` - Vector of audio samples as i16 values
    pub fn new(channels: u16, sample_rate: u32, samples: Vec<i16>, loop_infinite: bool) -> Self {
        println!(
            "🔊 Creating new LoopingSamplesBuffer with {} samples",
            samples.len()
        );
        println!(
            "📊 Sample rate: {}, Channels: {}, Loop: {}",
            sample_rate, channels, loop_infinite
        );
        Self {
            samples: Arc::new(samples),
            channels,
            sample_rate,
            current_index: 0,
            loop_infinite,
            finished: false,
        }
    }

    /// Enable or disable infinite looping
    pub fn set_loop_infinite(&mut self, loop_infinite: bool) {
        self.loop_infinite = loop_infinite;
        if loop_infinite {
            self.finished = false; // Reset finished state when enabling looping
        }
    }

    /// Check if the buffer is set to loop infinitely
    pub fn is_loop_infinite(&self) -> bool {
        self.loop_infinite
    }

    /// Reset the buffer to the beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.finished = false;
    }

    /// Check if the buffer has finished playing (only relevant when loop_infinite is false)
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Create a new buffer that shares the same sample data but with independent playback state
    pub fn clone_with_shared_samples(&self) -> Self {
        Self {
            samples: Arc::clone(&self.samples), // Share the same sample data
            channels: self.channels,
            sample_rate: self.sample_rate,
            current_index: 0, // Reset playback position
            loop_infinite: self.loop_infinite,
            finished: false,
        }
    }

    /// Seek to a specific sample position
    /// Returns true if the seek was successful, false if position is out of bounds
    pub fn seek_to_sample(&mut self, sample_index: usize) -> bool {
        if sample_index >= self.samples.len() {
            false
        } else {
            self.current_index = sample_index;
            self.finished = false; // Reset finished state when seeking
            true
        }
    }

    /// Seek to a specific time position in seconds
    /// Returns true if the seek was successful, false if position is out of bounds
    pub fn seek_to_time(&mut self, seconds: f32) -> bool {
        if seconds < 0.0 {
            return false;
        }

        let samples_per_second = self.sample_rate as f32 * self.channels as f32;
        let target_sample = (seconds * samples_per_second).round() as usize;

        self.seek_to_sample(target_sample)
    }

    /// Seek to a normalized position (0.0 to 1.0)
    /// Returns true if the seek was successful, false if position is out of bounds
    pub fn seek_to_progress(&mut self, progress: f32) -> bool {
        if progress < 0.0 || progress > 1.0 {
            return false;
        }

        let target_sample = (progress * self.samples.len() as f32).round() as usize;
        self.seek_to_sample(target_sample.min(self.samples.len() - 1))
    }

    /// Get current playback position in samples
    pub fn get_current_sample_position(&self) -> usize {
        self.current_index
    }

    /// Get current playback position in seconds
    pub fn get_current_time_position(&self) -> f32 {
        let samples_per_second = self.sample_rate as f32 * self.channels as f32;
        self.current_index as f32 / samples_per_second
    }

    /// Get current playback progress as a normalized value (0.0 to 1.0)
    pub fn get_current_progress(&self) -> f32 {
        if self.samples.is_empty() {
            0.0
        } else {
            self.current_index as f32 / self.samples.len() as f32
        }
    }

    /// Get the total duration in seconds
    pub fn get_total_duration(&self) -> f32 {
        let total_samples = self.samples.len() as f32;
        let samples_per_second = self.sample_rate as f32 * self.channels as f32;
        total_samples / samples_per_second
    }
}

impl Iterator for LoopingSamplesBuffer {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples.is_empty() || self.finished {
            epr!("SAMPLES IS EMPTY");
            return None;
        }

        let sample = self.samples[self.current_index];
        self.current_index += 1;
        // println!("SAMPLES {}", self.samples.len());
        // println!("CUR INDEX: {}", self.current_index);

        // Check if we've reached the end of the samples
        if self.current_index >= self.samples.len() {
            if self.loop_infinite {
                // Reset to beginning for infinite looping
                self.current_index = 0;
            } else {
                // Mark as finished for single playthrough
                self.finished = true;
            }
        }

        Some(sample)
    }
}

impl Source for LoopingSamplesBuffer {
    fn current_frame_len(&self) -> Option<usize> {
        if self.loop_infinite {
            None // Infinite length due to looping
        } else {
            Some(self.samples.len())
        }
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        if self.loop_infinite {
            None // Infinite duration due to looping
        } else {
            // Calculate duration for single playthrough
            let total_samples = self.samples.len() as f32;
            let samples_per_second = self.sample_rate as f32 * self.channels as f32;
            Some(Duration::from_secs_f32(total_samples / samples_per_second))
        }
    }
}
