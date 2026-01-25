// Audio artifact implementation

use crate::artifacts::{CompressionType, StorableArtifact, StorageHints, StoragePriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ============================================================================
// HYBRID ARTIFACT TYPES - In-Memory vs On-Disk
// ============================================================================

/// Audio sample buffer with format information.
/// This is the in-memory representation of decoded audio.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Interleaved audio samples (f32 normalized to [-1.0, 1.0])
    pub samples: Arc<Vec<f32>>,

    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Number of audio channels
    pub channels: u32,
}

impl AudioBuffer {
    /// Create a new audio buffer
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u32) -> Self {
        Self {
            samples: Arc::new(samples),
            sample_rate,
            channels,
        }
    }

    /// Create from an existing Arc (for sharing ownership)
    pub fn from_arc(samples: Arc<Vec<f32>>, sample_rate: u32, channels: u32) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
        }
    }

    /// Get duration in seconds
    pub fn duration_seconds(&self) -> f64 {
        if self.channels == 0 || self.sample_rate == 0 {
            return 0.0;
        }
        let total_frames = self.samples.len() / self.channels as usize;
        total_frames as f64 / self.sample_rate as f64
    }

    /// Get the total number of frames (samples per channel)
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            return 0;
        }
        self.samples.len() / self.channels as usize
    }

    /// Get a slice of samples for a given time range
    pub fn get_samples_at_time(&self, start_time: f64, duration: f64) -> Option<&[f32]> {
        let start_frame = (start_time * self.sample_rate as f64) as usize;
        let frame_count = (duration * self.sample_rate as f64) as usize;
        let start_idx = start_frame * self.channels as usize;
        let end_idx = (start_frame + frame_count) * self.channels as usize;

        if end_idx <= self.samples.len() {
            Some(&self.samples[start_idx..end_idx])
        } else {
            None
        }
    }

    /// Estimated memory size in bytes
    pub fn memory_size(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>()
    }
}

/// Serializable version of AudioBuffer metadata (samples stored separately)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBufferMeta {
    pub sample_rate: u32,
    pub channels: u32,
    pub frame_count: usize,
    pub duration_seconds: f64,
}

impl From<&AudioBuffer> for AudioBufferMeta {
    fn from(buffer: &AudioBuffer) -> Self {
        Self {
            sample_rate: buffer.sample_rate,
            channels: buffer.channels,
            frame_count: buffer.frame_count(),
            duration_seconds: buffer.duration_seconds(),
        }
    }
}

/// Represents where audio data is stored - either in memory or on disk.
///
/// This is the core of the hybrid artifact model:
/// - `InMemory`: Samples are decoded and ready for processing (fast, no I/O)
/// - `OnDisk`: Samples are in a file (requires loading, but persistent)
///
/// Operations can consume artifacts agnostically via the `SampleReader` trait.
#[derive(Debug, Clone)]
pub enum AudioData {
    /// Audio samples held in memory (decoded, ready for processing)
    InMemory(AudioBuffer),

    /// Audio samples stored in a file on disk
    OnDisk { path: PathBuf, format: String },

    /// A reference to another artifact's output (for lazy evaluation)
    /// This allows building graphs without materializing intermediate results.
    Reference {
        /// ID of the source operation/artifact
        source_id: String,
        /// Expected duration (for planning purposes)
        expected_duration: f64,
    },
}

impl AudioData {
    /// Check if this data is currently in memory
    pub fn is_in_memory(&self) -> bool {
        matches!(self, AudioData::InMemory(_))
    }

    /// Check if this data is on disk
    pub fn is_on_disk(&self) -> bool {
        matches!(self, AudioData::OnDisk { .. })
    }

    /// Check if this is a lazy reference
    pub fn is_reference(&self) -> bool {
        matches!(self, AudioData::Reference { .. })
    }

    /// Get the path if on disk
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            AudioData::OnDisk { path, .. } => Some(path),
            _ => None,
        }
    }

    /// Get the buffer if in memory
    pub fn buffer(&self) -> Option<&AudioBuffer> {
        match self {
            AudioData::InMemory(buffer) => Some(buffer),
            _ => None,
        }
    }

    /// Get the format (from disk or inferred from buffer)
    pub fn format(&self) -> String {
        match self {
            AudioData::InMemory(_) => "pcm_f32".to_string(), // In-memory is always decoded PCM
            AudioData::OnDisk { format, .. } => format.clone(),
            AudioData::Reference { .. } => "reference".to_string(),
        }
    }
}

/// Serializable representation of AudioData for storage/transport
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AudioDataSerialized {
    InMemory {
        meta: AudioBufferMeta,
        // Note: actual samples would be stored separately or omitted
    },
    OnDisk {
        path: String,
        format: String,
    },
    Reference {
        source_id: String,
        expected_duration: f64,
    },
}

impl From<&AudioData> for AudioDataSerialized {
    fn from(data: &AudioData) -> Self {
        match data {
            AudioData::InMemory(buffer) => AudioDataSerialized::InMemory {
                meta: AudioBufferMeta::from(buffer),
            },
            AudioData::OnDisk { path, format } => AudioDataSerialized::OnDisk {
                path: path.to_string_lossy().to_string(),
                format: format.clone(),
            },
            AudioData::Reference {
                source_id,
                expected_duration,
            } => AudioDataSerialized::Reference {
                source_id: source_id.clone(),
                expected_duration: *expected_duration,
            },
        }
    }
}

// ============================================================================
// SAMPLE READER TRAIT - Abstraction for reading samples regardless of storage
// ============================================================================

/// Error type for sample reading operations
#[derive(Debug, thiserror::Error)]
pub enum SampleReadError {
    #[error("Audio data is a reference and needs to be resolved first")]
    UnresolvedReference,

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Time range out of bounds: requested {requested}s, available {available}s")]
    OutOfBounds { requested: f64, available: f64 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors that can occur during materialization (disk I/O)
#[derive(Debug, thiserror::Error)]
pub enum MaterializeError {
    #[error("No audio data available")]
    NoData,

    #[error("Audio data is a reference and needs to be resolved first")]
    UnresolvedReference,

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Failed to encode audio: {0}")]
    EncodeError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ============================================================================
// HELPER FUNCTIONS FOR WAV FILE I/O
// ============================================================================

/// Write an AudioBuffer to a WAV file
pub fn write_wav_file(path: PathBuf, buffer: &AudioBuffer) -> Result<(), MaterializeError> {
    use std::io::{BufWriter, Write};

    let file = std::fs::File::create(&path)?;
    let mut writer = BufWriter::new(file);

    let sample_rate = buffer.sample_rate;
    let channels = buffer.channels as u16;
    let bits_per_sample: u16 = 32; // f32 samples
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = channels * (bits_per_sample / 8);
    let data_size = (buffer.samples.len() * 4) as u32; // f32 = 4 bytes

    // WAV header
    writer.write_all(b"RIFF")?;
    writer.write_all(&(36 + data_size).to_le_bytes())?;
    writer.write_all(b"WAVE")?;

    // fmt chunk
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // chunk size
    writer.write_all(&3u16.to_le_bytes())?; // format = IEEE float
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;

    // Write samples
    for sample in buffer.samples.iter() {
        writer.write_all(&sample.to_le_bytes())?;
    }

    writer.flush()?;
    Ok(())
}

/// Load an audio file into an AudioBuffer
///
/// This is a simplified loader that handles WAV files.
/// For production use, this should use symphonia or similar library.
pub fn load_audio_to_buffer(path: &PathBuf) -> Result<AudioBuffer, MaterializeError> {
    use std::io::{BufReader, Read};

    if !path.exists() {
        return Err(MaterializeError::FileNotFound(path.clone()));
    }

    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    // Read RIFF header
    let mut header = [0u8; 12];
    reader.read_exact(&mut header)?;

    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err(MaterializeError::DecodeError(
            "Not a valid WAV file".to_string(),
        ));
    }

    // Read chunks until we find fmt and data
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits_per_sample = 0u16;
    let mut audio_format = 0u16;
    let mut samples: Vec<f32> = Vec::new();

    loop {
        let mut chunk_header = [0u8; 8];
        if reader.read_exact(&mut chunk_header).is_err() {
            break;
        }

        let chunk_id = &chunk_header[0..4];
        let chunk_size = u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]);

        if chunk_id == b"fmt " {
            let mut fmt_data = vec![0u8; chunk_size as usize];
            reader.read_exact(&mut fmt_data)?;

            audio_format = u16::from_le_bytes([fmt_data[0], fmt_data[1]]);
            channels = u16::from_le_bytes([fmt_data[2], fmt_data[3]]);
            sample_rate = u32::from_le_bytes([fmt_data[4], fmt_data[5], fmt_data[6], fmt_data[7]]);
            bits_per_sample = u16::from_le_bytes([fmt_data[14], fmt_data[15]]);
        } else if chunk_id == b"data" {
            let bytes_per_sample = (bits_per_sample / 8) as usize;
            let num_samples = chunk_size as usize / bytes_per_sample;
            samples.reserve(num_samples);

            let mut sample_data = vec![0u8; chunk_size as usize];
            reader.read_exact(&mut sample_data)?;

            // Convert based on format
            match (audio_format, bits_per_sample) {
                (1, 16) => {
                    // PCM 16-bit
                    for chunk in sample_data.chunks(2) {
                        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                        samples.push(sample as f32 / 32768.0);
                    }
                }
                (1, 24) => {
                    // PCM 24-bit
                    for chunk in sample_data.chunks(3) {
                        let sample = i32::from_le_bytes([0, chunk[0], chunk[1], chunk[2]]) >> 8;
                        samples.push(sample as f32 / 8388608.0);
                    }
                }
                (1, 32) => {
                    // PCM 32-bit integer
                    for chunk in sample_data.chunks(4) {
                        let sample = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        samples.push(sample as f32 / 2147483648.0);
                    }
                }
                (3, 32) => {
                    // IEEE float 32-bit
                    for chunk in sample_data.chunks(4) {
                        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        samples.push(sample);
                    }
                }
                _ => {
                    return Err(MaterializeError::DecodeError(format!(
                        "Unsupported WAV format: format={}, bits={}",
                        audio_format, bits_per_sample
                    )));
                }
            }
            break;
        } else {
            // Skip unknown chunk
            let mut skip_data = vec![0u8; chunk_size as usize];
            reader.read_exact(&mut skip_data)?;
        }
    }

    if samples.is_empty() {
        return Err(MaterializeError::DecodeError(
            "No audio data found in WAV file".to_string(),
        ));
    }

    Ok(AudioBuffer::new(samples, sample_rate, channels as u32))
}

/// Trait for reading audio samples from any source
pub trait SampleReader: Send + Sync {
    /// Get the total duration in seconds
    fn duration(&self) -> f64;

    /// Get the sample rate
    fn sample_rate(&self) -> u32;

    /// Get the channel count
    fn channels(&self) -> u32;

    /// Read samples for a given time range (returns interleaved samples)
    /// Returns None if the range is out of bounds
    fn read_samples(&self, start_time: f64, duration: f64) -> Result<Vec<f32>, SampleReadError>;

    /// Read all samples (use with caution for large files)
    fn read_all_samples(&self) -> Result<Vec<f32>, SampleReadError> {
        self.read_samples(0.0, self.duration())
    }

    /// Check if the data is immediately available (in memory)
    fn is_ready(&self) -> bool;
}

// ============================================================================
// AUDIO ARTIFACT - The main type for audio operation outputs
// ============================================================================

/// Audio file artifact with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioArtifact {
    /// Path to the audio file (for disk-backed artifacts)
    /// Note: This is kept for backward compatibility but may be empty for in-memory artifacts
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

    /// The actual audio data storage (transient, not serialized)
    #[serde(skip)]
    pub data: Option<AudioData>,
}

impl AudioArtifact {
    /// Create a new disk-backed audio artifact (backward compatible)
    pub fn new(
        path: PathBuf,
        format: String,
        sample_rate: u32,
        channels: u32,
        duration: f64,
    ) -> Self {
        Self {
            path: path.clone(),
            format: format.clone(),
            sample_rate,
            channels,
            duration,
            metadata: HashMap::new(),
            data: Some(AudioData::OnDisk { path, format }),
        }
    }

    /// Create an in-memory audio artifact from a buffer
    ///
    /// This is the preferred way to create artifacts for intermediate operations
    /// as it avoids disk I/O entirely.
    pub fn from_buffer(buffer: AudioBuffer) -> Self {
        let duration = buffer.duration_seconds();
        let sample_rate = buffer.sample_rate;
        let channels = buffer.channels;

        Self {
            path: PathBuf::new(), // No path for in-memory artifacts
            format: "pcm_f32".to_string(),
            sample_rate,
            channels,
            duration,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("storage_type".to_string(), "in_memory".to_string());
                meta
            },
            data: Some(AudioData::InMemory(buffer)),
        }
    }

    /// Create an in-memory artifact from raw samples
    pub fn from_samples(samples: Vec<f32>, sample_rate: u32, channels: u32) -> Self {
        let buffer = AudioBuffer::new(samples, sample_rate, channels);
        Self::from_buffer(buffer)
    }

    /// Create a reference artifact for lazy evaluation
    pub fn from_reference(
        source_id: String,
        expected_duration: f64,
        sample_rate: u32,
        channels: u32,
    ) -> Self {
        Self {
            path: PathBuf::new(),
            format: "reference".to_string(),
            sample_rate,
            channels,
            duration: expected_duration,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("storage_type".to_string(), "reference".to_string());
                meta.insert("source_id".to_string(), source_id.clone());
                meta
            },
            data: Some(AudioData::Reference {
                source_id,
                expected_duration,
            }),
        }
    }

    /// Check if this artifact has data currently in memory
    pub fn is_in_memory(&self) -> bool {
        matches!(&self.data, Some(AudioData::InMemory(_)))
    }

    /// Check if this artifact is backed by a file on disk
    pub fn is_on_disk(&self) -> bool {
        matches!(&self.data, Some(AudioData::OnDisk { .. }))
    }

    /// Check if this is a reference to another operation's output
    pub fn is_reference(&self) -> bool {
        matches!(&self.data, Some(AudioData::Reference { .. }))
    }

    /// Get the audio buffer if in memory
    pub fn buffer(&self) -> Option<&AudioBuffer> {
        match &self.data {
            Some(AudioData::InMemory(buffer)) => Some(buffer),
            _ => None,
        }
    }

    /// Get the underlying AudioData
    pub fn audio_data(&self) -> Option<&AudioData> {
        self.data.as_ref()
    }

    /// Set the underlying AudioData
    pub fn set_audio_data(&mut self, data: AudioData) {
        // Update duration and path based on data type
        match &data {
            AudioData::InMemory(buffer) => {
                self.duration = buffer.duration_seconds();
                self.sample_rate = buffer.sample_rate;
                self.channels = buffer.channels;
                self.path = PathBuf::new();
                self.format = "pcm_f32".to_string();
            }
            AudioData::OnDisk { path, format } => {
                self.path = path.clone();
                self.format = format.clone();
            }
            AudioData::Reference {
                expected_duration, ..
            } => {
                self.duration = *expected_duration;
            }
        }
        self.data = Some(data);
    }

    /// Materialize this artifact to disk (if in memory)
    ///
    /// Returns the path to the written file. If already on disk, returns existing path.
    /// This is used for:
    /// - Explicit export/save operations
    /// - Caching intermediate results
    /// - Crossing process boundaries
    pub fn materialize_to_disk(
        &mut self,
        output_path: PathBuf,
    ) -> Result<PathBuf, MaterializeError> {
        match &self.data {
            Some(AudioData::InMemory(buffer)) => {
                // Write buffer to WAV file
                write_wav_file(output_path.clone(), buffer)?;

                // Update artifact to reference the file
                self.path = output_path.clone();
                self.format = "wav".to_string();
                self.data = Some(AudioData::OnDisk {
                    path: output_path.clone(),
                    format: "wav".to_string(),
                });
                self.metadata
                    .insert("storage_type".to_string(), "on_disk".to_string());
                self.metadata
                    .insert("materialized".to_string(), "true".to_string());

                Ok(output_path)
            }
            Some(AudioData::OnDisk { path, .. }) => {
                // Already on disk, just return the path
                Ok(path.clone())
            }
            Some(AudioData::Reference { .. }) => Err(MaterializeError::UnresolvedReference),
            None => {
                // Fallback: if we have a path, assume it's on disk
                if self.path.exists() {
                    Ok(self.path.clone())
                } else {
                    Err(MaterializeError::NoData)
                }
            }
        }
    }

    /// Load this artifact into memory from disk (if on disk)
    ///
    /// This is used when an operation needs to process the actual samples.
    pub fn load_into_memory(&mut self) -> Result<&AudioBuffer, MaterializeError> {
        // Determine what action to take based on current state
        enum LoadAction {
            AlreadyLoaded,
            LoadFromPath(PathBuf),
            IsReference,
            NoData,
        }

        let action = match &self.data {
            Some(AudioData::InMemory(_)) => LoadAction::AlreadyLoaded,
            Some(AudioData::OnDisk { path, .. }) => LoadAction::LoadFromPath(path.clone()),
            Some(AudioData::Reference { .. }) => LoadAction::IsReference,
            None => {
                if self.path.exists() {
                    LoadAction::LoadFromPath(self.path.clone())
                } else {
                    LoadAction::NoData
                }
            }
        };

        // Now handle the action (borrow has ended)
        match action {
            LoadAction::AlreadyLoaded => {
                // Already in memory, just return reference
            }
            LoadAction::LoadFromPath(path) => {
                // Load the buffer
                let buffer = load_audio_to_buffer(&path)?;
                self.duration = buffer.duration_seconds();
                self.sample_rate = buffer.sample_rate;
                self.channels = buffer.channels;
                self.data = Some(AudioData::InMemory(buffer));
                self.metadata
                    .insert("storage_type".to_string(), "in_memory".to_string());
            }
            LoadAction::IsReference => {
                return Err(MaterializeError::UnresolvedReference);
            }
            LoadAction::NoData => {
                return Err(MaterializeError::NoData);
            }
        }

        // Now return a reference to the buffer
        match &self.data {
            Some(AudioData::InMemory(buffer)) => Ok(buffer),
            _ => Err(MaterializeError::NoData),
        }
    }

    /// Add metadata to the audio artifact
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set the audio data and return self (builder pattern)
    pub fn with_data(mut self, data: AudioData) -> Self {
        self.set_audio_data(data);
        self
    }

    /// Get the file size in bytes
    pub fn size_bytes(&self) -> Result<u64, std::io::Error> {
        // For in-memory artifacts, calculate from buffer
        if let Some(AudioData::InMemory(buffer)) = &self.data {
            return Ok(buffer.memory_size() as u64);
        }
        // For on-disk, get file metadata
        std::fs::metadata(&self.path).map(|meta| meta.len())
    }

    /// Check if the audio file exists (for disk-backed artifacts)
    pub fn exists(&self) -> bool {
        match &self.data {
            Some(AudioData::InMemory(_)) => true, // In-memory always "exists"
            Some(AudioData::OnDisk { path, .. }) => path.exists(),
            Some(AudioData::Reference { .. }) => false, // Reference needs resolution
            None => self.path.exists(),                 // Fallback to path check
        }
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
