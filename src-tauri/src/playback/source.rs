// AudioSource trait for playback

use std::io::{Read, Seek};

/// Trait for audio sources that can be played back
pub trait AudioSource: Send + Sync {
    /// Get the sample rate of the audio source
    fn sample_rate(&self) -> u32;

    /// Get the number of channels
    fn channels(&self) -> u16;

    /// Get the total number of samples
    fn total_samples(&self) -> Option<u64>;

    /// Read samples into the provided buffer
    /// Returns the number of samples read
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize, AudioSourceError>;

    /// Seek to a specific sample position
    fn seek(&mut self, position: u64) -> Result<(), AudioSourceError>;

    /// Get the current position in samples
    fn position(&self) -> u64;

    /// Check if the source has reached the end
    fn is_finished(&self) -> bool;

    /// Reset the source to the beginning
    fn reset(&mut self) -> Result<(), AudioSourceError> {
        self.seek(0)
    }

    /// Get the duration in seconds
    fn duration(&self) -> Option<f64> {
        self.total_samples()
            .map(|samples| samples as f64 / self.sample_rate() as f64)
    }
}

/// Errors that can occur with audio sources
#[derive(Debug, thiserror::Error)]
pub enum AudioSourceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Decode error: {0}")]
    DecodeError(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Seek failed: {0}")]
    SeekFailed(String),

    #[error("Invalid position: {0}")]
    InvalidPosition(u64),
}
