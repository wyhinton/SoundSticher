// Global waveform cache with lazy + deduplicated generation
//
// This cache is not per-operation - waveforms are global view artifacts
// that can be shared across multiple operations.

use crate::combine::generate_waveform_path;
use crate::waveform::types::*;
use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};
use tokio::sync::oneshot;

/// Cached waveform entry with LRU tracking
#[derive(Clone)]
struct CachedWaveform {
    waveform: Arc<Waveform>,
    last_accessed: Instant,
    access_count: u64,
}

/// In-flight computation state
struct InFlightComputation {
    /// Receivers waiting for this computation to complete
    waiters: Vec<oneshot::Sender<Arc<Waveform>>>,
}

/// Global waveform cache with LRU eviction and deduplication
pub struct WaveformCache {
    /// Completed waveforms
    completed: RwLock<HashMap<String, CachedWaveform>>,
    /// In-flight computations (prevents duplicate work)
    in_flight: Mutex<HashMap<String, InFlightComputation>>,
    /// Maximum cache size in entries
    max_entries: usize,
    /// Cache statistics
    stats: RwLock<CacheStats>,
}

impl WaveformCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            completed: RwLock::new(HashMap::new()),
            in_flight: Mutex::new(HashMap::new()),
            max_entries,
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Get a waveform from cache or compute it
    pub fn get_or_compute(
        &self,
        audio_key: &AudioKey,
        spec: &WaveformSpec,
    ) -> Result<(Arc<Waveform>, bool), WaveformError> {
        let cache_key = WaveformCacheKey::new(audio_key.clone(), spec.clone());
        let key_str = cache_key.to_string_key();

        // Fast path: check cache first (read lock)
        {
            let cache = self.completed.read().unwrap();
            if let Some(cached) = cache.get(&key_str) {
                // Update stats
                if let Ok(mut stats) = self.stats.write() {
                    stats.hits += 1;
                }
                return Ok((cached.waveform.clone(), true));
            }
        }

        // Cache miss - need to compute
        if let Ok(mut stats) = self.stats.write() {
            stats.misses += 1;
        }

        // Compute the waveform
        let start = Instant::now();
        let waveform = self.compute_waveform(audio_key, spec)?;
        let compute_time = start.elapsed();

        let waveform = Arc::new(waveform);

        // Store in cache (write lock)
        {
            let mut cache = self.completed.write().unwrap();

            // Check if we need to evict
            if cache.len() >= self.max_entries {
                self.evict_lru(&mut cache);
            }

            cache.insert(
                key_str,
                CachedWaveform {
                    waveform: waveform.clone(),
                    last_accessed: Instant::now(),
                    access_count: 1,
                },
            );
        }

        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            stats.total_compute_time_ms += compute_time.as_millis() as u64;
        }

        Ok((waveform, false))
    }

    /// Update last accessed time for a cache entry
    pub fn touch(&self, audio_key: &AudioKey, spec: &WaveformSpec) {
        let cache_key = WaveformCacheKey::new(audio_key.clone(), spec.clone());
        let key_str = cache_key.to_string_key();

        if let Ok(mut cache) = self.completed.write() {
            if let Some(entry) = cache.get_mut(&key_str) {
                entry.last_accessed = Instant::now();
                entry.access_count += 1;
            }
        }
    }

    /// Evict least recently used entries
    fn evict_lru(&self, cache: &mut HashMap<String, CachedWaveform>) {
        // Find the oldest entry
        let oldest_key = cache
            .iter()
            .min_by_key(|(_, v)| v.last_accessed)
            .map(|(k, _)| k.clone());

        if let Some(key) = oldest_key {
            cache.remove(&key);
            if let Ok(mut stats) = self.stats.write() {
                stats.evictions += 1;
            }
        }
    }

    /// Compute waveform for an audio file
    fn compute_waveform(
        &self,
        audio_key: &AudioKey,
        spec: &WaveformSpec,
    ) -> Result<Waveform, WaveformError> {
        let file_path = &audio_key.source_id;

        // Load samples using symphonia (same as combine.rs)
        let samples = self.load_samples(file_path)?;

        if samples.is_empty() {
            return Ok(Waveform::empty());
        }

        // Generate SVG path using the existing function from combine.rs
        let svg_path =
            generate_waveform_path(&samples, spec.width as usize, spec.height as usize, 0.0);

        // Calculate peaks for alternative rendering
        let peaks = self.calculate_peaks(&samples, spec.width as usize, spec.normalize);

        // Calculate duration (assuming 44100 sample rate for now)
        let sample_rate = 44100u32;
        let duration = samples.len() as f64 / sample_rate as f64;

        Ok(Waveform::new(
            svg_path,
            peaks,
            sample_rate,
            duration,
            samples.len(),
            spec.width,
            spec.height,
        ))
    }

    /// Load audio samples from a file
    fn load_samples(&self, file_path: &str) -> Result<Vec<i16>, WaveformError> {
        let file = File::open(file_path).map_err(|e| WaveformError::FileNotFound(e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let probed = get_probe()
            .format(
                &Default::default(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| WaveformError::DecodeError(e.to_string()))?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| WaveformError::DecodeError("No default track found".to_string()))?;

        let mut decoder = get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| WaveformError::DecodeError(e.to_string()))?;

        let mut samples: Vec<i16> = Vec::new();

        while let Ok(packet) = format.next_packet() {
            if let Ok(decoded) = decoder.decode(&packet) {
                let spec = *decoded.spec();
                let mut sample_buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
                sample_buf.copy_interleaved_ref(decoded);
                samples.extend(sample_buf.samples().iter().copied());
            }
        }

        Ok(samples)
    }

    /// Calculate min/max peaks for waveform
    fn calculate_peaks(&self, samples: &[i16], width: usize, normalize: bool) -> Vec<(f32, f32)> {
        if samples.is_empty() || width == 0 {
            return Vec::new();
        }

        let samples_per_pixel = samples.len() / width.max(1);
        let mut peaks = Vec::with_capacity(width);

        // Find max amplitude if normalizing
        let max_amp = if normalize {
            samples
                .iter()
                .map(|s| s.abs() as f32)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(1.0)
        } else {
            i16::MAX as f32
        };

        for x in 0..width {
            let start = x * samples_per_pixel;
            let end = ((x + 1) * samples_per_pixel).min(samples.len());

            let slice = &samples[start..end];
            if slice.is_empty() {
                peaks.push((0.0, 0.0));
                continue;
            }

            let min = *slice.iter().min().unwrap_or(&0) as f32 / max_amp;
            let max = *slice.iter().max().unwrap_or(&0) as f32 / max_amp;

            peaks.push((min, max));
        }

        peaks
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Clear all cached waveforms
    pub fn clear(&self) {
        let mut cache = self.completed.write().unwrap();
        cache.clear();
    }

    /// Invalidate waveform for a specific file (when file changes)
    pub fn invalidate(&self, file_path: &str) {
        let mut cache = self.completed.write().unwrap();
        // Remove all entries that match this file path
        cache.retain(|k, _| !k.starts_with(file_path));
    }

    /// Get number of cached entries
    pub fn len(&self) -> usize {
        self.completed.read().unwrap().len()
    }

    /// Check if a waveform is cached
    pub fn is_cached(&self, audio_key: &AudioKey, spec: &WaveformSpec) -> bool {
        let cache_key = WaveformCacheKey::new(audio_key.clone(), spec.clone());
        let key_str = cache_key.to_string_key();
        self.completed.read().unwrap().contains_key(&key_str)
    }
}

/// Waveform cache error types
#[derive(Debug, Clone)]
pub enum WaveformError {
    FileNotFound(String),
    DecodeError(String),
    CacheError(String),
}

impl std::fmt::Display for WaveformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaveformError::FileNotFound(path) => write!(f, "File not found: {}", path),
            WaveformError::DecodeError(msg) => write!(f, "Decode error: {}", msg),
            WaveformError::CacheError(msg) => write!(f, "Cache error: {}", msg),
        }
    }
}

impl std::error::Error for WaveformError {}

// Global singleton instance
lazy_static::lazy_static! {
    pub static ref WAVEFORM_CACHE: WaveformCache = WaveformCache::new(1000);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key = AudioKey::with_hash("test.wav".to_string(), 12345);
        let spec = WaveformSpec::default();
        let cache_key = WaveformCacheKey::new(key, spec);
        let key_str = cache_key.to_string_key();
        assert!(key_str.contains("test.wav"));
        assert!(key_str.contains("12345"));
    }

    #[test]
    fn test_waveform_spec_defaults() {
        let spec = WaveformSpec::default();
        assert_eq!(spec.width, 1000);
        assert_eq!(spec.height, 70);
        assert!(!spec.normalize);
    }
}
