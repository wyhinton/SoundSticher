// Split operation implementation

use crate::artifacts::{Artifact, AudioArtifact};
use crate::ops::{Operation, OperationCategory, OperationContext, OperationError, OperationResult};

#[derive(Debug)]
pub struct SplitOperation {
    pub split_type: SplitType,
}

#[derive(Debug, Clone)]
pub enum SplitType {
    /// Split by time intervals
    TimeSegments,
    /// Split by silence detection
    SilenceDetection,
    /// Split into equal duration chunks
    EqualChunks,
}

impl SplitOperation {
    pub fn new(split_type: SplitType) -> Self {
        Self { split_type }
    }
}

impl Operation for SplitOperation {
    fn name(&self) -> &str {
        "split"
    }

    fn required_inputs(&self) -> Vec<String> {
        vec!["input".to_string()]
    }

    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "split_type": {
                    "type": "string",
                    "enum": ["time_segments", "silence_detection", "equal_chunks", "channels"],
                    "default": "equal_chunks"
                },
                "segment_duration": {
                    "type": "number",
                    "minimum": 0.1,
                    "default": 10.0,
                    "description": "Duration in seconds for equal chunks"
                },
                "time_points": {
                    "type": "array",
                    "items": { "type": "number", "minimum": 0 },
                    "description": "Time points in seconds for time_segments split"
                },
                "silence_threshold": {
                    "type": "number",
                    "minimum": -60,
                    "maximum": 0,
                    "default": -30,
                    "description": "Silence threshold in dB for silence detection"
                },
                "min_silence_duration": {
                    "type": "number",
                    "minimum": 0.1,
                    "default": 1.0,
                    "description": "Minimum silence duration in seconds"
                },
                "padding": {
                    "type": "number",
                    "minimum": 0,
                    "default": 0.1,
                    "description": "Padding around split points in seconds"
                }
            },
            "required": ["split_type"]
        })
    }

    fn validate_parameters(&self, parameters: &serde_json::Value) -> Result<(), OperationError> {
        let split_type: String = serde_json::from_value(
            parameters
                .get("split_type")
                .unwrap_or(&serde_json::json!("equal_chunks"))
                .clone(),
        )?;

        match split_type.as_str() {
            "time_segments" | "silence_detection" | "equal_chunks" | "channels" => Ok(()),
            _ => Err(OperationError::InvalidInput(format!(
                "Invalid split type: {}",
                split_type
            ))),
        }
    }

    fn execute(&self, context: OperationContext) -> OperationResult {
        context.report_progress(0.0);

        // Get input artifact
        let input_artifact = context.get_input("input")?;
        let input_audio = match input_artifact {
            Artifact::Audio(audio) => audio,
            _ => {
                return Err(OperationError::InvalidInput(
                    "Input must be audio".to_string(),
                ))
            }
        };

        context.report_progress(0.1);

        // Get parameters
        let split_type: String = context.get_parameter("split_type")?;

        context.report_progress(0.2);

        // Perform split operation based on type
        let output_segments = match split_type.as_str() {
            "time_segments" => {
                let time_points: Vec<f64> = context.get_parameter("time_points")?;
                self.split_by_time_points(input_audio, &time_points, &context)?
            }
            "silence_detection" => {
                let threshold: f64 = context.get_parameter("silence_threshold").unwrap_or(-30.0);
                let min_duration: f64 =
                    context.get_parameter("min_silence_duration").unwrap_or(1.0);
                self.split_by_silence(input_audio, threshold, min_duration, &context)?
            }
            "equal_chunks" => {
                let chunk_duration: f64 = context.get_parameter("segment_duration").unwrap_or(10.0);
                self.split_into_equal_chunks(input_audio, chunk_duration, &context)?
            }
            _ => {
                return Err(OperationError::InvalidInput(format!(
                    "Unsupported split type: {}",
                    split_type
                )));
            }
        };

        context.report_progress(1.0);
        Ok(Artifact::AudioList(output_segments))
    }

    fn category(&self) -> OperationCategory {
        OperationCategory::Audio
    }

    fn description(&self) -> &str {
        "Split audio files into multiple segments using various methods"
    }

    fn estimated_duration(&self, _context: &OperationContext) -> std::time::Duration {
        match self.split_type {
            SplitType::TimeSegments | SplitType::EqualChunks => std::time::Duration::from_secs(1),
            SplitType::SilenceDetection => std::time::Duration::from_secs(5), // Requires analysis
        }
    }

    fn memory_requirement(&self, _context: &OperationContext) -> usize {
        match self.split_type {
            SplitType::SilenceDetection => 200 * 1024 * 1024, // 200MB - needs full audio in memory
            _ => 50 * 1024 * 1024,                            // 50MB for other types
        }
    }
}

impl SplitOperation {
    fn split_by_time_points(
        &self,
        input: &AudioArtifact,
        time_points: &[f64],
        context: &OperationContext,
    ) -> Result<Vec<AudioArtifact>, OperationError> {
        let mut segments = Vec::new();
        let _padding: f64 = context.get_parameter("padding").unwrap_or(0.1);

        // Add start and end points
        let mut all_points = vec![0.0];
        all_points.extend_from_slice(time_points);
        all_points.push(input.duration);
        all_points.sort_by(|a, b| a.partial_cmp(b).unwrap());

        for (i, window) in all_points.windows(2).enumerate() {
            let start_time = window[0];
            let end_time = window[1];

            // Skip if segment is too short
            if end_time - start_time < 0.1 {
                continue;
            }

            let progress = 0.2 + (0.7 * (i as f32 / (all_points.len() - 1) as f32));
            context.report_progress(progress);

            let segment = self.create_segment(input, i, start_time, end_time, &context.work_dir)?;

            segments.push(segment);
        }

        Ok(segments)
    }

    fn split_by_silence(
        &self,
        input: &AudioArtifact,
        _threshold_db: f64,
        _min_silence_duration: f64,
        context: &OperationContext,
    ) -> Result<Vec<AudioArtifact>, OperationError> {
        // TODO: Implement silence detection algorithm
        // This would require:
        // 1. Load audio data
        // 2. Calculate amplitude/RMS over time
        // 3. Find regions below threshold
        // 4. Merge adjacent silent regions
        // 5. Split at silence boundaries

        context.report_progress(0.3);

        // Placeholder: Split into 3 segments for demonstration
        let segment_duration = input.duration / 3.0;
        let mut segments = Vec::new();

        for i in 0..3 {
            let start_time = i as f64 * segment_duration;
            let end_time = ((i + 1) as f64 * segment_duration).min(input.duration);

            context.report_progress(0.3 + (0.6 * (i as f32 / 3.0)));

            let segment = self.create_segment(input, i, start_time, end_time, &context.work_dir)?;

            segments.push(segment);
        }

        Ok(segments)
    }

    fn split_into_equal_chunks(
        &self,
        input: &AudioArtifact,
        chunk_duration: f64,
        context: &OperationContext,
    ) -> Result<Vec<AudioArtifact>, OperationError> {
        let num_chunks = (input.duration / chunk_duration).ceil() as usize;
        let mut segments = Vec::new();

        for i in 0..num_chunks {
            let start_time = i as f64 * chunk_duration;
            let end_time = ((i + 1) as f64 * chunk_duration).min(input.duration);

            context.report_progress(0.2 + (0.7 * (i as f32 / num_chunks as f32)));

            let segment = self.create_segment(input, i, start_time, end_time, &context.work_dir)?;

            segments.push(segment);
        }

        Ok(segments)
    }

    fn create_segment(
        &self,
        input: &AudioArtifact,
        index: usize,
        start_time: f64,
        end_time: f64,
        work_dir: &std::path::Path,
    ) -> Result<AudioArtifact, OperationError> {
        // Create output path
        let output_path = work_dir.join(format!(
            "split_{}_{:03}.wav",
            input.path.file_stem().unwrap_or_default().to_string_lossy(),
            index
        ));

        // TODO: Implement actual audio segment extraction
        // This would use an audio library to:
        // 1. Load the input file
        // 2. Extract samples between start_time and end_time
        // 3. Write to output file

        // Placeholder for now
        std::fs::write(&output_path, b"placeholder_segment_audio")?;

        Ok(AudioArtifact {
            path: output_path,
            format: input.format.clone(),
            sample_rate: input.sample_rate,
            channels: input.channels,
            duration: end_time - start_time,
            metadata: {
                let mut meta = input.metadata.clone();
                meta.insert("start_time".to_string(), start_time.to_string());
                meta.insert("end_time".to_string(), end_time.to_string());
                meta.insert("segment_index".to_string(), index.to_string());
                meta
            },
        })
    }
}
