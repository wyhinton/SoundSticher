// Cook task definition and management

use crate::graph::OpId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::{Duration, SystemTime};

/// A cooking task that represents an operation to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookTask {
    /// Unique identifier for the operation this task represents
    pub op_id: OpId,

    /// Type of operation to execute
    pub operation_type: String,

    /// Operation parameters
    pub parameters: serde_json::Value,

    /// Task priority
    pub priority: CookTaskPriority,

    /// Current status of the task
    pub status: TaskStatus,

    /// When this task was created
    pub created_at: SystemTime,

    /// When this task was last modified
    pub updated_at: SystemTime,

    /// Estimated execution time
    pub estimated_duration: Duration,

    /// Estimated memory requirement in bytes
    pub estimated_memory: usize,

    /// Task metadata
    pub metadata: std::collections::HashMap<String, String>,

    /// Whether this task can run in parallel with others
    pub parallelizable: bool,

    /// Task dependencies (operation IDs that must complete first)
    pub dependencies: Vec<OpId>,

    /// Task timeout
    pub timeout: Option<Duration>,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CookTaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
    Immediate = 4,
}

impl Default for CookTaskPriority {
    fn default() -> Self {
        CookTaskPriority::Normal
    }
}

impl CookTaskPriority {
    /// Get numeric value for priority comparison
    pub fn as_i32(&self) -> i32 {
        match self {
            CookTaskPriority::Low => 0,
            CookTaskPriority::Normal => 1,
            CookTaskPriority::High => 2,
            CookTaskPriority::Critical => 3,
            CookTaskPriority::Immediate => 4,
        }
    }

    /// Create priority from numeric value
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => CookTaskPriority::Low,
            1 => CookTaskPriority::Normal,
            2 => CookTaskPriority::High,
            3 => CookTaskPriority::Critical,
            4.. => CookTaskPriority::Immediate,
            _ => CookTaskPriority::Low,
        }
    }
}

/// Current status of a cooking task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is waiting to be scheduled
    Pending,

    /// Task is in the execution queue
    Queued,

    /// Task is currently running
    Running,

    /// Task completed successfully
    Completed,

    /// Task failed with an error
    Failed,

    /// Task was cancelled before completion
    Cancelled,

    /// Task is waiting for dependencies
    WaitingForDependencies,

    /// Task was skipped (e.g., result already cached)
    Skipped,
}

impl TaskStatus {
    /// Check if the task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::Skipped
        )
    }

    /// Check if the task is currently active
    pub fn is_active(&self) -> bool {
        matches!(self, TaskStatus::Running)
    }

    /// Check if the task can be started
    pub fn can_start(&self) -> bool {
        matches!(self, TaskStatus::Queued)
    }
}

impl CookTask {
    /// Create a new cooking task
    pub fn new(op_id: OpId, operation_type: String, parameters: serde_json::Value) -> Self {
        let now = SystemTime::now();

        Self {
            op_id,
            operation_type,
            parameters,
            priority: CookTaskPriority::default(),
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            estimated_duration: Duration::from_secs(1),
            estimated_memory: 1024 * 1024, // 1MB default
            metadata: std::collections::HashMap::new(),
            parallelizable: true,
            dependencies: Vec::new(),
            timeout: None,
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: CookTaskPriority) -> Self {
        self.priority = priority;
        self.updated_at = SystemTime::now();
        self
    }

    /// Set estimated duration
    pub fn with_estimated_duration(mut self, duration: Duration) -> Self {
        self.estimated_duration = duration;
        self.updated_at = SystemTime::now();
        self
    }

    /// Set estimated memory requirement
    pub fn with_estimated_memory(mut self, memory_bytes: usize) -> Self {
        self.estimated_memory = memory_bytes;
        self.updated_at = SystemTime::now();
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self.updated_at = SystemTime::now();
        self
    }

    /// Set parallelizability
    pub fn with_parallelizable(mut self, parallelizable: bool) -> Self {
        self.parallelizable = parallelizable;
        self.updated_at = SystemTime::now();
        self
    }

    /// Add dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<OpId>) -> Self {
        self.dependencies = dependencies;
        self.updated_at = SystemTime::now();
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self.updated_at = SystemTime::now();
        self
    }

    /// Update task status
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = SystemTime::now();
    }

    /// Get task age
    pub fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default()
    }

    /// Get time since last update
    pub fn time_since_update(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.updated_at)
            .unwrap_or_default()
    }

    /// Check if task has timed out
    pub fn is_timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout {
            if self.status == TaskStatus::Running {
                return self.time_since_update() > timeout;
            }
        }
        false
    }

    /// Get task weight for scheduling (combines priority and resource requirements)
    pub fn scheduling_weight(&self) -> f64 {
        let priority_weight = self.priority.as_i32() as f64;
        let age_weight = self.age().as_secs_f64() / 3600.0; // Age in hours
        let memory_weight = -(self.estimated_memory as f64 / (1024.0 * 1024.0)); // Negative because less memory is better
        let duration_weight = -self.estimated_duration.as_secs_f64(); // Negative because shorter is better

        priority_weight * 10.0 + age_weight + memory_weight * 0.1 + duration_weight * 0.01
    }

    /// Validate task configuration
    pub fn validate(&self) -> Result<(), TaskValidationError> {
        if self.operation_type.is_empty() {
            return Err(TaskValidationError::EmptyOperationType);
        }

        if self.estimated_memory == 0 {
            return Err(TaskValidationError::InvalidMemoryEstimate);
        }

        if self.estimated_duration == Duration::ZERO {
            return Err(TaskValidationError::InvalidDurationEstimate);
        }

        // Check for circular dependencies (simplified check)
        if self.dependencies.contains(&self.op_id) {
            return Err(TaskValidationError::CircularDependency);
        }

        Ok(())
    }

    /// Create a copy of this task with a new operation ID
    pub fn duplicate_with_new_id(&self, new_op_id: OpId) -> Self {
        let mut new_task = self.clone();
        new_task.op_id = new_op_id;
        new_task.status = TaskStatus::Pending;
        new_task.created_at = SystemTime::now();
        new_task.updated_at = SystemTime::now();
        new_task
    }

    /// Get a human-readable description of the task
    pub fn description(&self) -> String {
        format!(
            "{} ({}): {:?} - {}",
            self.operation_type,
            self.op_id.data().as_ffi(),
            self.status,
            if let Some(label) = self.metadata.get("label") {
                label.clone()
            } else {
                "No description".to_string()
            }
        )
    }
}

// Implement ordering for priority queue (higher priority first)
impl PartialEq for CookTask {
    fn eq(&self, other: &Self) -> bool {
        self.scheduling_weight() == other.scheduling_weight()
    }
}

impl Eq for CookTask {}

impl PartialOrd for CookTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CookTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher scheduling weight should come first in the priority queue
        self.scheduling_weight()
            .partial_cmp(&other.scheduling_weight())
            .unwrap_or(Ordering::Equal)
            .reverse()
    }
}

/// Task validation errors
#[derive(Debug, thiserror::Error)]
pub enum TaskValidationError {
    #[error("Operation type cannot be empty")]
    EmptyOperationType,

    #[error("Invalid memory estimate")]
    InvalidMemoryEstimate,

    #[error("Invalid duration estimate")]
    InvalidDurationEstimate,

    #[error("Task has circular dependency")]
    CircularDependency,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Task builder for fluent task creation
#[derive(Debug)]
pub struct TaskBuilder {
    op_id: OpId,
    operation_type: String,
    parameters: serde_json::Value,
    priority: CookTaskPriority,
    estimated_duration: Duration,
    estimated_memory: usize,
    metadata: std::collections::HashMap<String, String>,
    parallelizable: bool,
    dependencies: Vec<OpId>,
    timeout: Option<Duration>,
}

impl TaskBuilder {
    pub fn new(op_id: OpId, operation_type: String) -> Self {
        Self {
            op_id,
            operation_type,
            parameters: serde_json::Value::Object(Default::default()),
            priority: CookTaskPriority::default(),
            estimated_duration: Duration::from_secs(1),
            estimated_memory: 1024 * 1024,
            metadata: std::collections::HashMap::new(),
            parallelizable: true,
            dependencies: Vec::new(),
            timeout: None,
        }
    }

    pub fn parameters(mut self, parameters: serde_json::Value) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn priority(mut self, priority: CookTaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn estimated_duration(mut self, duration: Duration) -> Self {
        self.estimated_duration = duration;
        self
    }

    pub fn estimated_memory(mut self, memory_bytes: usize) -> Self {
        self.estimated_memory = memory_bytes;
        self
    }

    pub fn metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn parallelizable(mut self, parallelizable: bool) -> Self {
        self.parallelizable = parallelizable;
        self
    }

    pub fn dependencies(mut self, dependencies: Vec<OpId>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build(self) -> CookTask {
        let now = SystemTime::now();

        CookTask {
            op_id: self.op_id,
            operation_type: self.operation_type,
            parameters: self.parameters,
            priority: self.priority,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            estimated_duration: self.estimated_duration,
            estimated_memory: self.estimated_memory,
            metadata: self.metadata,
            parallelizable: self.parallelizable,
            dependencies: self.dependencies,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_ordering() {
        let low = CookTaskPriority::Low;
        let high = CookTaskPriority::High;

        assert!(high > low);
        assert_eq!(high.as_i32(), 2);
        assert_eq!(low.as_i32(), 0);
    }

    #[test]
    fn test_task_scheduling_weight() {
        let mut task1 = CookTask::new(OpId::default(), "test".to_string(), serde_json::json!({}));
        task1.priority = CookTaskPriority::High;

        let mut task2 = CookTask::new(OpId::default(), "test".to_string(), serde_json::json!({}));
        task2.priority = CookTaskPriority::Low;

        assert!(task1.scheduling_weight() > task2.scheduling_weight());
    }
}
