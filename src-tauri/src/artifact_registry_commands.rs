use crate::artifacts::{ArtifactRecord, ArtifactRegistryStats};
use crate::error::Error;
use crate::state::AppState;
use crate::util::id::id_utils;
use std::sync::Arc;
use tauri::State;

/// Get all artifact records from the registry
#[tauri::command]
pub async fn get_artifact_registry_records(
    app_state: State<'_, Arc<AppState>>,
) -> Result<Vec<ArtifactRecord>, Error> {
    let records = app_state.artifact_registry.list_all_records();
    Ok(records)
}

/// Get artifact registry statistics
#[tauri::command]
pub async fn get_artifact_registry_stats(
    app_state: State<'_, Arc<AppState>>,
) -> Result<ArtifactRegistryStats, Error> {
    let stats = app_state.artifact_registry.get_stats();
    Ok(stats)
}

/// Get artifacts created by a specific operation
#[tauri::command]
pub async fn get_artifacts_by_operation(
    operation_id: String,
    app_state: State<'_, Arc<AppState>>,
) -> Result<Vec<ArtifactRecord>, Error> {
    eprintln!(
        "[artifact_registry] get_artifacts_by_operation called with operation_id: {}",
        operation_id
    );

    // Get all records from the registry
    let all_records = app_state.artifact_registry.list_all_records();
    eprintln!(
        "[artifact_registry] Total artifacts in registry: {}",
        all_records.len()
    );

    if all_records.is_empty() {
        eprintln!("[artifact_registry] Registry is empty, returning empty list");
        return Ok(vec![]);
    }

    // Filter records by comparing the string representation of the OpId
    let filtered: Vec<ArtifactRecord> = all_records
        .clone()
        .into_iter()
        .filter(|record| {
            let op_id_string = id_utils::id_to_string(record.creator_op_id);
            let matches = op_id_string == operation_id;

            if matches {
                eprintln!(
                    "[artifact_registry] Found matching artifact: id={}, op_id={}",
                    record.id, op_id_string
                );
            }

            matches
        })
        .collect();

    eprintln!(
        "[artifact_registry] Filtered results: {}/{} artifacts match operation_id={}",
        filtered.len(),
        all_records.clone().len(),
        operation_id
    );

    if filtered.is_empty() {
        eprintln!(
            "[artifact_registry] No artifacts found for operation_id: {}",
            operation_id
        );
    }

    Ok(filtered)
}
/// Clear the entire artifact registry
#[tauri::command]
pub async fn clear_artifact_registry(app_state: State<'_, Arc<AppState>>) -> Result<String, Error> {
    app_state.artifact_registry.clear();
    Ok("Artifact registry cleared successfully".to_string())
}

/// Refresh the existence status of all artifacts
#[tauri::command]
pub async fn refresh_artifact_registry_status(
    app_state: State<'_, Arc<AppState>>,
) -> Result<String, Error> {
    app_state.artifact_registry.refresh_existence_status();
    Ok("Artifact registry status refreshed successfully".to_string())
}
