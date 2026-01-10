// Duration service - wraps duration cache with service layer
//
// This service provides a higher-level interface for duration computation
// and caching, following the same pattern as WaveformService.

use crate::duration_cache::DurationCache;
use crate::error::Error;
use crate::logging::{LogSystem, LoggingService};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;

/// Duration service state (wraps the cache)
pub struct DurationService {
    cache: Arc<Mutex<DurationCache>>,
}

impl DurationService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(DurationCache::new())),
        }
    }

    /// Get duration for a single file
    pub fn get_duration(&self, file_path: &str) -> Result<DurationResponse, Error> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| Error::DurationError(format!("Duration cache lock error: {}", e)))?;

        let duration = cache.get_or_compute(file_path);

        Ok(DurationResponse {
            path: file_path.to_string(),
            duration_seconds: duration,
            cache_hit: cache.is_cached_simple(file_path),
        })
    }

    /// Get durations for multiple files (batch operation)
    pub fn get_durations_batch(&self, file_paths: &[String]) -> BatchDurationResponse {
        let mut items = Vec::with_capacity(file_paths.len());
        let mut total_cache_hits = 0u32;
        let mut total_computed = 0u32;
        let mut total_errors = 0u32;

        let cache_result = self.cache.lock();
        if cache_result.is_err() {
            return BatchDurationResponse {
                items: file_paths
                    .iter()
                    .map(|path| BatchDurationItem {
                        path: path.clone(),
                        duration_seconds: None,
                        error: Some("Cache lock error".to_string()),
                        cache_hit: false,
                    })
                    .collect(),
                total_cache_hits: 0,
                total_computed: 0,
                total_errors: file_paths.len() as u32,
            };
        }

        let mut cache = cache_result.unwrap();

        for file_path in file_paths {
            // Check if cached first
            let was_cached = cache.is_cached_simple(file_path);
            let duration = cache.get_or_compute(file_path);

            match duration {
                Some(dur) => {
                    if was_cached {
                        total_cache_hits += 1;
                    } else {
                        total_computed += 1;
                    }
                    items.push(BatchDurationItem {
                        path: file_path.clone(),
                        duration_seconds: Some(dur),
                        error: None,
                        cache_hit: was_cached,
                    });
                }
                None => {
                    total_errors += 1;
                    items.push(BatchDurationItem {
                        path: file_path.clone(),
                        duration_seconds: None,
                        error: Some("Failed to compute duration".to_string()),
                        cache_hit: false,
                    });
                }
            }
        }

        BatchDurationResponse {
            items,
            total_cache_hits,
            total_computed,
            total_errors,
        }
    }

    /// Invalidate cached duration for a file
    pub fn invalidate(&self, file_path: &str) -> Result<(), Error> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| Error::DurationError(format!("Duration cache lock error: {}", e)))?;
        cache.invalidate(file_path);
        Ok(())
    }

    /// Clear all cached durations
    pub fn clear(&self) -> Result<(), Error> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| Error::DurationError(format!("Duration cache lock error: {}", e)))?;
        cache.clear();
        Ok(())
    }

    /// Get cache statistics
    // pub fn get_stats(&self) -> Result<DurationCacheStats, Error> {
    //     self.cache.get
    // }

    /// Get access to the internal cache (for compatibility with existing code)
    pub fn get_cache(&self) -> Arc<Mutex<DurationCache>> {
        self.cache.clone()
    }
}

impl Default for DurationService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TYPES
// ============================================================================

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DurationResponse {
    pub path: String,
    pub duration_seconds: Option<f32>,
    pub cache_hit: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDurationItem {
    pub path: String,
    pub duration_seconds: Option<f32>,
    pub error: Option<String>,
    pub cache_hit: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDurationResponse {
    pub items: Vec<BatchDurationItem>,
    pub total_cache_hits: u32,
    pub total_computed: u32,
    pub total_errors: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DurationRequest {
    pub path: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BatchDurationRequest {
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DurationCacheStats {
    pub entries: usize,
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Get a single duration
#[tauri::command]
pub async fn get_duration(
    service: State<'_, Arc<DurationService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    request: DurationRequest,
) -> Result<DurationResponse, Error> {
    // Log the request
    if let Ok(logger) = logging_service.lock() {
        logger.debug_with_data(
            LogSystem::Duration,
            "Processing single duration request",
            Some("single_request"),
            serde_json::json!({
                "file_path": request.path,
            }),
        );
    }

    let result = service.get_duration(&request.path);

    // Log the result
    if let Ok(logger) = logging_service.lock() {
        match &result {
            Ok(response) => {
                logger.debug_with_data(
                    LogSystem::Duration,
                    "Successfully retrieved duration",
                    Some("single_result"),
                    serde_json::json!({
                        "file_path": request.path,
                        "cache_hit": response.cache_hit,
                        "duration": response.duration_seconds,
                    }),
                );
            }
            Err(e) => {
                logger.error(
                    LogSystem::Duration,
                    &format!("Failed to get duration for {}: {}", request.path, e),
                    Some("single_error"),
                );
            }
        }
    }

    result
}

/// Get durations for multiple files (batch)
#[tauri::command]
pub async fn get_durations_batch(
    service: State<'_, Arc<DurationService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    request: BatchDurationRequest,
) -> Result<BatchDurationResponse, Error> {
    // Log the request
    if let Ok(logger) = logging_service.lock() {
        logger.debug_with_data(
            LogSystem::Duration,
            "Processing batch duration request",
            Some("batch_request"),
            serde_json::json!({
                "file_count": request.paths.len(),
            }),
        );
    }

    let response = service.get_durations_batch(&request.paths);

    // Log the result
    if let Ok(logger) = logging_service.lock() {
        logger.debug_with_data(
            LogSystem::Duration,
            "Completed batch duration request",
            Some("batch_result"),
            serde_json::json!({
                "total_files": request.paths.len(),
                "cache_hits": response.total_cache_hits,
                "computed": response.total_computed,
                "errors": response.total_errors,
            }),
        );
    }

    Ok(response)
}

/// Invalidate duration cache entry
#[tauri::command]
pub async fn invalidate_duration(
    service: State<'_, Arc<DurationService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
    path: String,
) -> Result<(), Error> {
    if let Ok(logger) = logging_service.lock() {
        logger.debug(
            LogSystem::Duration,
            &format!("Invalidating duration cache for: {}", path),
            Some("invalidate"),
        );
    }

    service.invalidate(&path)
}

/// Clear duration cache
#[tauri::command]
pub async fn clear_duration_cache(
    service: State<'_, Arc<DurationService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    if let Ok(logger) = logging_service.lock() {
        logger.info(
            LogSystem::Duration,
            "Clearing duration cache",
            Some("clear"),
        );
    }

    service.clear()
}
