// Audio regions artifact implementation

use crate::artifacts::{
    AudioArtifact, CompressionType, StorableArtifact, StorageHints, StoragePriority,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Regions artifact representing audio segments with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionsArtifact {
    /// The source audio file these regions reference
    pub source_audio: AudioArtifact,

    /// List of audio regions
    pub regions: Vec<AudioRegion>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Raw region data for efficient storage
    pub data: Vec<u8>,
}

/// Individual audio region with timing and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRegion {
    /// Unique identifier for this region
    pub id: String,

    /// Start time in seconds
    pub start_time: f64,

    /// End time in seconds  
    pub end_time: f64,

    /// Optional label/name for the region
    pub label: Option<String>,

    /// Region-specific metadata
    pub metadata: HashMap<String, String>,

    /// Optional color for UI display
    pub color: Option<String>,

    /// Whether this region is currently selected/active
    pub selected: bool,
}

impl AudioRegion {
    pub fn new(id: String, start_time: f64, end_time: f64) -> Self {
        Self {
            id,
            start_time,
            end_time,
            label: None,
            metadata: HashMap::new(),
            color: None,
            selected: false,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the duration of this region
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Check if this region contains a given time point
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time <= self.end_time
    }

    /// Check if this region overlaps with another region
    pub fn overlaps_with(&self, other: &AudioRegion) -> bool {
        !(self.end_time <= other.start_time || other.end_time <= self.start_time)
    }

    /// Get the overlap duration with another region
    pub fn overlap_duration(&self, other: &AudioRegion) -> f64 {
        if !self.overlaps_with(other) {
            return 0.0;
        }

        let overlap_start = self.start_time.max(other.start_time);
        let overlap_end = self.end_time.min(other.end_time);
        overlap_end - overlap_start
    }

    /// Split this region at a given time point
    pub fn split_at(&self, split_time: f64) -> Option<(AudioRegion, AudioRegion)> {
        if split_time <= self.start_time || split_time >= self.end_time {
            return None;
        }

        let first = AudioRegion {
            id: format!("{}_1", self.id),
            start_time: self.start_time,
            end_time: split_time,
            label: self.label.as_ref().map(|l| format!("{} (1)", l)),
            metadata: self.metadata.clone(),
            color: self.color.clone(),
            selected: false,
        };

        let second = AudioRegion {
            id: format!("{}_2", self.id),
            start_time: split_time,
            end_time: self.end_time,
            label: self.label.as_ref().map(|l| format!("{} (2)", l)),
            metadata: self.metadata.clone(),
            color: self.color.clone(),
            selected: false,
        };

        Some((first, second))
    }

    /// Validate the region
    pub fn validate(&self) -> Result<(), RegionValidationError> {
        if self.start_time < 0.0 {
            return Err(RegionValidationError::NegativeStartTime(self.start_time));
        }

        if self.end_time <= self.start_time {
            return Err(RegionValidationError::InvalidTimeRange {
                start: self.start_time,
                end: self.end_time,
            });
        }

        Ok(())
    }
}

impl RegionsArtifact {
    pub fn new(source_audio: AudioArtifact) -> Self {
        Self {
            source_audio,
            regions: Vec::new(),
            metadata: HashMap::new(),
            data: Vec::new(),
        }
    }

    /// Add a region to this artifact
    pub fn add_region(&mut self, region: AudioRegion) -> Result<(), RegionValidationError> {
        region.validate()?;

        // Validate against source audio duration
        if region.end_time > self.source_audio.duration {
            return Err(RegionValidationError::ExceedsSourceDuration {
                region_end: region.end_time,
                source_duration: self.source_audio.duration,
            });
        }

        self.regions.push(region);
        Ok(())
    }

    /// Remove a region by ID
    pub fn remove_region(&mut self, region_id: &str) -> Option<AudioRegion> {
        if let Some(index) = self.regions.iter().position(|r| r.id == region_id) {
            Some(self.regions.remove(index))
        } else {
            None
        }
    }

    /// Get a region by ID
    pub fn get_region(&self, region_id: &str) -> Option<&AudioRegion> {
        self.regions.iter().find(|r| r.id == region_id)
    }

    /// Get mutable reference to a region by ID
    pub fn get_region_mut(&mut self, region_id: &str) -> Option<&mut AudioRegion> {
        self.regions.iter_mut().find(|r| r.id == region_id)
    }

    /// Get all regions that contain a given time point
    pub fn regions_at_time(&self, time: f64) -> Vec<&AudioRegion> {
        self.regions
            .iter()
            .filter(|r| r.contains_time(time))
            .collect()
    }

    /// Get all regions within a time range
    pub fn regions_in_range(&self, start_time: f64, end_time: f64) -> Vec<&AudioRegion> {
        self.regions
            .iter()
            .filter(|r| !(r.end_time <= start_time || r.start_time >= end_time))
            .collect()
    }

    /// Get total duration covered by all regions
    pub fn total_region_duration(&self) -> f64 {
        self.regions.iter().map(|r| r.duration()).sum()
    }

    /// Get the earliest start time
    pub fn earliest_start_time(&self) -> Option<f64> {
        self.regions
            .iter()
            .map(|r| r.start_time)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Get the latest end time
    pub fn latest_end_time(&self) -> Option<f64> {
        self.regions
            .iter()
            .map(|r| r.end_time)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Sort regions by start time
    pub fn sort_by_start_time(&mut self) {
        self.regions
            .sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    }

    /// Find overlapping regions
    pub fn find_overlaps(&self) -> Vec<(usize, usize)> {
        let mut overlaps = Vec::new();

        for (i, region1) in self.regions.iter().enumerate() {
            for (j, region2) in self.regions.iter().enumerate().skip(i + 1) {
                if region1.overlaps_with(region2) {
                    overlaps.push((i, j));
                }
            }
        }

        overlaps
    }

    /// Merge overlapping regions
    pub fn merge_overlapping_regions(&mut self) {
        self.sort_by_start_time();
        let mut merged: Vec<AudioRegion> = Vec::new();

        for region in self.regions.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.overlaps_with(&region) {
                    // Merge regions
                    last.end_time = last.end_time.max(region.end_time);
                    if last.label.is_none() {
                        last.label = region.label;
                    }
                    continue;
                }
            }
            merged.push(region);
        }

        self.regions = merged;
    }

    /// Validate all regions
    pub fn validate_all(&self) -> Vec<(usize, RegionValidationError)> {
        let mut errors = Vec::new();

        for (index, region) in self.regions.iter().enumerate() {
            if let Err(error) = region.validate() {
                errors.push((index, error));
            }

            // Check against source duration
            if region.end_time > self.source_audio.duration {
                errors.push((
                    index,
                    RegionValidationError::ExceedsSourceDuration {
                        region_end: region.end_time,
                        source_duration: self.source_audio.duration,
                    },
                ));
            }
        }

        errors
    }

    /// Export regions as separate audio files
    pub fn export_regions(
        &self,
        output_dir: &std::path::Path,
    ) -> Result<Vec<AudioArtifact>, RegionExportError> {
        let mut exported = Vec::new();

        for region in &self.regions {
            let output_path = output_dir.join(format!(
                "{}_{}.{}",
                region.label.as_deref().unwrap_or(&region.id),
                region.id,
                self.source_audio.extension()
            ));

            // TODO: Implement actual audio extraction
            // This would involve:
            // 1. Load source audio
            // 2. Extract samples for the time range
            // 3. Write to new file

            // Placeholder for now
            std::fs::write(&output_path, b"placeholder_region_audio")?;

            let region_audio = AudioArtifact {
                path: output_path,
                format: self.source_audio.format.clone(),
                sample_rate: self.source_audio.sample_rate,
                channels: self.source_audio.channels,
                duration: region.duration(),
                metadata: {
                    let mut meta = self.source_audio.metadata.clone();
                    meta.extend(region.metadata.clone());
                    meta.insert("region_id".to_string(), region.id.clone());
                    if let Some(ref label) = region.label {
                        meta.insert("region_label".to_string(), label.clone());
                    }
                    meta
                },
                data: None,
            };

            exported.push(region_audio);
        }

        Ok(exported)
    }
}

impl StorableArtifact for RegionsArtifact {
    fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(self)?)
    }

    fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(data)?)
    }

    fn get_id(&self) -> String {
        format!("regions_{}", self.source_audio.get_id())
    }

    fn storage_hints(&self) -> StorageHints {
        StorageHints {
            compression: CompressionType::Lz4,
            priority: StoragePriority::Normal,
            temporary: false,
        }
    }
}

/// Region validation errors
#[derive(Debug, thiserror::Error)]
pub enum RegionValidationError {
    #[error("Negative start time: {0}")]
    NegativeStartTime(f64),

    #[error("Invalid time range: start={start}, end={end}")]
    InvalidTimeRange { start: f64, end: f64 },

    #[error("Region end time {region_end} exceeds source duration {source_duration}")]
    ExceedsSourceDuration {
        region_end: f64,
        source_duration: f64,
    },
}

/// Region export errors
#[derive(Debug, thiserror::Error)]
pub enum RegionExportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Audio processing error: {0}")]
    AudioError(String),

    #[error("Invalid region: {0}")]
    InvalidRegion(String),
}
