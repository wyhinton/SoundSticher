use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::artifacts::Artifact;
use crate::graph::OpId;
use crate::util::id::id_utils::generate_unique_id;

/// Unique identifier for artifacts in the registry
pub type ArtifactId = String;

/// Metadata record for an artifact in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    /// Unique artifact identifier
    pub id: ArtifactId,

    /// ID of the operation that created this artifact (backend SlotMap key)
    pub creator_op_id: OpId,

    /// Frontend operation ID string (e.g., "op_mkxk4epg_itm7ep")
    /// This is the ID used by the frontend to identify operations
    #[serde(default)]
    pub frontend_op_id: Option<String>,

    /// Timestamp when the artifact was created
    pub created_at: u64,

    /// Type of artifact (from Artifact::artifact_type())
    pub artifact_type: String,

    /// Size of the artifact in bytes
    pub size_bytes: u64,

    /// Whether the artifact still exists/is valid
    pub exists: bool,

    /// Additional metadata tags
    pub tags: HashMap<String, String>,

    /// File paths referenced by this artifact (if any)
    pub file_paths: Vec<String>,
}

impl ArtifactRecord {
    /// Create a new artifact record from an artifact and operation ID
    pub fn from_artifact(
        artifact: &Artifact,
        creator_op_id: OpId,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_artifact_with_frontend_id(artifact, creator_op_id, None)
    }

    /// Create a new artifact record with an optional frontend operation ID
    pub fn from_artifact_with_frontend_id(
        artifact: &Artifact,
        creator_op_id: OpId,
        frontend_op_id: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let id = generate_unique_id();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        Ok(Self {
            id,
            creator_op_id,
            frontend_op_id,
            created_at: now,
            artifact_type: artifact.artifact_type().to_string(),
            size_bytes: artifact.size_bytes()?,
            exists: artifact.exists(),
            tags: HashMap::new(),
            file_paths: artifact
                .get_file_paths()
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        })
    }

    /// Add a metadata tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Update the exists flag by checking the artifact
    pub fn update_exists_status(&mut self, artifact: &Artifact) {
        self.exists = artifact.exists();
        self.size_bytes = artifact.size_bytes().unwrap_or(0);
    }
}

/// Registry for tracking artifacts created by operations
#[derive(Debug)]
pub struct ArtifactRegistry {
    /// Map from artifact ID to artifact record (includes both metadata and artifact data)
    artifacts: DashMap<ArtifactId, (Artifact, ArtifactRecord)>,

    /// Map from operation ID to list of artifact IDs it created
    by_op: DashMap<OpId, Vec<ArtifactId>>,
}

impl Default for ArtifactRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactRegistry {
    /// Create a new empty artifact registry
    pub fn new() -> Self {
        Self {
            artifacts: DashMap::new(),
            by_op: DashMap::new(),
        }
    }

    /// Register an artifact created by an operation
    pub fn register_artifact(
        &self,
        artifact: Artifact,
        creator_op_id: OpId,
    ) -> Result<ArtifactId, Box<dyn std::error::Error>> {
        self.register_artifact_with_frontend_id(artifact, creator_op_id, None)
    }

    /// Register an artifact with a frontend operation ID
    pub fn register_artifact_with_frontend_id(
        &self,
        artifact: Artifact,
        creator_op_id: OpId,
        frontend_op_id: Option<String>,
    ) -> Result<ArtifactId, Box<dyn std::error::Error>> {
        let record = ArtifactRecord::from_artifact_with_frontend_id(&artifact, creator_op_id, frontend_op_id)?;
        let artifact_id = record.id.clone();

        // Store the artifact and record together
        self.artifacts
            .insert(artifact_id.clone(), (artifact, record));

        // Track which operation created this artifact
        self.by_op
            .entry(creator_op_id)
            .or_default()
            .push(artifact_id.clone());

        Ok(artifact_id)
    }

    /// Register an artifact with additional metadata tags
    pub fn register_artifact_with_tags(
        &self,
        artifact: Artifact,
        creator_op_id: OpId,
        tags: HashMap<String, String>,
    ) -> Result<ArtifactId, Box<dyn std::error::Error>> {
        self.register_artifact_with_tags_and_frontend_id(artifact, creator_op_id, tags, None)
    }

    /// Register an artifact with tags and frontend operation ID
    pub fn register_artifact_with_tags_and_frontend_id(
        &self,
        artifact: Artifact,
        creator_op_id: OpId,
        tags: HashMap<String, String>,
        frontend_op_id: Option<String>,
    ) -> Result<ArtifactId, Box<dyn std::error::Error>> {
        let mut record = ArtifactRecord::from_artifact_with_frontend_id(&artifact, creator_op_id, frontend_op_id)?;
        record.tags = tags;
        let artifact_id = record.id.clone();

        // Store the artifact and record together
        self.artifacts
            .insert(artifact_id.clone(), (artifact, record));

        // Track which operation created this artifact
        self.by_op
            .entry(creator_op_id)
            .or_default()
            .push(artifact_id.clone());

        Ok(artifact_id)
    }

    /// Get an artifact by ID
    pub fn get_artifact(&self, artifact_id: &ArtifactId) -> Option<Artifact> {
        self.artifacts.get(artifact_id).map(|entry| entry.0.clone())
    }

    /// Get artifact metadata by ID
    pub fn get_record(&self, artifact_id: &ArtifactId) -> Option<ArtifactRecord> {
        self.artifacts.get(artifact_id).map(|entry| entry.1.clone())
    }

    /// Get all artifacts created by a specific operation
    pub fn get_artifacts_by_op(&self, op_id: &OpId) -> Vec<(ArtifactId, Artifact)> {
        if let Some(artifact_ids) = self.by_op.get(op_id) {
            artifact_ids
                .iter()
                .filter_map(|id| {
                    self.artifacts
                        .get(id)
                        .map(|entry| (id.clone(), entry.0.clone()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all artifact records created by a specific operation
    pub fn get_records_by_op(&self, op_id: &OpId) -> Vec<ArtifactRecord> {
        if let Some(artifact_ids) = self.by_op.get(op_id) {
            artifact_ids
                .iter()
                .filter_map(|id| self.artifacts.get(id).map(|entry| entry.1.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all artifact IDs in the registry
    pub fn list_all_ids(&self) -> Vec<ArtifactId> {
        self.artifacts
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get all artifact records in the registry
    pub fn list_all_records(&self) -> Vec<ArtifactRecord> {
        self.artifacts
            .iter()
            .map(|entry| entry.value().1.clone())
            .collect()
    }

    /// Remove an artifact from the registry
    pub fn remove_artifact(&self, artifact_id: &ArtifactId) -> Option<(Artifact, ArtifactRecord)> {
        if let Some((_, (artifact, record))) = self.artifacts.remove(artifact_id) {
            // Remove from operation tracking
            if let Some(mut ids) = self.by_op.get_mut(&record.creator_op_id) {
                ids.retain(|id| id != artifact_id);
                if ids.is_empty() {
                    drop(ids); // Release the mutable reference
                    self.by_op.remove(&record.creator_op_id);
                }
            }
            Some((artifact, record))
        } else {
            None
        }
    }

    /// Remove all artifacts created by a specific operation
    pub fn remove_artifacts_by_op(
        &self,
        op_id: &OpId,
    ) -> Vec<(ArtifactId, Artifact, ArtifactRecord)> {
        let artifact_ids = if let Some((_, ids)) = self.by_op.remove(op_id) {
            ids
        } else {
            return Vec::new();
        };

        let mut removed = Vec::new();
        for artifact_id in artifact_ids {
            if let Some((_, (artifact, record))) = self.artifacts.remove(&artifact_id) {
                removed.push((artifact_id, artifact, record));
            }
        }

        removed
    }

    /// Update artifact existence status for all artifacts
    pub fn refresh_existence_status(&self) {
        for mut entry in self.artifacts.iter_mut() {
            let (artifact, record) = entry.value_mut();
            record.update_exists_status(artifact);
        }
    }

    /// Get statistics about the registry
    pub fn get_stats(&self) -> ArtifactRegistryStats {
        let total_artifacts = self.artifacts.len();
        let mut total_size = 0u64;
        let mut existing_count = 0;
        let mut type_counts = HashMap::new();

        for entry in self.artifacts.iter() {
            let record = &entry.value().1;
            total_size += record.size_bytes;
            if record.exists {
                existing_count += 1;
            }
            *type_counts.entry(record.artifact_type.clone()).or_insert(0) += 1;
        }

        ArtifactRegistryStats {
            total_artifacts,
            existing_artifacts: existing_count,
            total_size_bytes: total_size,
            operations_with_artifacts: self.by_op.len(),
            artifacts_by_type: type_counts,
        }
    }

    /// Clear the entire registry
    pub fn clear(&self) {
        self.artifacts.clear();
        self.by_op.clear();
    }
}

/// Statistics about the artifact registry
#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactRegistryStats {
    pub total_artifacts: usize,
    pub existing_artifacts: usize,
    pub total_size_bytes: u64,
    pub operations_with_artifacts: usize,
    pub artifacts_by_type: HashMap<String, usize>,
}
