use crate::artifacts::{ArtifactRecord, ArtifactRegistry, ArtifactRegistryStats};
use crate::error::Error;
use crate::graph::OpId;
use crate::logging::{LogSystem, LoggingService};
use crate::state::AppState;
use crate::util::id::id_utils;
use std::sync::{Arc, Mutex};
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
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<ArtifactDebugInfo, Error> {
    let registry = &app_state.artifact_registry;
    let logger = logging_service.lock().unwrap();

    logger.debug(
        LogSystem::Artifacts,
        "get_artifact_debug_info called",
        Some("debug"),
    );

    let stats = ArtifactsService::get_stats(registry);
    let records = ArtifactsService::get_all_records(registry);

    logger.debug(
        LogSystem::Artifacts,
        &format!("Retrieved {} artifact records", records.len()),
        Some("debug"),
    );

    // Convert to frontend-safe records
    let frontend_records: Vec<ArtifactRecordForFrontend> =
        records.into_iter().map(|r| r.into()).collect();

    // Group records by operation for easier debugging
    let mut records_by_operation = std::collections::HashMap::new();
    for record in &frontend_records {
        records_by_operation
            .entry(record.creator_op_id.clone())
            .or_insert_with(Vec::new)
            .push(record.clone());
    }

    let total_operations_with_artifacts = records_by_operation.len();

    logger.info(
        LogSystem::Artifacts,
        &format!(
            "Debug info: {} total artifacts across {} operations",
            frontend_records.len(),
            total_operations_with_artifacts
        ),
        Some("debug"),
    );

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
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    filter: ArtifactFilter,
) -> Result<Vec<ArtifactRecordForFrontend>, Error> {
    let registry = &app_state.artifact_registry;
    let logger = logging_service.lock().unwrap();

    logger.debug(
        LogSystem::Artifacts,
        &format!("get_filtered_artifacts called with filter: {:?}", filter),
        Some("filter"),
    );

    let mut records = ArtifactsService::get_all_records(registry);
    let initial_count = records.len();

    logger.debug(
        LogSystem::Artifacts,
        &format!(
            "Starting with {} total artifacts in registry",
            initial_count
        ),
        Some("filter"),
    );

    // Apply filters
    if let Some(artifact_type) = &filter.artifact_type {
        records.retain(|r| r.artifact_type == *artifact_type);
        logger.debug(
            LogSystem::Artifacts,
            &format!(
                "After artifact_type filter '{}': {} records",
                artifact_type,
                records.len()
            ),
            Some("filter"),
        );
    }

    if let Some(exists) = filter.exists {
        records.retain(|r| r.exists == exists);
        logger.debug(
            LogSystem::Artifacts,
            &format!(
                "After exists filter '{}': {} records",
                exists,
                records.len()
            ),
            Some("filter"),
        );
    }

    if let Some(operation_id) = &filter.operation_id {
        logger.debug(
            LogSystem::Artifacts,
            &format!("Filtering by operation_id: '{}'", operation_id),
            Some("filter"),
        );

        // Log all available operation IDs for comparison (both backend and frontend IDs)
        let available_ids: Vec<(String, Option<String>)> = records
            .iter()
            .map(|r| {
                (
                    id_utils::id_to_string(r.creator_op_id),
                    r.frontend_op_id.clone(),
                )
            })
            .collect();

        logger.debug(
            LogSystem::Artifacts,
            &format!(
                "Available operation IDs in registry ({} total): backend_ids={:?}, frontend_ids={:?}",
                available_ids.len(),
                available_ids.iter().map(|(b, _)| b.clone()).collect::<Vec<_>>(),
                available_ids.iter().filter_map(|(_, f)| f.clone()).collect::<Vec<_>>()
            ),
            Some("filter"),
        );

        // Check if the filter looks like a frontend ID (starts with "op_")
        let is_frontend_id = operation_id.starts_with("op_");

        records.retain(|r| {
            let backend_id_string = id_utils::id_to_string(r.creator_op_id);

            // Try to match against frontend_op_id first if it looks like a frontend ID
            let matches = if is_frontend_id {
                // Match against frontend_op_id (preferred for frontend queries)
                let frontend_match = r.frontend_op_id.as_ref() == Some(operation_id);
                // Fallback to backend ID match if no frontend match
                frontend_match || backend_id_string == *operation_id
            } else {
                // Match against backend ID first, then try frontend ID
                let backend_match = backend_id_string == *operation_id;
                let frontend_match = r.frontend_op_id.as_ref() == Some(operation_id);
                backend_match || frontend_match
            };

            // Log each comparison for debugging
            logger.debug(
                LogSystem::Artifacts,
                &format!(
                    "Comparing: filter='{}' vs backend='{}' frontend='{:?}' => matches={}",
                    operation_id, backend_id_string, r.frontend_op_id, matches
                ),
                Some("filter"),
            );

            matches
        });

        logger.debug(
            LogSystem::Artifacts,
            &format!("After operation_id filter: {} records", records.len()),
            Some("filter"),
        );
    }

    if let Some(min_size) = filter.min_size_bytes {
        records.retain(|r| r.size_bytes >= min_size);
        logger.debug(
            LogSystem::Artifacts,
            &format!(
                "After min_size filter '{}': {} records",
                min_size,
                records.len()
            ),
            Some("filter"),
        );
    }

    if let Some(max_size) = filter.max_size_bytes {
        records.retain(|r| r.size_bytes <= max_size);
        logger.debug(
            LogSystem::Artifacts,
            &format!(
                "After max_size filter '{}': {} records",
                max_size,
                records.len()
            ),
            Some("filter"),
        );
    }

    // Sort by creation time (newest first)
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Convert to frontend-safe records
    let frontend_records: Vec<ArtifactRecordForFrontend> =
        records.into_iter().map(|r| r.into()).collect();

    logger.info(
        LogSystem::Artifacts,
        &format!(
            "get_filtered_artifacts returning {} of {} artifacts",
            frontend_records.len(),
            initial_count
        ),
        Some("filter"),
    );

    Ok(frontend_records)
}

/// Clear all artifacts from the registry
#[tauri::command]
pub async fn clear_artifact_registry_debug(
    app_state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<String, Error> {
    let registry = &app_state.artifact_registry;
    let logger = logging_service.lock().unwrap();
    let count_before = registry.get_stats().total_artifacts;

    logger.info(
        LogSystem::Artifacts,
        &format!("Clearing artifact registry ({} artifacts)", count_before),
        Some("clear"),
    );

    ArtifactsService::clear_registry(registry);

    logger.info(
        LogSystem::Artifacts,
        &format!("Cleared {} artifacts from the registry", count_before),
        Some("clear"),
    );

    Ok(format!(
        "Cleared {} artifacts from the registry",
        count_before
    ))
}

/// Refresh existence status of all artifacts
#[tauri::command]
pub async fn refresh_artifacts_existence(
    app_state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<String, Error> {
    let registry = &app_state.artifact_registry;
    let logger = logging_service.lock().unwrap();

    logger.debug(
        LogSystem::Artifacts,
        "Refreshing artifact existence status",
        Some("refresh"),
    );

    ArtifactsService::refresh_existence_status(registry);

    let stats = registry.get_stats();

    logger.info(
        LogSystem::Artifacts,
        &format!(
            "Refreshed existence status: {} total, {} existing, {} missing",
            stats.total_artifacts,
            stats.existing_artifacts,
            stats.total_artifacts - stats.existing_artifacts
        ),
        Some("refresh"),
    );

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
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    operation_id: String,
) -> Result<String, Error> {
    let registry = &app_state.artifact_registry;
    let logger = logging_service.lock().unwrap();

    logger.debug(
        LogSystem::Artifacts,
        &format!("Removing artifacts for operation: {}", operation_id),
        Some("remove"),
    );

    // Since we can't reconstruct OpId from string, we'll need to find and remove artifacts manually
    let all_records = registry.list_all_records();
    let mut removed_count = 0;

    for record in all_records {
        let op_id_string = id_utils::id_to_string(record.creator_op_id);
        if op_id_string == operation_id && registry.remove_artifact(&record.id).is_some() {
            removed_count += 1;
            logger.debug(
                LogSystem::Artifacts,
                &format!("Removed artifact: {}", record.id),
                Some("remove"),
            );
        }
    }

    logger.info(
        LogSystem::Artifacts,
        &format!(
            "Removed {} artifacts for operation '{}'",
            removed_count, operation_id
        ),
        Some("remove"),
    );

    Ok(format!(
        "Removed {} artifacts created by operation '{}'",
        removed_count, operation_id
    ))
}

/// Get detailed artifact information by ID
#[tauri::command]
pub async fn get_artifact_details_debug(
    app_state: State<'_, Arc<AppState>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    artifact_id: String,
) -> Result<Option<ArtifactRecordForFrontend>, Error> {
    let registry = &app_state.artifact_registry;
    let logger = logging_service.lock().unwrap();

    logger.debug(
        LogSystem::Artifacts,
        &format!("Getting artifact details for: {}", artifact_id),
        Some("details"),
    );

    if let Some(record) = registry.get_record(&artifact_id) {
        logger.debug(
            LogSystem::Artifacts,
            &format!(
                "Found artifact: id={}, type={}, size={}",
                record.id, record.artifact_type, record.size_bytes
            ),
            Some("details"),
        );
        Ok(Some(record.into()))
    } else {
        logger.debug(
            LogSystem::Artifacts,
            &format!("Artifact not found: {}", artifact_id),
            Some("details"),
        );
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
    pub creator_op_id: String, // Backend OpId converted to string for serialization
    pub frontend_op_id: Option<String>, // Frontend operation ID for filtering/display
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
            frontend_op_id: record.frontend_op_id,
            created_at: record.created_at,
            artifact_type: record.artifact_type,
            size_bytes: record.size_bytes,
            exists: record.exists,
            tags: record.tags,
            file_paths: record.file_paths,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ArtifactFilter {
    pub artifact_type: Option<String>,
    pub exists: Option<bool>,
    pub operation_id: Option<String>,
    pub min_size_bytes: Option<u64>,
    pub max_size_bytes: Option<u64>,
}
