// Artifact enum and base types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::artifacts::{AudioArtifact, RegionsArtifact};

/// Universal artifact type that represents any data produced by operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Artifact {
    /// Single audio file
    Audio(AudioArtifact),

    /// Collection of audio files
    AudioList(Vec<AudioArtifact>),

    /// Audio regions/segments with timing information
    Regions(RegionsArtifact),

    /// Raw binary data
    Binary(BinaryArtifact),

    /// Text/metadata
    Text(TextArtifact),

    /// Numerical data (measurements, analysis results)
    Numeric(NumericArtifact),
}

impl Artifact {
    /// Get the artifact type as a string
    pub fn artifact_type(&self) -> &'static str {
        match self {
            Artifact::Audio(_) => "audio",
            Artifact::AudioList(_) => "audio_list",
            Artifact::Regions(_) => "regions",
            Artifact::Binary(_) => "binary",
            Artifact::Text(_) => "text",
            Artifact::Numeric(_) => "numeric",
        }
    }

    /// Get the size of the artifact in bytes
    pub fn size_bytes(&self) -> Result<u64, std::io::Error> {
        match self {
            Artifact::Audio(audio) => audio.size_bytes(),
            Artifact::AudioList(list) => {
                let mut total = 0;
                for audio in list {
                    total += audio.size_bytes()?;
                }
                Ok(total)
            }
            Artifact::Regions(regions) => Ok(regions.data.len() as u64),
            Artifact::Binary(binary) => Ok(binary.data.len() as u64),
            Artifact::Text(text) => Ok(text.content.len() as u64),
            Artifact::Numeric(numeric) => Ok(numeric.values.len() as u64 * 8), // Assuming f64
        }
    }

    /// Check if the artifact exists on disk (for file-based artifacts)
    pub fn exists(&self) -> bool {
        match self {
            Artifact::Audio(audio) => audio.path.exists(),
            Artifact::AudioList(list) => list.iter().all(|audio| audio.path.exists()),
            Artifact::Regions(_)
            | Artifact::Binary(_)
            | Artifact::Text(_)
            | Artifact::Numeric(_) => true,
        }
    }

    /// Get all file paths referenced by this artifact
    pub fn get_file_paths(&self) -> Vec<PathBuf> {
        match self {
            Artifact::Audio(audio) => vec![audio.path.clone()],
            Artifact::AudioList(list) => list.iter().map(|audio| audio.path.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// Convert to AudioArtifact if possible
    pub fn as_audio(&self) -> Option<&AudioArtifact> {
        match self {
            Artifact::Audio(audio) => Some(audio),
            _ => None,
        }
    }

    /// Convert to AudioList if possible
    pub fn as_audio_list(&self) -> Option<&Vec<AudioArtifact>> {
        match self {
            Artifact::AudioList(list) => Some(list),
            _ => None,
        }
    }

    /// Convert to RegionsArtifact if possible
    pub fn as_regions(&self) -> Option<&RegionsArtifact> {
        match self {
            Artifact::Regions(regions) => Some(regions),
            _ => None,
        }
    }
}

/// Binary data artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryArtifact {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub metadata: HashMap<String, String>,
}

impl BinaryArtifact {
    pub fn new(data: Vec<u8>, mime_type: String) -> Self {
        Self {
            data,
            mime_type,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Text data artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextArtifact {
    pub content: String,
    pub format: TextFormat,
    pub encoding: String,
    pub metadata: HashMap<String, String>,
}

impl TextArtifact {
    pub fn new(content: String, format: TextFormat) -> Self {
        Self {
            content,
            format,
            encoding: "utf-8".to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextFormat {
    PlainText,
    Markdown,
    Json,
    Yaml,
    Xml,
    Html,
    Csv,
}

/// Numeric data artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericArtifact {
    pub values: Vec<f64>,
    pub labels: Vec<String>,
    pub units: String,
    pub metadata: HashMap<String, String>,
}

impl NumericArtifact {
    pub fn new(values: Vec<f64>) -> Self {
        Self {
            values,
            labels: Vec::new(),
            units: String::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_units(mut self, units: String) -> Self {
        self.units = units;
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Trait for artifacts that can be stored and retrieved
pub trait StorableArtifact {
    /// Serialize to bytes for storage
    fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>>;

    /// Deserialize from bytes
    fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;

    /// Get a unique identifier for this artifact
    fn get_id(&self) -> String;

    /// Get storage hints (compression, priority, etc.)
    fn storage_hints(&self) -> StorageHints {
        StorageHints::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageHints {
    pub compression: CompressionType,
    pub priority: StoragePriority,
    pub temporary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum CompressionType {
    #[default]
    None,
    Lz4,
    Zstd,
    Gzip,
}


#[derive(Debug, Clone)]
#[derive(Default)]
pub enum StoragePriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

