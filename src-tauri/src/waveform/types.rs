// Waveform types and keys
//
// Waveforms are view-layer artifacts derived from audio files.
// They are keyed by audio identity + parameters and globally cached.

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Stable audio identity for content-addressable caching
/// This ensures two ops referencing the same file resolve to the same waveform
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioKey {
    /// Source identifier (file path or asset id)
    pub source_id: String,
    /// Content hash (based on file mtime + size for fast validation)
    pub content_hash: u64,
}

impl Hash for AudioKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_id.hash(state);
        self.content_hash.hash(state);
    }
}

impl AudioKey {
    /// Create a new AudioKey from a file path
    pub fn from_path(path: &str) -> Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(path)?;
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let size = metadata.len();

        // Combine mtime and size for a fast content hash
        let content_hash = mtime ^ (size << 32) ^ (size >> 32);

        Ok(Self {
            source_id: path.to_string(),
            content_hash,
        })
    }

    /// Create a key with explicit content hash (for testing or custom sources)
    pub fn with_hash(source_id: String, content_hash: u64) -> Self {
        Self {
            source_id,
            content_hash,
        }
    }
}

/// Channel mode for waveform rendering
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Default)]
pub enum ChannelMode {
    /// Sum all channels to mono
    #[default]
    Mono,
    /// Keep stereo (or first two channels)
    Stereo,
    /// Use left channel only
    Left,
    /// Use right channel only
    Right,
}

/// Waveform generation parameters (part of cache key)
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct WaveformSpec {
    /// Target width in pixels (determines samples per pixel)
    pub width: u32,
    /// Target height in pixels
    pub height: u32,
    /// Channel mode for multi-channel audio
    pub channel_mode: ChannelMode,
    /// Whether to normalize the waveform to use full height
    pub normalize: bool,
}

impl Default for WaveformSpec {
    fn default() -> Self {
        Self {
            width: 1000,
            height: 70,
            channel_mode: ChannelMode::Mono,
            normalize: false,
        }
    }
}

impl WaveformSpec {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    pub fn with_channel_mode(mut self, mode: ChannelMode) -> Self {
        self.channel_mode = mode;
        self
    }
}

/// Composite cache key combining audio identity and waveform spec
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct WaveformCacheKey {
    pub audio_key: AudioKey,
    pub spec: WaveformSpec,
}

impl WaveformCacheKey {
    pub fn new(audio_key: AudioKey, spec: WaveformSpec) -> Self {
        Self { audio_key, spec }
    }

    /// Serialize to a string key for HashMap usage
    pub fn to_string_key(&self) -> String {
        format!(
            "{}:{}:{}x{}:{}:{}",
            self.audio_key.source_id,
            self.audio_key.content_hash,
            self.spec.width,
            self.spec.height,
            match self.spec.channel_mode {
                ChannelMode::Mono => "mono",
                ChannelMode::Stereo => "stereo",
                ChannelMode::Left => "left",
                ChannelMode::Right => "right",
            },
            self.spec.normalize
        )
    }
}

/// Waveform data - a lightweight, shareable artifact
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Waveform {
    /// SVG path string for rendering
    pub svg_path: String,
    /// Min/max peaks per bucket (for alternative rendering)
    pub peaks: Vec<(f32, f32)>,
    /// Sample rate of source audio
    pub sample_rate: u32,
    /// Duration of source audio in seconds
    pub duration: f64,
    /// Number of samples in source audio
    pub sample_count: usize,
    /// Width used for generation
    pub width: u32,
    /// Height used for generation
    pub height: u32,
}

impl Waveform {
    pub fn new(
        svg_path: String,
        peaks: Vec<(f32, f32)>,
        sample_rate: u32,
        duration: f64,
        sample_count: usize,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            svg_path,
            peaks,
            sample_rate,
            duration,
            sample_count,
            width,
            height,
        }
    }

    /// Empty waveform for errors or missing files
    pub fn empty() -> Self {
        Self {
            svg_path: String::new(),
            peaks: Vec::new(),
            sample_rate: 44100,
            duration: 0.0,
            sample_count: 0,
            width: 0,
            height: 0,
        }
    }
}

/// Waveform request from frontend
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformRequest {
    /// File path of the audio file
    pub file_path: String,
    /// Target width in pixels
    #[serde(default = "default_width")]
    pub width: u32,
    /// Target height in pixels
    #[serde(default = "default_height")]
    pub height: u32,
    /// Whether to normalize
    #[serde(default)]
    pub normalize: bool,
}

fn default_width() -> u32 {
    1000
}

fn default_height() -> u32 {
    70
}

/// Waveform response to frontend
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformResponse {
    /// The audio key used for caching
    pub audio_key: AudioKey,
    /// The generated waveform
    pub waveform: Waveform,
    /// Whether this was a cache hit
    pub cache_hit: bool,
}

/// Batch waveform request
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchWaveformRequest {
    /// List of file paths to get waveforms for
    pub file_paths: Vec<String>,
    /// Shared spec for all waveforms
    pub width: u32,
    pub height: u32,
    pub normalize: bool,
}

/// Single item in batch response
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchWaveformItem {
    pub file_path: String,
    pub audio_key: AudioKey,
    pub waveform: Option<Waveform>,
    pub error: Option<String>,
    pub cache_hit: bool,
}

/// Batch waveform response
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchWaveformResponse {
    pub items: Vec<BatchWaveformItem>,
    pub total_cache_hits: u32,
    pub total_computed: u32,
    pub total_errors: u32,
}

/// Cache statistics
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_compute_time_ms: u64,
}
