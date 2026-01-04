// Artifact storage management

use crate::artifacts::{Artifact, CompressionType, StorableArtifact, StorageHints};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Storage manager for artifacts with caching and compression
#[derive(Debug)]
pub struct ArtifactStorage {
    /// Base directory for storage
    storage_dir: PathBuf,

    /// In-memory cache of artifacts
    cache: Arc<RwLock<HashMap<String, Arc<Artifact>>>>,

    /// Storage metadata
    metadata: Arc<RwLock<StorageMetadata>>,

    /// Maximum cache size in bytes
    max_cache_size: usize,

    /// Current cache size in bytes
    current_cache_size: Arc<RwLock<usize>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StorageMetadata {
    artifacts: HashMap<String, ArtifactEntry>,
    total_size: u64,
    last_cleanup: std::time::SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactEntry {
    id: String,
    file_path: PathBuf,
    size_bytes: u64,
    compression: CompressionType,
    created_at: std::time::SystemTime,
    last_accessed: std::time::SystemTime,
    access_count: u64,
    temporary: bool,
}

impl ArtifactStorage {
    /// Create a new artifact storage manager
    pub fn new(storage_dir: PathBuf, max_cache_size: usize) -> Result<Self, StorageError> {
        std::fs::create_dir_all(&storage_dir)?;

        let metadata_path = storage_dir.join("metadata.json");
        let metadata = if metadata_path.exists() {
            let data = std::fs::read_to_string(&metadata_path)?;
            serde_json::from_str(&data)?
        } else {
            StorageMetadata {
                artifacts: HashMap::new(),
                total_size: 0,
                last_cleanup: std::time::SystemTime::now(),
            }
        };

        Ok(Self {
            storage_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(metadata)),
            max_cache_size,
            current_cache_size: Arc::new(RwLock::new(0)),
        })
    }

    /// Store an artifact
    pub fn store(&self, artifact: &Artifact) -> Result<String, StorageError> {
        let artifact_id = self.generate_artifact_id(artifact);
        let storage_hints = self.get_storage_hints(artifact);

        // Serialize artifact
        let data = match artifact {
            Artifact::Audio(audio) => audio.to_bytes()?,
            Artifact::AudioList(list) => serde_json::to_vec(list)?,
            Artifact::Regions(regions) => regions.to_bytes()?,
            Artifact::Binary(binary) => serde_json::to_vec(binary)?,
            Artifact::Text(text) => serde_json::to_vec(text)?,
            Artifact::Numeric(numeric) => serde_json::to_vec(numeric)?,
        };

        // Compress if needed
        let (compressed_data, actual_compression) =
            self.compress_data(data, storage_hints.compression)?;

        // Write to disk
        let file_path = self.get_file_path(&artifact_id);
        std::fs::write(&file_path, &compressed_data)?;

        // Update metadata
        let entry = ArtifactEntry {
            id: artifact_id.clone(),
            file_path: file_path
                .strip_prefix(&self.storage_dir)
                .unwrap()
                .to_path_buf(),
            size_bytes: compressed_data.len() as u64,
            compression: actual_compression,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            access_count: 1,
            temporary: storage_hints.temporary,
        };

        {
            let mut metadata = self.metadata.write().unwrap();
            metadata.artifacts.insert(artifact_id.clone(), entry);
            metadata.total_size += compressed_data.len() as u64;
        }

        // Add to cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(artifact_id.clone(), Arc::new(artifact.clone()));

            let mut cache_size = self.current_cache_size.write().unwrap();
            *cache_size += compressed_data.len();
        }

        // Check if cache cleanup is needed
        self.cleanup_cache_if_needed()?;

        // Save metadata
        self.save_metadata()?;

        Ok(artifact_id)
    }

    /// Retrieve an artifact by ID
    pub fn retrieve(&self, artifact_id: &str) -> Result<Option<Arc<Artifact>>, StorageError> {
        // Check cache first
        {
            let cache = self.cache.read().unwrap();
            if let Some(artifact) = cache.get(artifact_id) {
                self.update_access_stats(artifact_id)?;
                return Ok(Some(artifact.clone()));
            }
        }

        // Load from disk
        let metadata = self.metadata.read().unwrap();
        let entry = match metadata.artifacts.get(artifact_id) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let file_path = self.storage_dir.join(&entry.file_path);
        let compressed_data = std::fs::read(&file_path)?;

        // Decompress
        let data = self.decompress_data(compressed_data, entry.compression.clone())?;

        // Deserialize based on artifact type (need to determine type from ID or metadata)
        let artifact = self.deserialize_artifact(&data, artifact_id)?;

        // Add to cache
        {
            let mut cache = self.cache.write().unwrap();
            let arc_artifact = Arc::new(artifact);
            cache.insert(artifact_id.to_string(), arc_artifact.clone());

            let mut cache_size = self.current_cache_size.write().unwrap();
            *cache_size += data.len();

            self.update_access_stats(artifact_id)?;

            Ok(Some(arc_artifact))
        }
    }

    /// Delete an artifact
    pub fn delete(&self, artifact_id: &str) -> Result<bool, StorageError> {
        let mut metadata = self.metadata.write().unwrap();

        if let Some(entry) = metadata.artifacts.remove(artifact_id) {
            // Remove from disk
            let file_path = self.storage_dir.join(&entry.file_path);
            if file_path.exists() {
                std::fs::remove_file(&file_path)?;
            }

            // Remove from cache
            {
                let mut cache = self.cache.write().unwrap();
                cache.remove(artifact_id);
            }

            metadata.total_size = metadata.total_size.saturating_sub(entry.size_bytes);
            self.save_metadata()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all stored artifacts
    pub fn list_artifacts(&self) -> Vec<String> {
        let metadata = self.metadata.read().unwrap();
        metadata.artifacts.keys().cloned().collect()
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let metadata = self.metadata.read().unwrap();
        let cache = self.cache.read().unwrap();
        let cache_size = *self.current_cache_size.read().unwrap();

        StorageStats {
            total_artifacts: metadata.artifacts.len(),
            total_disk_size: metadata.total_size,
            cached_artifacts: cache.len(),
            cache_size_bytes: cache_size,
            max_cache_size: self.max_cache_size,
            storage_dir: self.storage_dir.clone(),
        }
    }

    /// Cleanup temporary artifacts
    pub fn cleanup_temporary(&self) -> Result<usize, StorageError> {
        let mut count = 0;
        let ids_to_remove: Vec<String> = {
            let metadata = self.metadata.read().unwrap();
            metadata
                .artifacts
                .iter()
                .filter_map(|(id, entry)| {
                    if entry.temporary {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        for id in ids_to_remove {
            if self.delete(&id)? {
                count += 1;
            }
        }

        Ok(count)
    }

    // Private helper methods

    fn generate_artifact_id(&self, artifact: &Artifact) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash based on artifact content for deduplication
        match artifact {
            Artifact::Audio(audio) => {
                audio.path.hash(&mut hasher);
                audio.sample_rate.hash(&mut hasher);
                audio.duration.to_bits().hash(&mut hasher);
            }
            _ => {
                // For other types, use current timestamp + random
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    .hash(&mut hasher);
            }
        }

        format!("artifact_{:x}", hasher.finish())
    }

    fn get_storage_hints(&self, artifact: &Artifact) -> StorageHints {
        match artifact {
            Artifact::Audio(audio) => audio.storage_hints(),
            Artifact::Regions(regions) => regions.storage_hints(),
            _ => Default::default(),
        }
    }

    fn get_file_path(&self, artifact_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.dat", artifact_id))
    }

    fn compress_data(
        &self,
        data: Vec<u8>,
        compression: CompressionType,
    ) -> Result<(Vec<u8>, CompressionType), StorageError> {
        match compression {
            CompressionType::None => Ok((data, CompressionType::None)),
            CompressionType::Lz4 => {
                // TODO: Implement LZ4 compression
                // For now, return uncompressed
                Ok((data, CompressionType::None))
            }
            CompressionType::Zstd => {
                // TODO: Implement Zstd compression
                Ok((data, CompressionType::None))
            }
            CompressionType::Gzip => {
                // TODO: Implement Gzip compression
                Ok((data, CompressionType::None))
            }
        }
    }

    fn decompress_data(
        &self,
        data: Vec<u8>,
        compression: CompressionType,
    ) -> Result<Vec<u8>, StorageError> {
        match compression {
            CompressionType::None => Ok(data),
            _ => {
                // TODO: Implement decompression
                Ok(data)
            }
        }
    }

    fn deserialize_artifact(
        &self,
        data: &[u8],
        artifact_id: &str,
    ) -> Result<Artifact, StorageError> {
        // For now, try to deserialize as different types
        // In practice, you'd store the type information in metadata

        // Try audio first
        if let Ok(audio) = serde_json::from_slice::<crate::artifacts::AudioArtifact>(data) {
            return Ok(Artifact::Audio(audio));
        }

        // Try regions
        if let Ok(regions) = serde_json::from_slice::<crate::artifacts::RegionsArtifact>(data) {
            return Ok(Artifact::Regions(regions));
        }

        Err(StorageError::DeserializationFailed(format!(
            "Cannot deserialize artifact {}",
            artifact_id
        )))
    }

    fn update_access_stats(&self, artifact_id: &str) -> Result<(), StorageError> {
        let mut metadata = self.metadata.write().unwrap();
        if let Some(entry) = metadata.artifacts.get_mut(artifact_id) {
            entry.last_accessed = std::time::SystemTime::now();
            entry.access_count += 1;
        }
        Ok(())
    }

    fn cleanup_cache_if_needed(&self) -> Result<(), StorageError> {
        let cache_size = *self.current_cache_size.read().unwrap();

        if cache_size > self.max_cache_size {
            // Remove least recently used items
            // TODO: Implement LRU cache cleanup
        }

        Ok(())
    }

    fn save_metadata(&self) -> Result<(), StorageError> {
        let metadata = self.metadata.read().unwrap();
        let metadata_path = self.storage_dir.join("metadata.json");
        let data = serde_json::to_string_pretty(&*metadata)?;
        std::fs::write(metadata_path, data)?;
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_artifacts: usize,
    pub total_disk_size: u64,
    pub cached_artifacts: usize,
    pub cache_size_bytes: usize,
    pub max_cache_size: usize,
    pub storage_dir: PathBuf,
}

/// Storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Decompression error: {0}")]
    DecompressionError(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("Storage full")]
    StorageFull,

    #[error("Invalid artifact ID: {0}")]
    InvalidArtifactId(String),
}
