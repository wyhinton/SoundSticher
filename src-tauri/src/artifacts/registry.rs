use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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
    
    /// ID of the operation that created this artifact
    pub creator_op_id: OpId,
    
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
    pub fn from_artifact(artifact: &Artifact, creator_op_id: OpId) -> Result<Self, Box<dyn std::error::Error>> {
        let id = generate_unique_id();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        Ok(Self {
            id,
            creator_op_id,
            created_at: now,
            artifact_type: artifact.artifact_type().to_string(),
            size_bytes: artifact.size_bytes()?,
            exists: artifact.exists(),
            tags: HashMap::new(),
            file_paths: artifact.get_file_paths()
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
    /// Map from artifact ID to artifact data
    artifacts: Arc<Mutex<HashMap<ArtifactId, Artifact>>>,
    
    /// Map from artifact ID to metadata record
    records: Arc<Mutex<HashMap<ArtifactId, ArtifactRecord>>>,
    
    /// Map from operation ID to list of artifact IDs it created
    op_artifacts: Arc<Mutex<HashMap<OpId, Vec<ArtifactId>>>>,
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
            artifacts: Arc::new(Mutex::new(HashMap::new())),
            records: Arc::new(Mutex::new(HashMap::new())),
            op_artifacts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Register an artifact created by an operation
    pub fn register_artifact(&self, artifact: Artifact, creator_op_id: OpId) -> Result<ArtifactId, Box<dyn std::error::Error>> {
        let record = ArtifactRecord::from_artifact(&artifact, creator_op_id)?;
        let artifact_id = record.id.clone();
        
        // Store the artifact and record
        {
            let mut artifacts = self.artifacts.lock().unwrap();
            artifacts.insert(artifact_id.clone(), artifact);
        }
        
        {
            let mut records = self.records.lock().unwrap();
            records.insert(artifact_id.clone(), record);
        }
        
        // Track which operation created this artifact
        {
            let mut op_artifacts = self.op_artifacts.lock().unwrap();
            op_artifacts.entry(creator_op_id).or_insert_with(Vec::new).push(artifact_id.clone());
        }
        
        Ok(artifact_id)
    }
    
    /// Register an artifact with additional metadata tags
    pub fn register_artifact_with_tags(
        &self, 
        artifact: Artifact, 
        creator_op_id: OpId, 
        tags: HashMap<String, String>
    ) -> Result<ArtifactId, Box<dyn std::error::Error>> {
        let mut record = ArtifactRecord::from_artifact(&artifact, creator_op_id)?;
        record.tags = tags;
        let artifact_id = record.id.clone();
        
        // Store the artifact and record
        {
            let mut artifacts = self.artifacts.lock().unwrap();
            artifacts.insert(artifact_id.clone(), artifact);
        }
        
        {
            let mut records = self.records.lock().unwrap();
            records.insert(artifact_id.clone(), record);
        }
        
        // Track which operation created this artifact
        {
            let mut op_artifacts = self.op_artifacts.lock().unwrap();
            op_artifacts.entry(creator_op_id).or_insert_with(Vec::new).push(artifact_id.clone());
        }
        
        Ok(artifact_id)
    }
    
    /// Get an artifact by ID
    pub fn get_artifact(&self, artifact_id: &ArtifactId) -> Option<Artifact> {
        let artifacts = self.artifacts.lock().unwrap();
        artifacts.get(artifact_id).cloned()
    }
    
    /// Get artifact metadata by ID
    pub fn get_record(&self, artifact_id: &ArtifactId) -> Option<ArtifactRecord> {
        let records = self.records.lock().unwrap();
        records.get(artifact_id).cloned()
    }
    
    /// Get all artifacts created by a specific operation
    pub fn get_artifacts_by_op(&self, op_id: &OpId) -> Vec<(ArtifactId, Artifact)> {
        let op_artifacts = self.op_artifacts.lock().unwrap();
        let artifacts = self.artifacts.lock().unwrap();
        
        if let Some(artifact_ids) = op_artifacts.get(op_id) {
            artifact_ids.iter()
                .filter_map(|id| {
                    artifacts.get(id).map(|artifact| (id.clone(), artifact.clone()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all artifact records created by a specific operation
    pub fn get_records_by_op(&self, op_id: &OpId) -> Vec<ArtifactRecord> {
        let op_artifacts = self.op_artifacts.lock().unwrap();
        let records = self.records.lock().unwrap();
        
        if let Some(artifact_ids) = op_artifacts.get(op_id) {
            artifact_ids.iter()
                .filter_map(|id| records.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get all artifact IDs in the registry
    pub fn list_all_ids(&self) -> Vec<ArtifactId> {
        let artifacts = self.artifacts.lock().unwrap();
        artifacts.keys().cloned().collect()
    }
    
    /// Get all artifact records in the registry
    pub fn list_all_records(&self) -> Vec<ArtifactRecord> {
        let records = self.records.lock().unwrap();
        records.values().cloned().collect()
    }
    
    /// Remove an artifact from the registry
    pub fn remove_artifact(&self, artifact_id: &ArtifactId) -> Option<(Artifact, ArtifactRecord)> {
        let artifact = {
            let mut artifacts = self.artifacts.lock().unwrap();
            artifacts.remove(artifact_id)
        };
        
        let record = {
            let mut records = self.records.lock().unwrap();
            records.remove(artifact_id)
        };
        
        // Remove from operation tracking
        if let Some(ref record) = record {
            let mut op_artifacts = self.op_artifacts.lock().unwrap();
            if let Some(ids) = op_artifacts.get_mut(&record.creator_op_id) {
                ids.retain(|id| id != artifact_id);
                if ids.is_empty() {
                    op_artifacts.remove(&record.creator_op_id);
                }
            }
        }
        
        match (artifact, record) {
            (Some(artifact), Some(record)) => Some((artifact, record)),
            _ => None,
        }
    }
    
    /// Remove all artifacts created by a specific operation
    pub fn remove_artifacts_by_op(&self, op_id: &OpId) -> Vec<(ArtifactId, Artifact, ArtifactRecord)> {
        let artifact_ids = {
            let mut op_artifacts = self.op_artifacts.lock().unwrap();
            op_artifacts.remove(op_id).unwrap_or_default()
        };
        
        let mut removed = Vec::new();
        for artifact_id in artifact_ids {
            if let Some((artifact, record)) = self.remove_artifact(&artifact_id) {
                removed.push((artifact_id, artifact, record));
            }
        }
        
        removed
    }
    
    /// Update artifact existence status for all artifacts
    pub fn refresh_existence_status(&self) {
        let artifacts = self.artifacts.lock().unwrap();
        let mut records = self.records.lock().unwrap();
        
        for (id, artifact) in artifacts.iter() {
            if let Some(record) = records.get_mut(id) {
                record.update_exists_status(artifact);
            }
        }
    }
    
    /// Get statistics about the registry
    pub fn get_stats(&self) -> ArtifactRegistryStats {
        let artifacts = self.artifacts.lock().unwrap();
        let records = self.records.lock().unwrap();
        let op_artifacts = self.op_artifacts.lock().unwrap();
        
        let total_artifacts = artifacts.len();
        let total_size = records.values()
            .map(|r| r.size_bytes)
            .sum::<u64>();
        
        let existing_count = records.values()
            .filter(|r| r.exists)
            .count();
        
        let mut type_counts = HashMap::new();
        for record in records.values() {
            *type_counts.entry(record.artifact_type.clone()).or_insert(0) += 1;
        }
        
        ArtifactRegistryStats {
            total_artifacts,
            existing_artifacts: existing_count,
            total_size_bytes: total_size,
            operations_with_artifacts: op_artifacts.len(),
            artifacts_by_type: type_counts,
        }
    }
    
    /// Clear the entire registry
    pub fn clear(&self) {
        let mut artifacts = self.artifacts.lock().unwrap();
        let mut records = self.records.lock().unwrap();
        let mut op_artifacts = self.op_artifacts.lock().unwrap();
        
        artifacts.clear();
        records.clear();
        op_artifacts.clear();
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
