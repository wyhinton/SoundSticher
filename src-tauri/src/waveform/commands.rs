// Tauri commands for waveform service
//
// These commands allow the frontend to request waveforms from the cache.
// Operations do not store waveforms - they request them via this service.

use crate::error::Error;
use crate::waveform::cache::{WaveformCache, WaveformError};
use crate::waveform::types::*;
use std::sync::Arc;
use tauri::State;
/// Waveform service state (wraps the cache)
pub struct WaveformService {
    cache: Arc<WaveformCache>,
}

impl WaveformService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(WaveformCache::new(1000)),
        }
    }

    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(WaveformCache::new(max_entries)),
        }
    }

    /// Get waveform for a single file
    pub fn get_waveform(
        &self,
        file_path: &str,
        spec: &WaveformSpec,
    ) -> Result<WaveformResponse, WaveformError> {
        let audio_key = AudioKey::from_path(file_path)
            .map_err(|e| WaveformError::FileNotFound(e.to_string()))?;

        let (waveform, cache_hit) = self.cache.get_or_compute(&audio_key, spec)?;

        Ok(WaveformResponse {
            audio_key,
            waveform: (*waveform).clone(),
            cache_hit,
        })
    }

    /// Get waveforms for multiple files (batch operation)
    pub fn get_waveforms_batch(
        &self,
        file_paths: &[String],
        spec: &WaveformSpec,
    ) -> BatchWaveformResponse {
        let mut items = Vec::with_capacity(file_paths.len());
        let mut total_cache_hits = 0u32;
        let mut total_computed = 0u32;
        let mut total_errors = 0u32;

        for file_path in file_paths {
            let result = AudioKey::from_path(file_path)
                .map_err(|e| WaveformError::FileNotFound(e.to_string()))
                .and_then(|audio_key| {
                    self.cache
                        .get_or_compute(&audio_key, spec)
                        .map(|(wf, hit)| (audio_key, wf, hit))
                });

            match result {
                Ok((audio_key, waveform, cache_hit)) => {
                    if cache_hit {
                        total_cache_hits += 1;
                    } else {
                        total_computed += 1;
                    }
                    items.push(BatchWaveformItem {
                        file_path: file_path.clone(),
                        audio_key,
                        waveform: Some((*waveform).clone()),
                        error: None,
                        cache_hit,
                    });
                }
                Err(e) => {
                    total_errors += 1;
                    items.push(BatchWaveformItem {
                        file_path: file_path.clone(),
                        audio_key: AudioKey::with_hash(file_path.clone(), 0),
                        waveform: None,
                        error: Some(e.to_string()),
                        cache_hit: false,
                    });
                }
            }
        }

        BatchWaveformResponse {
            items,
            total_cache_hits,
            total_computed,
            total_errors,
        }
    }

    /// Invalidate cached waveform for a file
    pub fn invalidate(&self, file_path: &str) {
        self.cache.invalidate(file_path);
    }

    /// Clear all cached waveforms
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.cache.get_stats()
    }
}

impl Default for WaveformService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Get a single waveform
#[tauri::command]
pub async fn get_waveform(
    service: State<'_, Arc<WaveformService>>,
    request: WaveformRequest,
) -> Result<WaveformResponse, Error> {
    let spec = WaveformSpec::new(request.width, request.height).with_normalize(request.normalize);

    service
        .get_waveform(&request.file_path, &spec)
        .map_err(|e| Error::WaveformError(e.to_string()))
}

/// Get waveforms for multiple files (batch)
#[tauri::command]
pub async fn get_waveforms_batch(
    service: State<'_, Arc<WaveformService>>,
    request: BatchWaveformRequest,
) -> Result<BatchWaveformResponse, Error> {
    let spec = WaveformSpec::new(request.width, request.height).with_normalize(request.normalize);

    Ok(service.get_waveforms_batch(&request.file_paths, &spec))
}

/// Invalidate cached waveform for a file
#[tauri::command]
pub fn invalidate_waveform(
    service: State<'_, Arc<WaveformService>>,
    file_path: String,
) -> Result<(), Error> {
    service.invalidate(&file_path);
    Ok(())
}

/// Clear all cached waveforms
#[tauri::command]
pub fn clear_waveform_cache(service: State<'_, Arc<WaveformService>>) -> Result<(), Error> {
    service.clear();
    Ok(())
}

/// Get waveform cache statistics
#[tauri::command]
pub fn get_waveform_cache_stats(
    service: State<'_, Arc<WaveformService>>,
) -> Result<CacheStats, Error> {
    Ok(service.get_stats())
}

/// Get waveforms for an operation's audio files
/// This is the main entry point when switching operations in the UI
#[tauri::command]
pub async fn get_waveforms_for_operation(
    service: State<'_, Arc<WaveformService>>,
    file_paths: Vec<String>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<BatchWaveformResponse, Error> {
    let spec = WaveformSpec::new(width.unwrap_or(1000), height.unwrap_or(70));

    Ok(service.get_waveforms_batch(&file_paths, &spec))
}
