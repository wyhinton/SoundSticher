// Region-based audio source

use crate::artifacts::AudioRegion;
use crate::playback::{AudioSource, AudioSourceError};

/// Audio source that plays a specific region of another source
pub struct RegionSource<T: AudioSource> {
    inner: T,
    region: AudioRegion,
    current_position: u64,
    start_sample: u64,
    end_sample: u64,
}

impl<T: AudioSource> RegionSource<T> {
    pub fn new(mut inner: T, region: AudioRegion) -> Result<Self, AudioSourceError> {
        let sample_rate = inner.sample_rate() as f64;
        let start_sample = (region.start_time * sample_rate) as u64;
        let end_sample = (region.end_time * sample_rate) as u64;

        // Seek to the start of the region
        inner.seek(start_sample)?;

        Ok(Self {
            inner,
            region,
            current_position: 0,
            start_sample,
            end_sample,
        })
    }

    pub fn region(&self) -> &AudioRegion {
        &self.region
    }
}

impl<T: AudioSource> AudioSource for RegionSource<T> {
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn total_samples(&self) -> Option<u64> {
        Some(self.end_sample - self.start_sample)
    }

    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize, AudioSourceError> {
        let remaining = self.end_sample - (self.start_sample + self.current_position);
        let samples_to_read = (buffer.len() as u64).min(remaining) as usize;

        if samples_to_read == 0 {
            return Ok(0);
        }

        let read = self.inner.read_samples(&mut buffer[..samples_to_read])?;
        self.current_position += read as u64;
        Ok(read)
    }

    fn seek(&mut self, position: u64) -> Result<(), AudioSourceError> {
        let total = self.total_samples().unwrap_or(0);
        if position > total {
            return Err(AudioSourceError::InvalidPosition(position));
        }

        self.inner.seek(self.start_sample + position)?;
        self.current_position = position;
        Ok(())
    }

    fn position(&self) -> u64 {
        self.current_position
    }

    fn is_finished(&self) -> bool {
        self.current_position >= self.total_samples().unwrap_or(0)
    }
}
