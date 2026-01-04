// Fingerprinting for change detection and caching

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

/// Represents a fingerprint of content for change detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub hash: String,
    pub timestamp: std::time::SystemTime,
    pub size: u64,
    pub metadata: HashMap<String, String>,
}

impl Fingerprint {
    /// Create a fingerprint from file content
    pub fn from_file(path: &Path) -> Result<Self, FingerprintError> {
        let metadata = std::fs::metadata(path)?;
        let content = std::fs::read(path)?;
        let hash = Self::hash_content(&content);

        Ok(Self {
            hash,
            timestamp: metadata.modified()?,
            size: metadata.len(),
            metadata: HashMap::new(),
        })
    }

    /// Create a fingerprint from raw content
    pub fn from_content(content: &[u8]) -> Self {
        Self {
            hash: Self::hash_content(content),
            timestamp: std::time::SystemTime::now(),
            size: content.len() as u64,
            metadata: HashMap::new(),
        }
    }

    /// Create a fingerprint from parameters
    pub fn from_parameters(params: &serde_json::Value) -> Self {
        let content = serde_json::to_vec(params).unwrap_or_default();
        Self::from_content(&content)
    }

    fn hash_content(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
