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
    // Filter records by comparing the string representation of the OpId
    let all_records = app_state.artifact_registry.list_all_records();
    let filtered: Vec<ArtifactRecord> = all_records
        .into_iter()
        .filter(|record| {
            let op_id_string = id_utils::id_to_string(record.creator_op_id);
            op_id_string == operation_id
        })
        .collect();

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
