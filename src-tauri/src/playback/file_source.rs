// File-based audio source

use crate::playback::{AudioSource, AudioSourceError};
use std::path::Path;

/// Audio source that reads from a file
pub struct FileSource {
    sample_rate: u32,
    channels: u16,
    total_samples: Option<u64>,
    current_position: u64,
    // TODO: Add actual audio decoder (symphonia, etc.)
}

impl FileSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, AudioSourceError> {
        // TODO: Implement file loading with an audio library
        // This is a placeholder implementation

        let _path = path.as_ref();

        Ok(Self {
            sample_rate: 44100,
            channels: 2,
            total_samples: Some(44100 * 10), // 10 seconds of audio
            current_position: 0,
        })
    }
}

impl AudioSource for FileSource {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn total_samples(&self) -> Option<u64> {
        self.total_samples
    }

    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize, AudioSourceError> {
        // TODO: Implement actual audio decoding
        // For now, generate silence

        let remaining = self
            .total_samples
            .unwrap_or(0)
            .saturating_sub(self.current_position);
        let samples_to_read = (buffer.len() as u64).min(remaining) as usize;

        // Fill with silence
        for sample in &mut buffer[..samples_to_read] {
            *sample = 0.0;
        }

        self.current_position += samples_to_read as u64;
        Ok(samples_to_read)
    }

    fn seek(&mut self, position: u64) -> Result<(), AudioSourceError> {
        let total = self.total_samples.unwrap_or(0);
        if position > total {
            return Err(AudioSourceError::InvalidPosition(position));
        }

        self.current_position = position;
        Ok(())
    }

    fn position(&self) -> u64 {
        self.current_position
    }

    fn is_finished(&self) -> bool {
        if let Some(total) = self.total_samples {
            self.current_position >= total
        } else {
            false
        }
    }
}
