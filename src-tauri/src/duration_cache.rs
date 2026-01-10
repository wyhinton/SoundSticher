// Audio duration cache with file metadata invalidation
//
// This cache prevents redundant audio duration computation by caching results
// keyed on (path, mtime, size). When files change, the cache automatically
// invalidates stale entries.

use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::State;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::probe::ProbeResult;
use symphonia::default::get_probe;

pub fn get_file_duration(path: &str) -> Option<f32> {
    let file = std::fs::File::open(path).ok()?;
    // let mreader = std::io::BufReader::new(file);
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    let path = Path::new(&path);
    if let Some(extension) = path.extension() {
        hint.with_extension(&extension.to_string_lossy());
    };

    let probed: ProbeResult = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let format = probed.format;
    let track = format.default_track().or_else(|| format.tracks().first())?;

    let duration = track.codec_params.n_frames?;
    let sample_rate = track.codec_params.sample_rate?;

    let length = duration as f32 / sample_rate as f32;
    log::info!(
        "Got duration: {}, sample_rate: {}, length: {}",
        duration,
        sample_rate,
        length
    );
    Some(duration as f32 / sample_rate as f32)
}

/// Cache key based on file metadata to detect changes
#[derive(Hash, Eq, PartialEq, Clone)]
struct DurationCacheKey {
    path: PathBuf,
    modified: SystemTime,
    size: u64,
}

impl DurationCacheKey {
    fn new(path: PathBuf, modified: SystemTime, size: u64) -> Self {
        Self {
            path,
            modified,
            size,
        }
    }
}

/// Thread-safe duration cache with automatic invalidation on file changes
pub struct DurationCache {
    inner: HashMap<DurationCacheKey, f32>,
}

impl DurationCache {
    /// Create a new empty duration cache
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Get cached duration or compute and cache it
    ///
    /// Returns None if:
    /// - File cannot be accessed
    /// - Duration cannot be computed
    /// - File metadata cannot be read
    ///
    /// Automatically invalidates cache entry if file has changed (different mtime or size)
    pub fn get_or_compute(&mut self, path: &str) -> Option<f32> {
        let path_buf = PathBuf::from(path);

        // Get file metadata to create cache key
        let meta = std::fs::metadata(&path_buf).ok()?;
        let modified = meta.modified().ok()?;
        let size = meta.len();

        let key = DurationCacheKey::new(path_buf, modified, size);

        // Check if already cached
        if let Some(duration) = self.inner.get(&key) {
            log::debug!("Duration cache hit for {}: {:.2}s", path, duration);
            return Some(*duration);
        }

        // Cache miss - compute duration
        log::debug!("Duration cache miss for {}, computing...", path);
        let duration = get_file_duration(path)?;

        // Store in cache
        self.inner.insert(key, duration);
        log::info!("Cached duration for {}: {:.2}s", path, duration);

        Some(duration)
    }

    /// Get a batch of durations, using cache where possible
    pub fn get_or_compute_batch(&mut self, paths: &[String]) -> Vec<(String, Option<f32>)> {
        paths
            .iter()
            .map(|path| (path.clone(), self.get_or_compute(path)))
            .collect()
    }

    /// Invalidate cache entries for a specific file
    ///
    /// Removes all cache entries with matching path, forcing recomputation
    /// on next access
    pub fn invalidate(&mut self, path: &str) {
        let path_buf = PathBuf::from(path);
        let before_count = self.inner.len();
        self.inner.retain(|k, _| k.path != path_buf);
        let removed = before_count - self.inner.len();
        if removed > 0 {
            log::info!("Invalidated {} cache entries for {}", removed, path);
        }
    }

    /// Clear all cached durations
    pub fn clear(&mut self) {
        let count = self.inner.len();
        self.inner.clear();
        if count > 0 {
            log::info!("Cleared {} duration cache entries", count);
        }
    }

    /// Get number of cached entries
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get cache statistics
    pub fn stats(&self) -> DurationCacheStats {
        DurationCacheStats {
            entries: self.inner.len(),
        }
    }

    /// Simple check if a file path might be cached (without full metadata check)
    pub fn is_cached_simple(&self, path: &str) -> bool {
        let path_buf = std::path::PathBuf::from(path);
        self.inner.iter().any(|(key, _)| key.path == path_buf)
    }
}

impl Default for DurationCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the duration cache
#[derive(Clone, Debug, serde::Serialize)]
pub struct DurationCacheStats {
    pub entries: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DurationResponse {
    pub path: String,
    pub duration_seconds: Option<f32>,
}

#[tauri::command]
pub async fn get_duration(
    cache: State<'_, Arc<Mutex<DurationCache>>>,
    path: String,
) -> Result<DurationResponse, String> {
    let mut cache = cache.lock().map_err(|e| format!("Cache lock error: {e}"))?;
    let duration = cache.get_or_compute(&path);
    Ok(DurationResponse {
        path,
        duration_seconds: duration,
    })
}

#[tauri::command]
pub async fn get_durations(
    cache: State<'_, Arc<Mutex<DurationCache>>>,
    paths: Vec<String>,
) -> Result<Vec<DurationResponse>, String> {
    let mut cache = cache.lock().map_err(|e| format!("Cache lock error: {e}"))?;
    let results = cache
        .get_or_compute_batch(&paths)
        .into_iter()
        .map(|(path, duration)| DurationResponse {
            path,
            duration_seconds: duration,
        })
        .collect();
    Ok(results)
}

#[tauri::command]
pub fn invalidate_duration(
    cache: State<'_, Arc<Mutex<DurationCache>>>,
    path: String,
) -> Result<(), String> {
    let mut cache = cache.lock().map_err(|e| format!("Cache lock error: {e}"))?;
    cache.invalidate(&path);
    Ok(())
}

#[tauri::command]
pub fn clear_duration_cache(cache: State<'_, Arc<Mutex<DurationCache>>>) -> Result<(), String> {
    let mut cache = cache.lock().map_err(|e| format!("Cache lock error: {e}"))?;
    cache.clear();
    Ok(())
}

#[tauri::command]
pub fn get_duration_cache_stats(
    cache: State<'_, Arc<Mutex<DurationCache>>>,
) -> Result<DurationCacheStats, String> {
    let cache = cache.lock().map_err(|e| format!("Cache lock error: {e}"))?;
    Ok(cache.stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = DurationCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let cache = DurationCache::new();
        let stats = cache.stats();
        assert_eq!(stats.entries, 0);
    }
}
