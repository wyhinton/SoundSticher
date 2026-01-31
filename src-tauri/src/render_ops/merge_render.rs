// Merge operation implementation
//
// This operation merges multiple audio artifacts into a single output.
// It supports both in-memory and on-disk input artifacts, consuming them
// agnostically through the hybrid artifact model.

use crate::artifacts::{
    load_audio_to_buffer, write_wav_file, Artifact, AudioArtifact, AudioBuffer, AudioData,
};
use crate::render_ops::{
    OperationCategory, OperationContext, OperationError, OperationResult, RenderOperation,
};
use crate::util::id_utils;

#[derive(Debug)]
pub struct MergeOpRender {
    pub merge_type: MergeType,
}

impl Default for MergeOpRender {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum MergeType {
    /// Concatenate audio files sequentially
    Concatenate,
}

impl MergeOpRender {
    pub fn new() -> Self {
        Self {
            merge_type: MergeType::Concatenate,
        }
    }
}

impl RenderOperation for MergeOpRender {
    fn name(&self) -> &str {
        "merge"
    }

    fn required_inputs(&self) -> Vec<String> {
        vec!["inputs".to_string()]
    }

    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    fn validate_parameters(&self, _parameters: &serde_json::Value) -> Result<(), OperationError> {
        // Only concatenation is supported, no parameters needed
        Ok(())
    }

    fn execute(&self, context: OperationContext) -> OperationResult {
        context.report_progress(0.0);

        // Get input artifacts
        let inputs_artifact = context.get_input("inputs")?;
        let mut input_artifacts = match inputs_artifact {
            Artifact::Audio(audio) => vec![audio.clone()],
            Artifact::AudioList(list) => list.clone(),
            _ => {
                return Err(OperationError::InvalidInput(
                    "Input must be audio or audio list".to_string(),
                ))
            }
        };

        if input_artifacts.is_empty() {
            return Err(OperationError::InvalidInput(
                "At least one input file required".to_string(),
            ));
        }

        context.report_progress(0.1);

        // Perform concatenation - this now handles both in-memory and on-disk artifacts
        let (merged_buffer, _sample_rate, _channels) =
            self.concatenate_audio_buffers(&mut input_artifacts, &context)?;

        context.report_progress(0.9);

        // Create output path for the final file
        let output_path = context.work_dir.join(format!(
            "merged_{}.wav",
            id_utils::friendly_id(context.op_id, "op")
        ));

        // Write the merged output to disk (this is a final output, so we materialize)
        write_wav_file(output_path.clone(), &merged_buffer).map_err(|e| {
            OperationError::AudioError(format!("Failed to write merged audio: {}", e))
        })?;

        // Create output artifact - stores both the path and the in-memory buffer
        let mut output_audio = AudioArtifact::from_buffer(merged_buffer);
        output_audio.path = output_path.clone();
        output_audio.format = "wav".to_string();
        output_audio
            .metadata
            .insert("merge_type".to_string(), "concatenate".to_string());
        output_audio
            .metadata
            .insert("input_count".to_string(), input_artifacts.len().to_string());

        // Register the artifact in the registry with metadata tags
        let mut artifact_tags = std::collections::HashMap::new();
        artifact_tags.insert("operation_type".to_string(), "merge".to_string());
        artifact_tags.insert("merge_strategy".to_string(), "concatenate".to_string());
        artifact_tags.insert("input_count".to_string(), input_artifacts.len().to_string());
        artifact_tags.insert("output_format".to_string(), "wav".to_string());

        let artifact = Artifact::Audio(output_audio.clone());
        if let Ok(Some(artifact_id)) = context.publish_artifact_with_tags(artifact.clone(), artifact_tags) {
            // Artifact successfully registered - optionally log this
            // This could be used later for tracking, cleanup, or dependency management
        }

        context.report_progress(1.0);
        Ok(artifact)
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Audio
    }

    fn description(&self) -> &str {
        "Concatenate multiple audio files sequentially"
    }

    fn estimated_duration(&self, _context: &OperationContext) -> std::time::Duration {
        // Estimate based on input file sizes
        std::time::Duration::from_secs(2)
    }

    fn memory_requirement(&self, _context: &OperationContext) -> usize {
        // Estimate for concatenation - relatively modest memory requirements
        50 * 1024 * 1024 // 50MB
    }
}

impl MergeOpRender {
    /// Concatenate audio from multiple artifacts into a single buffer.
    ///
    /// This method handles both in-memory and on-disk artifacts transparently:
    /// - In-memory artifacts: samples are read directly from the buffer
    /// - On-disk artifacts: samples are loaded from the file
    fn concatenate_audio_buffers(
        &self,
        inputs: &mut [AudioArtifact],
        context: &OperationContext,
    ) -> Result<(AudioBuffer, u32, u32), OperationError> {
        if inputs.is_empty() {
            return Err(OperationError::InvalidInput(
                "No input artifacts to concatenate".to_string(),
            ));
        }

        // Get metadata from first input (borrow ends after these lines)
        let target_sample_rate = inputs[0].sample_rate;
        let target_channels = inputs[0].channels;
        let total_inputs = inputs.len();

        // Collect all samples
        let mut all_samples: Vec<f32> = Vec::new();

        for i in 0..total_inputs {
            let progress = 0.2 + (0.6 * (i as f32 / total_inputs as f32));
            context.report_progress(progress);

            // Get samples from the artifact (handles both in-memory and on-disk)
            let artifact = &mut inputs[i];
            let artifact_sample_rate = artifact.sample_rate;
            let artifact_channels = artifact.channels;
            let samples = self.get_samples_from_artifact(artifact)?;

            // TODO: Add sample rate conversion if needed
            if artifact_sample_rate != target_sample_rate {
                // For now, just warn - proper implementation would resample
                eprintln!(
                    "Warning: Sample rate mismatch in input {}: {} vs target {}",
                    i, artifact_sample_rate, target_sample_rate
                );
            }

            // TODO: Add channel conversion if needed
            if artifact_channels != target_channels {
                eprintln!(
                    "Warning: Channel count mismatch in input {}: {} vs target {}",
                    i, artifact_channels, target_channels
                );
            }

            all_samples.extend(samples);
        }

        let merged_buffer = AudioBuffer::new(all_samples, target_sample_rate, target_channels);

        Ok((merged_buffer, target_sample_rate, target_channels))
    }

    /// Get audio samples from an artifact, regardless of whether it's in-memory or on-disk.
    ///
    /// This is the key abstraction that makes the merge operation agnostic to
    /// how the input artifacts store their data.
    fn get_samples_from_artifact(
        &self,
        artifact: &mut AudioArtifact,
    ) -> Result<Vec<f32>, OperationError> {
        match &artifact.data {
            Some(AudioData::InMemory(buffer)) => {
                // Already in memory - just clone the samples
                Ok(buffer.samples.as_ref().clone())
            }
            Some(AudioData::OnDisk { path, .. }) => {
                // On disk - load into buffer
                let buffer = load_audio_to_buffer(path).map_err(|e| {
                    OperationError::AudioError(format!(
                        "Failed to load audio from {}: {}",
                        path.display(),
                        e
                    ))
                })?;
                Ok(buffer.samples.as_ref().clone())
            }
            Some(AudioData::Reference { .. }) => Err(OperationError::InvalidInput(
                "Cannot merge unresolved reference artifacts".to_string(),
            )),
            None => {
                // Fallback: try to load from path field
                if artifact.path.exists() {
                    let buffer = load_audio_to_buffer(&artifact.path).map_err(|e| {
                        OperationError::AudioError(format!(
                            "Failed to load audio from {}: {}",
                            artifact.path.display(),
                            e
                        ))
                    })?;
                    Ok(buffer.samples.as_ref().clone())
                } else {
                    Err(OperationError::InvalidInput(format!(
                        "No audio data available for artifact: {}",
                        artifact.path.display()
                    )))
                }
            }
        }
    }
}
