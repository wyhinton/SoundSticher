// Audio artifact implementation

use crate::artifacts::{CompressionType, StorableArtifact, StorageHints, StoragePriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Audio file artifact with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioArtifact {
    /// Path to the audio file
    pub path: PathBuf,

    /// Audio format (wav, mp3, flac, etc.)
    pub format: String,

    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Number of audio channels
    pub channels: u32,

    /// Duration in seconds
    pub duration: f64,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl AudioArtifact {
    pub fn new(
        path: PathBuf,
        format: String,
        sample_rate: u32,
        channels: u32,
        duration: f64,
    ) -> Self {
        Self {
            path,
            format,
            sample_rate,
            channels,
            duration,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the audio artifact
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the file size in bytes
    pub fn size_bytes(&self) -> Result<u64, std::io::Error> {
        std::fs::metadata(&self.path).map(|meta| meta.len())
    }

    /// Check if the audio file exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Get the filename without extension
    pub fn stem(&self) -> String {
        self.path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    /// Get the file extension
    pub fn extension(&self) -> String {
        self.path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    /// Calculate the bitrate in kbps (estimate)
    pub fn estimated_bitrate_kbps(&self) -> Result<u32, std::io::Error> {
        let size_bytes = self.size_bytes()?;
        if self.duration > 0.0 {
            let bitrate = (size_bytes as f64 * 8.0) / (self.duration * 1000.0);
            Ok(bitrate as u32)
        } else {
            Ok(0)
        }
    }

    /// Get audio format information
    pub fn format_info(&self) -> AudioFormatInfo {
        AudioFormatInfo {
            format: self.format.clone(),
            sample_rate: self.sample_rate,
            channels: self.channels,
            duration: self.duration,
            bit_depth: self
                .metadata
                .get("bit_depth")
                .and_then(|s| s.parse().ok())
                .unwrap_or(16),
        }
    }

    /// Check if this is a lossless format
    pub fn is_lossless(&self) -> bool {
        matches!(
            self.format.to_lowercase().as_str(),
            "wav" | "flac" | "aiff" | "alac"
        )
    }

    /// Check if formats are compatible for combining
    pub fn is_compatible_with(&self, other: &AudioArtifact) -> bool {
        self.sample_rate == other.sample_rate
            && self.channels == other.channels
            && self.format == other.format
    }

    /// Get a suitable output format for processing
    pub fn preferred_processing_format(&self) -> String {
        if self.is_lossless() {
            "wav".to_string()
        } else {
            self.format.clone()
        }
    }

    /// Create a new artifact with modified path
    pub fn with_new_path(&self, new_path: PathBuf) -> Self {
        let mut new_artifact = self.clone();
        new_artifact.path = new_path;
        new_artifact
    }

    /// Create a new artifact with modified metadata
    pub fn with_additional_metadata(&self, metadata: HashMap<String, String>) -> Self {
        let mut new_artifact = self.clone();
        new_artifact.metadata.extend(metadata);
        new_artifact
    }

    /// Get timing information
    pub fn timing_info(&self) -> TimingInfo {
        TimingInfo {
            duration: self.duration,
            sample_rate: self.sample_rate,
            total_samples: (self.duration * self.sample_rate as f64) as u64,
        }
    }

    /// Validate the audio file
    pub fn validate(&self) -> Result<(), AudioValidationError> {
        if !self.path.exists() {
            return Err(AudioValidationError::FileNotFound(self.path.clone()));
        }

        if self.duration <= 0.0 {
            return Err(AudioValidationError::InvalidDuration(self.duration));
        }

        if self.sample_rate == 0 {
            return Err(AudioValidationError::InvalidSampleRate(self.sample_rate));
        }

        if self.channels == 0 {
            return Err(AudioValidationError::InvalidChannels(self.channels));
        }

        Ok(())
    }
}

impl StorableArtifact for AudioArtifact {
    fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(self)?)
    }

    fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(data)?)
    }

    fn get_id(&self) -> String {
        format!(
            "audio_{}",
            self.path.to_string_lossy().replace(['/', '\\'], "_")
        )
    }

    fn storage_hints(&self) -> StorageHints {
        StorageHints {
            compression: CompressionType::Lz4,
            priority: StoragePriority::High,
            temporary: false,
        }
    }
}

/// Audio format information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormatInfo {
    pub format: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub duration: f64,
    pub bit_depth: u32,
}

impl AudioFormatInfo {
    /// Check if this format supports the given bit depth
    pub fn supports_bit_depth(&self, bit_depth: u32) -> bool {
        match self.format.to_lowercase().as_str() {
            "wav" | "aiff" => matches!(bit_depth, 16 | 24 | 32),
            "flac" => matches!(bit_depth, 16 | 24),
            "mp3" => bit_depth == 16, // MP3 is always 16-bit internally
            _ => true,                // Unknown formats, assume supported
        }
    }

    /// Get the theoretical maximum file size in bytes
    pub fn max_file_size_bytes(&self) -> u64 {
        let bytes_per_sample = (self.bit_depth / 8) as u64;
        let samples_per_second = self.sample_rate as u64 * self.channels as u64;
        let total_samples = (self.duration * samples_per_second as f64) as u64;
        total_samples * bytes_per_sample
    }
}

/// Timing information for audio
#[derive(Debug, Clone)]
pub struct TimingInfo {
    pub duration: f64,
    pub sample_rate: u32,
    pub total_samples: u64,
}

impl TimingInfo {
    /// Convert time in seconds to sample number
    pub fn time_to_sample(&self, time_seconds: f64) -> u64 {
        (time_seconds * self.sample_rate as f64) as u64
    }

    /// Convert sample number to time in seconds
    pub fn sample_to_time(&self, sample: u64) -> f64 {
        sample as f64 / self.sample_rate as f64
    }

    /// Get time range for a sample range
    pub fn sample_range_to_time_range(&self, start_sample: u64, end_sample: u64) -> (f64, f64) {
        (
            self.sample_to_time(start_sample),
            self.sample_to_time(end_sample),
        )
    }
}

/// Audio validation errors
#[derive(Debug, thiserror::Error)]
pub enum AudioValidationError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid duration: {0}")]
    InvalidDuration(f64),

    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(u32),

    #[error("Invalid channel count: {0}")]
    InvalidChannels(u32),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Collection of audio artifacts with operations
#[derive(Debug, Clone)]
pub struct AudioCollection {
    pub artifacts: Vec<AudioArtifact>,
    pub metadata: HashMap<String, String>,
}

impl AudioCollection {
    pub fn new() -> Self {
        Self {
            artifacts: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add(&mut self, artifact: AudioArtifact) {
        self.artifacts.push(artifact);
    }

    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Get total duration of all audio files
    pub fn total_duration(&self) -> f64 {
        self.artifacts.iter().map(|a| a.duration).sum()
    }

    /// Get total file size
    pub fn total_size_bytes(&self) -> Result<u64, std::io::Error> {
        let mut total = 0;
        for artifact in &self.artifacts {
            total += artifact.size_bytes()?;
        }
        Ok(total)
    }

    /// Check if all audio files have compatible formats
    pub fn all_compatible(&self) -> bool {
        if self.artifacts.len() <= 1 {
            return true;
        }

        let first = &self.artifacts[0];
        self.artifacts
            .iter()
            .skip(1)
            .all(|a| first.is_compatible_with(a))
    }

    /// Get unique sample rates in the collection
    pub fn unique_sample_rates(&self) -> Vec<u32> {
        let mut rates: Vec<u32> = self.artifacts.iter().map(|a| a.sample_rate).collect();
        rates.sort_unstable();
        rates.dedup();
        rates
    }

    /// Get unique channel counts in the collection
    pub fn unique_channel_counts(&self) -> Vec<u32> {
        let mut counts: Vec<u32> = self.artifacts.iter().map(|a| a.channels).collect();
        counts.sort_unstable();
        counts.dedup();
        counts
    }

    /// Filter by format
    pub fn filter_by_format(&self, format: &str) -> AudioCollection {
        AudioCollection {
            artifacts: self
                .artifacts
                .iter()
                .filter(|a| a.format.eq_ignore_ascii_case(format))
                .cloned()
                .collect(),
            metadata: self.metadata.clone(),
        }
    }

    /// Validate all audio files in the collection
    pub fn validate_all(&self) -> Vec<(usize, AudioValidationError)> {
        let mut errors = Vec::new();
        for (index, artifact) in self.artifacts.iter().enumerate() {
            if let Err(error) = artifact.validate() {
                errors.push((index, error));
            }
        }
        errors
    }
}
