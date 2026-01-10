// Core types for operation-based playback

use std::fmt;

/// Sample time in samples (not seconds)
/// This provides sample-accurate positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct SampleTime(pub u64);

impl SampleTime {
    pub fn new(samples: u64) -> Self {
        Self(samples)
    }

    pub fn from_seconds(seconds: f64, sample_rate: u32) -> Self {
        Self((seconds * sample_rate as f64) as u64)
    }

    pub fn to_seconds(&self, sample_rate: u32) -> f64 {
        self.0 as f64 / sample_rate as f64
    }

    pub fn samples(&self) -> u64 {
        self.0
    }

    pub fn add_samples(&self, samples: u64) -> Self {
        Self(self.0 + samples)
    }

    pub fn sub_samples(&self, samples: u64) -> Self {
        Self(self.0.saturating_sub(samples))
    }
}

impl std::ops::Add for SampleTime {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::Sub for SampleTime {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl std::ops::AddAssign for SampleTime {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl fmt::Display for SampleTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}smp", self.0)
    }
}

/// Unique identifier for an operation in the playback system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaybackOpId(pub u64);

impl PlaybackOpId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl From<u64> for PlaybackOpId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Audio format specification for playback
#[derive(Debug, Clone, Copy)]
pub struct AudioSpec {
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioSpec {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// Standard CD-quality stereo
    pub fn cd_quality() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
        }
    }

    /// Samples per channel for a given duration in seconds
    pub fn samples_for_duration(&self, seconds: f64) -> usize {
        (seconds * self.sample_rate as f64) as usize
    }

    /// Duration in seconds for a given number of samples per channel
    pub fn duration_for_samples(&self, samples: usize) -> f64 {
        samples as f64 / self.sample_rate as f64
    }
}

impl Default for AudioSpec {
    fn default() -> Self {
        Self::cd_quality()
    }
}

/// Errors that can occur during operation playback
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("Operation not found: {0:?}")]
    OperationNotFound(PlaybackOpId),

    #[error("Invalid seek position: {0}")]
    InvalidSeekPosition(SampleTime),

    #[error("Buffer underrun at {0}")]
    BufferUnderrun(SampleTime),

    #[error("Audio source error: {0}")]
    AudioSourceError(String),

    #[error("Timeline is empty")]
    EmptyTimeline,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for playback operations
pub type PlaybackResult<T> = Result<T, PlaybackError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_time_from_seconds() {
        let t = SampleTime::from_seconds(1.0, 44100);
        assert_eq!(t.samples(), 44100);
    }

    #[test]
    fn test_sample_time_to_seconds() {
        let t = SampleTime::new(44100);
        assert!((t.to_seconds(44100) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_sample_time_add() {
        let t1 = SampleTime::new(100);
        let t2 = SampleTime::new(50);
        assert_eq!((t1 + t2).samples(), 150);
    }

    #[test]
    fn test_sample_time_sub_saturating() {
        let t1 = SampleTime::new(50);
        let t2 = SampleTime::new(100);
        assert_eq!((t1 - t2).samples(), 0); // Saturates at 0
    }

    #[test]
    fn test_audio_spec_samples_for_duration() {
        let spec = AudioSpec::cd_quality();
        assert_eq!(spec.samples_for_duration(1.0), 44100);
        assert_eq!(spec.samples_for_duration(0.5), 22050);
    }
}
