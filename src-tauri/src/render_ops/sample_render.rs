// Sample-based playable operation
//
// This operation simply reads from a pre-loaded buffer of audio samples.

use crate::artifacts::{Artifact, AudioArtifact, AudioBuffer, AudioData};
use crate::playback::op_playback::AudioSpec;
use crate::render_ops::{
    OperationCategory, OperationContext, OperationError, OperationResult, RenderOperation,
};
use std::path::PathBuf;
use std::sync::Arc;

/// A simple sample-based playable operation that wraps pre-loaded audio data.
///
/// This is the most basic implementation of PlayableOp - it simply reads
/// from a buffer of samples at the requested time.
#[derive(Debug)]
pub struct SampleOpRender {
    /// The audio samples (interleaved for multi-channel)
    samples: Arc<Vec<f32>>,

    /// Audio specification
    spec: AudioSpec,

    /// Total duration in samples (per channel)
    duration_samples: u64,

    /// Optional name for logging/debugging
    name: Option<String>,
}

impl SampleOpRender {
    /// Create a new sample-based playable operation
    pub fn new(samples: Vec<f32>, spec: AudioSpec) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples: Arc::new(samples),
            spec,
            duration_samples,
            name: None,
        }
    }

    /// Create a new sample operation with a name
    pub fn with_name(samples: Vec<f32>, spec: AudioSpec, name: String) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples: Arc::new(samples),
            spec,
            duration_samples,
            name: Some(name),
        }
    }

    /// Create from an existing Arc<Vec<f32>> to share ownership
    pub fn from_arc(samples: Arc<Vec<f32>>, spec: AudioSpec) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples,
            spec,
            duration_samples,
            name: None,
        }
    }

    /// Create from an existing Arc<Vec<f32>> with a name
    pub fn from_arc_with_name(samples: Arc<Vec<f32>>, spec: AudioSpec, name: String) -> Self {
        let duration_samples = samples.len() as u64 / spec.channels as u64;
        Self {
            samples,
            spec,
            duration_samples,
            name: Some(name),
        }
    }

    /// Get a reference to the underlying samples
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Get the name of this operation, if set
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Set the name of this operation
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

impl RenderOperation for SampleOpRender {
    fn name(&self) -> &str {
        "sample_load"
    }

    fn required_inputs(&self) -> Vec<String> {
        // No inputs required - this operation loads from a file path parameter
        Vec::new()
    }

    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the audio file to load"
                },
                "name": {
                    "type": "string",
                    "description": "Optional name for the loaded audio"
                }
            },
            "required": ["file_path"]
        })
    }

    fn validate_parameters(&self, parameters: &serde_json::Value) -> Result<(), OperationError> {
        let file_path: String = serde_json::from_value(
            parameters
                .get("file_path")
                .ok_or_else(|| {
                    OperationError::InvalidInput("Missing file_path parameter".to_string())
                })?
                .clone(),
        )?;

        let path = PathBuf::from(&file_path);
        if !path.exists() {
            return Err(OperationError::InvalidInput(format!(
                "File does not exist: {}",
                file_path
            )));
        }

        Ok(())
    }

    fn execute(&self, context: OperationContext) -> OperationResult {
        context.report_progress(0.0);

        // Get file path from parameters
        let file_path: String = context.get_parameter("file_path")?;
        let name: Option<String> = context.get_parameter("name").ok();

        // Check if we should cache to disk (optional parameter, default false for in-memory)
        let cache_to_disk: bool = context.get_parameter("cache_to_disk").unwrap_or(false);

        context.report_progress(0.1);

        // Load the audio file using symphonia
        let (samples, sample_rate, channels, duration) = load_audio_file(&file_path)?;

        context.report_progress(0.8);

        // Create AudioArtifact - either in-memory or on-disk based on caching preference
        let path = PathBuf::from(&file_path);
        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut artifact = if cache_to_disk {
            // Create a disk-backed artifact that references the source file
            let mut a = AudioArtifact::new(path.clone(), format, sample_rate, channels, duration);

            // But also store the loaded samples in memory for immediate use
            let buffer = AudioBuffer::new(samples, sample_rate, channels);
            a.set_audio_data(AudioData::InMemory(buffer));
            a
        } else {
            // Create an in-memory artifact (preferred for pipeline operations)
            let buffer = AudioBuffer::new(samples, sample_rate, channels);
            let mut a = AudioArtifact::from_buffer(buffer);

            // Store the original file path in metadata for reference
            a.metadata
                .insert("source_file".to_string(), file_path.clone());
            a.metadata.insert("original_format".to_string(), format);
            a
        };

        // Add optional metadata
        if let Some(name) = name {
            artifact = artifact.with_metadata("name".to_string(), name);
        }

        context.report_progress(1.0);

        Ok(Artifact::Audio(artifact))
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::IO
    }

    fn description(&self) -> &str {
        "Load audio from a file"
    }

    fn estimated_duration(&self, _context: &OperationContext) -> std::time::Duration {
        // File loading is typically fast, but can vary based on file size
        std::time::Duration::from_millis(500)
    }

    fn memory_requirement(&self, context: &OperationContext) -> usize {
        // Estimate based on typical audio file sizes
        // Default to ~10MB for an average audio file
        if let Ok(file_path) = context.get_parameter::<String>("file_path") {
            if let Ok(metadata) = std::fs::metadata(&file_path) {
                return metadata.len() as usize;
            }
        }
        10 * 1024 * 1024
    }

    fn is_parallelizable(&self) -> bool {
        true // File loading can be done in parallel
    }
}

/// Load audio file and return samples with metadata
fn load_audio_file(file_path: &str) -> Result<(Vec<f32>, u32, u32, f64), OperationError> {
    use std::fs::File;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = File::open(file_path).map_err(|e| OperationError::IoError(e))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(file_path).extension() {
        hint.with_extension(&ext.to_string_lossy());
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| OperationError::AudioError(format!("Failed to probe file: {}", e)))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| OperationError::AudioError("No audio track found".to_string()))?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| OperationError::AudioError("No sample rate found".to_string()))?;

    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u32)
        .unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| OperationError::AudioError(format!("Failed to create decoder: {}", e)))?;

    let track_id = track.id;
    let mut samples: Vec<f32> = Vec::new();

    // Decode all samples
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                return Err(OperationError::AudioError(format!(
                    "Error reading packet: {}",
                    e
                )))
            }
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

        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        sample_buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(sample_buf.samples());
    }

    // Calculate duration in seconds
    let total_samples = samples.len() as u64 / channels as u64;
    let duration = total_samples as f64 / sample_rate as f64;

    Ok((samples, sample_rate, channels, duration))
}
