use crate::artifacts::{ArtifactRecord, ArtifactRegistry, ArtifactRegistryStats};
use crate::error::Error;
use crate::graph::OpId;
use crate::state::AppState;
use crate::util::id::id_utils;
use std::sync::Arc;
use tauri::State;

/// Service for interacting with and debugging the artifact registry
pub struct ArtifactsService;

impl ArtifactsService {
    /// Get all artifact records from the registry
    pub fn get_all_records(registry: &ArtifactRegistry) -> Vec<ArtifactRecord> {
        registry.list_all_records()
    }

    /// Get artifact registry statistics
    pub fn get_stats(registry: &ArtifactRegistry) -> ArtifactRegistryStats {
        registry.get_stats()
    }

    /// Get artifacts created by a specific operation
    pub fn get_records_by_operation(
        registry: &ArtifactRegistry,
        operation_id: &OpId,
    ) -> Vec<ArtifactRecord> {
        registry.get_records_by_op(operation_id)
    }

    /// Clear the entire artifact registry
    pub fn clear_registry(registry: &ArtifactRegistry) {
        registry.clear();
    }

    /// Refresh the existence status of all artifacts
    pub fn refresh_existence_status(registry: &ArtifactRegistry) {
        registry.refresh_existence_status();
    }

    /// Remove artifacts created by a specific operation
    pub fn remove_artifacts_by_operation(
        registry: &ArtifactRegistry,
        operation_id: &OpId,
    ) -> usize {
        let removed = registry.remove_artifacts_by_op(operation_id);
        removed.len()
    }

    /// Get detailed information about a specific artifact
    pub fn get_artifact_details(
        registry: &ArtifactRegistry,
        artifact_id: &str,
    ) -> Option<ArtifactRecord> {
        let artifact_id_string = artifact_id.to_string();
        registry.get_record(&artifact_id_string)
    }
}

// Tauri commands for frontend interaction

/// Get all artifact records with enhanced debugging information
#[tauri::command]
pub async fn get_artifact_debug_info(
    app_state: State<'_, Arc<AppState>>,
) -> Result<ArtifactDebugInfo, Error> {
    let registry = &app_state.artifact_registry;
    let stats = ArtifactsService::get_stats(registry);
    let records = ArtifactsService::get_all_records(registry);
    
    // Convert to frontend-safe records
    let frontend_records: Vec<ArtifactRecordForFrontend> = records
        .into_iter()
        .map(|r| r.into())
        .collect();
    
    // Group records by operation for easier debugging
    let mut records_by_operation = std::collections::HashMap::new();
    for record in &frontend_records {
        records_by_operation
            .entry(record.creator_op_id.clone())
            .or_insert_with(Vec::new)
            .push(record.clone());
    }

    let total_operations_with_artifacts = records_by_operation.len();

    Ok(ArtifactDebugInfo {
        stats,
        all_records: frontend_records,
        total_operations_with_artifacts,
        records_by_operation,
    })
}

/// Get artifact records filtered by various criteria
#[tauri::command]
pub async fn get_filtered_artifacts(
    app_state: State<'_, Arc<AppState>>,
    filter: ArtifactFilter,
) -> Result<Vec<ArtifactRecordForFrontend>, Error> {
    let registry = &app_state.artifact_registry;
    let mut records = ArtifactsService::get_all_records(registry);

    // Apply filters
    if let Some(artifact_type) = &filter.artifact_type {
        records.retain(|r| r.artifact_type == *artifact_type);
    }

    if let Some(exists) = filter.exists {
        records.retain(|r| r.exists == exists);
    }

    if let Some(operation_id) = &filter.operation_id {
        // Convert the operation_id string back to OpId for comparison
        // For now, we'll filter based on string representation of the OpId
        records.retain(|r| {
            let op_id_string = id_utils::id_to_string(r.creator_op_id);
            op_id_string == *operation_id
        });
    }

    if let Some(min_size) = filter.min_size_bytes {
        records.retain(|r| r.size_bytes >= min_size);
    }

    if let Some(max_size) = filter.max_size_bytes {
        records.retain(|r| r.size_bytes <= max_size);
    }

    // Sort by creation time (newest first)
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Convert to frontend-safe records
    let frontend_records: Vec<ArtifactRecordForFrontend> = records
        .into_iter()
        .map(|r| r.into())
        .collect();

    Ok(frontend_records)
}

/// Clear all artifacts from the registry
#[tauri::command]
pub async fn clear_artifact_registry_debug(
    app_state: State<'_, Arc<AppState>>,
) -> Result<String, Error> {
    let registry = &app_state.artifact_registry;
    let count_before = registry.get_stats().total_artifacts;
    
    ArtifactsService::clear_registry(registry);
    
    Ok(format!(
        "Cleared {} artifacts from the registry",
        count_before
    ))
}

/// Refresh existence status of all artifacts
#[tauri::command]
pub async fn refresh_artifacts_existence(
    app_state: State<'_, Arc<AppState>>,
) -> Result<String, Error> {
    let registry = &app_state.artifact_registry;
    ArtifactsService::refresh_existence_status(registry);
    
    let stats = registry.get_stats();
    Ok(format!(
        "Refreshed existence status for {} artifacts ({} existing, {} missing)",
        stats.total_artifacts,
        stats.existing_artifacts,
        stats.total_artifacts - stats.existing_artifacts
    ))
}

/// Remove artifacts by operation ID
#[tauri::command]
pub async fn remove_artifacts_by_operation_debug(
    app_state: State<'_, Arc<AppState>>,
    operation_id: String,
) -> Result<String, Error> {
    let registry = &app_state.artifact_registry;
    
    // Since we can't reconstruct OpId from string, we'll need to find and remove artifacts manually
    let all_records = registry.list_all_records();
    let mut removed_count = 0;
    
    for record in all_records {
        let op_id_string = id_utils::id_to_string(record.creator_op_id);
        if op_id_string == operation_id {
            if registry.remove_artifact(&record.id).is_some() {
                removed_count += 1;
            }
        }
    }
    
    Ok(format!(
        "Removed {} artifacts created by operation '{}'",
        removed_count, operation_id
    ))
}

/// Get detailed artifact information by ID
#[tauri::command]
pub async fn get_artifact_details_debug(
    app_state: State<'_, Arc<AppState>>,
    artifact_id: String,
) -> Result<Option<ArtifactRecordForFrontend>, Error> {
    let registry = &app_state.artifact_registry;
    if let Some(record) = registry.get_record(&artifact_id) {
        Ok(Some(record.into()))
    } else {
        Ok(None)
    }
}

// Data structures for frontend

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactDebugInfo {
    pub stats: ArtifactRegistryStats,
    pub all_records: Vec<ArtifactRecordForFrontend>,
    pub records_by_operation: std::collections::HashMap<String, Vec<ArtifactRecordForFrontend>>,
    pub total_operations_with_artifacts: usize,
}

/// Version of ArtifactRecord that's safe for frontend serialization
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtifactRecordForFrontend {
    pub id: String,
    pub creator_op_id: String,  // Converted to string for serialization
    pub created_at: u64,
    pub artifact_type: String,
    pub size_bytes: u64,
    pub exists: bool,
    pub tags: std::collections::HashMap<String, String>,
    pub file_paths: Vec<String>,
}

impl From<ArtifactRecord> for ArtifactRecordForFrontend {
    fn from(record: ArtifactRecord) -> Self {
        Self {
            id: record.id,
            creator_op_id: id_utils::id_to_string(record.creator_op_id),
            created_at: record.created_at,
            artifact_type: record.artifact_type,
            size_bytes: record.size_bytes,
            exists: record.exists,
            tags: record.tags,
            file_paths: record.file_paths,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactFilter {
    pub artifact_type: Option<String>,
    pub exists: Option<bool>,
    pub operation_id: Option<String>,
    pub min_size_bytes: Option<u64>,
    pub max_size_bytes: Option<u64>,
}

impl Default for ArtifactFilter {
    fn default() -> Self {
        Self {
            artifact_type: None,
            exists: None,
            operation_id: None,
            min_size_bytes: None,
            max_size_bytes: None,
        }
    }
}
