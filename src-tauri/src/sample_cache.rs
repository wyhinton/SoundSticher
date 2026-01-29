use crate::log_info;
use crate::logging::{LogSystem, LoggingService};
use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const MAX_SAMPLE_CACHE_BYTES: u64 = 6 * 1024 * 1024 * 1024; // 1GB

#[derive(Debug)]
pub struct AudioBuffer {
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: usize,
    pub data: Vec<f32>, // interleaved audio data
}

impl AudioBuffer {
    pub fn new(data: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        let frames = data.len() / channels as usize;
        Self {
            channels,
            sample_rate,
            frames,
            data,
        }
    }

    /// Get the memory footprint in bytes
    pub fn memory_size(&self) -> usize {
        self.data.len() * 4 // f32 = 4 bytes
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SampleCacheKey {
    pub path: PathBuf,
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct SampleCacheService {
    cache: Cache<SampleCacheKey, Arc<AudioBuffer>>,
}

impl SampleCacheService {
    pub fn new() -> Self {
        let cache = Cache::builder()
            .max_capacity(MAX_SAMPLE_CACHE_BYTES)
            .weigher(|_key, value: &Arc<AudioBuffer>| {
                // f32 = 4 bytes
                value.memory_size() as u32
            })
            .build();

        Self { cache }
    }

    pub fn get_or_load(
        &self,
        path: PathBuf,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Arc<AudioBuffer>, String> {
        let key = SampleCacheKey {
            path: path.clone(),
            sample_rate,
            channels,
        };

        // Fast path: check cache first
        if let Some(buffer) = self.cache.get(&key) {
            return Ok(buffer);
        }

        // Slow path: load and decode audio
        let samples = self.load_audio_samples(&path, sample_rate, channels)?;
        let buffer = Arc::new(AudioBuffer::new(samples, sample_rate, channels));

        self.cache.insert(key, buffer.clone());
        Ok(buffer)
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Remove a specific file from cache (all sample rates/channels)
    pub fn invalidate_file(&self, path: &PathBuf) {
        let keys: Vec<SampleCacheKey> = self
            .cache
            .iter()
            .filter(|(key, _)| key.path == *path)
            .map(|(key, _)| (*key).clone())
            .collect();

        for key in keys {
            self.cache.invalidate(&key);
        }
    }
    /// Get cache statistics
    pub fn stats(&self) -> SampleCacheStats {
        SampleCacheStats {
            entry_count: self.cache.entry_count(),
            weighted_size: self.cache.weighted_size(),
        }
    }

    fn load_audio_samples(
        &self,
        file_path: &PathBuf,
        target_sample_rate: u32,
        target_channels: u16,
    ) -> Result<Vec<f32>, String> {
        let file_path_str = file_path.to_string_lossy();

        let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = file_path.extension() {
            hint.with_extension(&ext.to_string_lossy());
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("Failed to probe file: {}", e))?;

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or("No audio track found")?;

        // Get source audio properties
        let source_sample_rate = track
            .codec_params
            .sample_rate
            .ok_or("No sample rate found in audio file")?;

        let source_channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1) as u16;

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        let track_id = track.id;
        let mut samples: Vec<f32> = Vec::new();

        // Decode all samples first
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => return Err(format!("Error reading packet: {}", e)),
            };

            if packet.track_id() != track_id {
                continue;
            }

            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(e) => {
                    eprintln!("Error decoding packet: {}", e);
                    continue;
                }
            };

            let mut sample_buf =
                SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
            sample_buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(sample_buf.samples());
        }

        // Apply channel conversion if needed
        let samples = if source_channels != target_channels {
            eprintln!(
                "Converting audio channels: {} -> {} channels for file: {}",
                source_channels, target_channels, file_path_str
            );
            convert_channels(&samples, source_channels, target_channels)?
        } else {
            samples
        };

        // Apply sample rate conversion if needed
        let samples = if source_sample_rate != target_sample_rate {
            eprintln!(
                "Resampling audio: {}Hz -> {}Hz for file: {}",
                source_sample_rate, target_sample_rate, file_path_str
            );
            resample_audio(
                &samples,
                source_sample_rate,
                target_sample_rate,
                target_channels,
            )?
        } else {
            samples
        };

        eprintln!(
            "Loaded audio file: {} ({}Hz, {} channels, {} samples)",
            file_path_str,
            target_sample_rate,
            target_channels,
            samples.len()
        );

        Ok(samples)
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleCacheStats {
    pub entry_count: u64,
    pub weighted_size: u64,
}

/// Convert between different channel counts
fn convert_channels(
    samples: &[f32],
    source_channels: u16,
    target_channels: u16,
) -> Result<Vec<f32>, String> {
    if source_channels == target_channels {
        return Ok(samples.to_vec());
    }

    let frames = samples.len() / source_channels as usize;
    let mut output = Vec::with_capacity(frames * target_channels as usize);

    for frame_idx in 0..frames {
        let source_frame_start = frame_idx * source_channels as usize;

        match (source_channels, target_channels) {
            // Mono to Stereo: duplicate the mono channel
            (1, 2) => {
                let mono_sample = samples[source_frame_start];
                output.push(mono_sample); // Left
                output.push(mono_sample); // Right
            }
            // Stereo to Mono: average left and right channels
            (2, 1) => {
                let left = samples[source_frame_start];
                let right = samples[source_frame_start + 1];
                let mono = (left + right) * 0.5;
                output.push(mono);
            }
            // Multi-channel to Stereo: downmix by averaging all channels
            (src, 2) if src > 2 => {
                let mut sum = 0.0;
                for ch in 0..src {
                    sum += samples[source_frame_start + ch as usize];
                }
                let avg = sum / src as f32;
                output.push(avg); // Left
                output.push(avg); // Right
            }
            // Multi-channel to Mono: downmix by averaging all channels
            (src, 1) if src > 1 => {
                let mut sum = 0.0;
                for ch in 0..src {
                    sum += samples[source_frame_start + ch as usize];
                }
                let avg = sum / src as f32;
                output.push(avg);
            }
            // Unsupported conversion
            (src, tgt) => {
                return Err(format!(
                    "Unsupported channel conversion: {} -> {} channels",
                    src, tgt
                ));
            }
        }
    }

    Ok(output)
}

/// Simple linear interpolation resampler
fn resample_audio(
    samples: &[f32],
    source_rate: u32,
    target_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, String> {
    if source_rate == target_rate {
        return Ok(samples.to_vec());
    }

    let source_frames = samples.len() / channels as usize;
    let ratio = target_rate as f64 / source_rate as f64;
    let target_frames = (source_frames as f64 * ratio).ceil() as usize;

    let mut output = Vec::with_capacity(target_frames * channels as usize);

    for target_frame in 0..target_frames {
        let source_pos = target_frame as f64 / ratio;
        let source_frame = source_pos.floor() as usize;
        let frac = source_pos - source_frame as f64;

        for ch in 0..channels {
            let ch_idx = ch as usize;

            // Get current sample
            let current_idx = source_frame * channels as usize + ch_idx;
            let current_sample = if current_idx < samples.len() {
                samples[current_idx]
            } else {
                0.0 // Pad with silence if beyond end
            };

            // Get next sample for interpolation
            let next_idx = (source_frame + 1) * channels as usize + ch_idx;
            let next_sample = if next_idx < samples.len() {
                samples[next_idx]
            } else {
                current_sample // Use current if no next sample
            };

            // Linear interpolation
            let interpolated = current_sample + (next_sample - current_sample) * frac as f32;
            output.push(interpolated);
        }
    }

    Ok(output)
}

/// Clear the sample cache
#[tauri::command]
pub fn clear_sample_cache(
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) {
    if let Ok(logger) = logging_service.lock() {
        log_info!(logger, LogSystem::Combine, "Clearing sample cache");
    }

    sample_cache.clear();
}

/// Get sample cache statistics
#[tauri::command]
pub fn get_sample_cache_stats(
    sample_cache: State<'_, Arc<SampleCacheService>>,
) -> SampleCacheStats {
    sample_cache.stats()
}

/// Invalidate a specific file in the sample cache
#[tauri::command]
pub fn invalidate_sample_cache(
    file_path: String,
    sample_cache: State<'_, Arc<SampleCacheService>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!("Invalidating sample cache for: {}", file_path)
        );
    }

    let path = std::path::PathBuf::from(file_path);
    sample_cache.invalidate_file(&path);
}
