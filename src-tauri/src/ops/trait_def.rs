// Operation trait definition

use crate::artifacts::Artifact;
use crate::graph::OpId;
use std::collections::HashMap;

/// Result of an operation execution
pub type OperationResult = Result<Artifact, OperationError>;

/// Errors that can occur during operation execution
#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Audio processing error: {0}")]
    AudioError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Context provided to operations during execution
pub struct OperationContext {
    /// The operation's unique identifier
    pub op_id: OpId,

    /// Input artifacts from dependencies
    pub inputs: HashMap<String, Artifact>,

    /// Operation parameters
    pub parameters: serde_json::Value,

    /// Working directory for temporary files
    pub work_dir: std::path::PathBuf,

    /// Optional progress callback
    pub progress_callback: Option<Box<dyn Fn(f32) + Send + Sync>>,
}

impl std::fmt::Debug for OperationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationContext")
            .field("op_id", &self.op_id)
            .field("inputs", &self.inputs)
            .field("parameters", &self.parameters)
            .field("work_dir", &self.work_dir)
            .field(
                "progress_callback",
                &self.progress_callback.as_ref().map(|_| "<callback>"),
            )
            .finish()
    }
}

impl OperationContext {
    /// Get an input artifact by name
    pub fn get_input(&self, name: &str) -> Result<&Artifact, OperationError> {
        self.inputs
            .get(name)
            .ok_or_else(|| OperationError::MissingDependency(format!("Missing input: {}", name)))
    }

    /// Get a parameter value
    pub fn get_parameter<T>(&self, name: &str) -> Result<T, OperationError>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = self
            .parameters
            .get(name)
            .ok_or_else(|| OperationError::InvalidInput(format!("Missing parameter: {}", name)))?;

        serde_json::from_value(value.clone()).map_err(|e| {
            OperationError::InvalidInput(format!("Invalid parameter '{}': {}", name, e))
        })
    }

    /// Report progress (0.0 to 1.0)
    pub fn report_progress(&self, progress: f32) {
        if let Some(ref callback) = self.progress_callback {
            callback(progress.clamp(0.0, 1.0));
        }
    }
}

/// Trait that all operations must implement
pub trait Operation: Send + Sync + std::fmt::Debug {
    /// Get the operation name/type
    fn name(&self) -> &str;

    /// Get the required input names
    fn required_inputs(&self) -> Vec<String>;

    /// Get the optional input names
    fn optional_inputs(&self) -> Vec<String> {
        Vec::new()
    }

    /// Get the expected parameter schema (JSON Schema)
    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    /// Validate parameters before execution
    fn validate_parameters(&self, parameters: &serde_json::Value) -> Result<(), OperationError> {
        // Default implementation - no validation
        Ok(())
    }

    /// Execute the operation
    fn execute(&self, context: OperationContext) -> OperationResult;

    /// Get estimated execution time (for scheduling)
    fn estimated_duration(&self, _context: &OperationContext) -> std::time::Duration {
        std::time::Duration::from_secs(1) // Default 1 second
    }

    /// Get memory requirements estimate (in bytes)
    fn memory_requirement(&self, _context: &OperationContext) -> usize {
        1024 * 1024 // Default 1MB
    }

    /// Check if this operation can run in parallel with others
    fn is_parallelizable(&self) -> bool {
        true // Most operations can run in parallel
    }

    /// Get operation category for UI grouping
    fn category(&self) -> OperationCategory {
        OperationCategory::Audio
    }

    /// Get operation description
    fn description(&self) -> &str {
        "No description available"
    }
}

/// Categories for grouping operations in UI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCategory {
    Audio,
    Effects,
    Analysis,
    IO,
    Utility,
}

impl std::fmt::Display for OperationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationCategory::Audio => write!(f, "Audio"),
            OperationCategory::Effects => write!(f, "Effects"),
            OperationCategory::Analysis => write!(f, "Analysis"),
            OperationCategory::IO => write!(f, "Input/Output"),
            OperationCategory::Utility => write!(f, "Utility"),
        }
    }
}

/// Registry for operation types
#[derive(Debug)]
pub struct OperationRegistry {
    operations: HashMap<String, Box<dyn Operation>>,
}

impl OperationRegistry {
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
        }
    }

    /// Register a new operation type
    pub fn register<T>(&mut self, operation: T)
    where
        T: Operation + 'static,
    {
        let name = operation.name().to_string();
        self.operations.insert(name, Box::new(operation));
    }

    /// Get an operation by name
    pub fn get(&self, name: &str) -> Option<&dyn Operation> {
        self.operations.get(name).map(|op| op.as_ref())
    }

    /// List all registered operation names
    pub fn list_operations(&self) -> Vec<&str> {
        self.operations.keys().map(|s| s.as_str()).collect()
    }

    /// Get operations by category
    pub fn get_by_category(&self, category: OperationCategory) -> Vec<&dyn Operation> {
        self.operations
            .values()
            .filter(|op| op.category() == category)
            .map(|op| op.as_ref())
            .collect()
    }
}
