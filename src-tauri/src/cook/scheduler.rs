// Cook scheduler for managing operation execution

use crate::artifacts::{Artifact, ArtifactStorage};
use crate::cook::{CookTask, CookTaskPriority, TaskStatus};
use crate::graph::{InvalidationManager, OpId, OperationNodeManager};
use crate::ops::{Operation, OperationContext, OperationRegistry, OperationResult};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Scheduler for executing cooking tasks
#[derive(Debug)]
pub struct CookScheduler {
    /// Operation registry for looking up operations
    operation_registry: Arc<OperationRegistry>,

    /// Node manager for operation nodes
    node_manager: Arc<Mutex<OperationNodeManager>>,

    /// Invalidation manager for dependency tracking
    invalidation_manager: Arc<Mutex<InvalidationManager>>,

    /// Artifact storage for inputs/outputs
    artifact_storage: Arc<ArtifactStorage>,

    /// Task queue (priority queue)
    task_queue: Arc<Mutex<BinaryHeap<CookTask>>>,

    /// Currently executing tasks
    executing_tasks: Arc<Mutex<HashMap<OpId, TaskHandle>>>,

    /// Completed tasks cache
    completed_tasks: Arc<Mutex<HashMap<OpId, TaskResult>>>,

    /// Worker threads
    worker_handles: Vec<thread::JoinHandle<()>>,

    /// Channels for communication
    task_sender: mpsc::Sender<SchedulerMessage>,
    task_receiver: Arc<Mutex<mpsc::Receiver<SchedulerMessage>>>,

    /// Scheduler configuration
    config: SchedulerConfig,

    /// Scheduler state
    state: Arc<Mutex<SchedulerState>>,
}

#[derive(Debug)]
struct TaskHandle {
    task: CookTask,
    started_at: Instant,
    thread_handle: Option<thread::JoinHandle<TaskResult>>,
}

#[derive(Debug, Clone)]
struct TaskResult {
    task_id: OpId,
    result: Result<Artifact, String>,
    execution_time: Duration,
    memory_used: usize,
}

#[derive(Debug)]
enum SchedulerMessage {
    TaskCompleted(TaskResult),
    TaskFailed(OpId, String),
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_concurrent_tasks: usize,
    pub max_memory_usage: usize,
    pub task_timeout: Duration,
    pub enable_caching: bool,
    pub work_directory: std::path::PathBuf,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: num_cpus::get(),
            max_memory_usage: 1024 * 1024 * 1024,   // 1GB
            task_timeout: Duration::from_secs(300), // 5 minutes
            enable_caching: true,
            work_directory: std::env::temp_dir(),
        }
    }
}

#[derive(Debug)]
struct SchedulerState {
    is_running: bool,
    total_tasks_executed: usize,
    total_execution_time: Duration,
    current_memory_usage: usize,
    last_cleanup: Instant,
}

impl CookScheduler {
    pub fn new(
        operation_registry: Arc<OperationRegistry>,
        node_manager: Arc<Mutex<OperationNodeManager>>,
        invalidation_manager: Arc<Mutex<InvalidationManager>>,
        artifact_storage: Arc<ArtifactStorage>,
        config: SchedulerConfig,
    ) -> Self {
        let (task_sender, task_receiver) = mpsc::channel();

        Self {
            operation_registry,
            node_manager,
            invalidation_manager,
            artifact_storage,
            task_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            executing_tasks: Arc::new(Mutex::new(HashMap::new())),
            completed_tasks: Arc::new(Mutex::new(HashMap::new())),
            worker_handles: Vec::new(),
            task_sender,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
            config,
            state: Arc::new(Mutex::new(SchedulerState {
                is_running: false,
                total_tasks_executed: 0,
                total_execution_time: Duration::new(0, 0),
                current_memory_usage: 0,
                last_cleanup: Instant::now(),
            })),
        }
    }

    /// Start the scheduler
    pub fn start(&mut self) -> Result<(), SchedulerError> {
        let mut state = self.state.lock().unwrap();
        if state.is_running {
            return Err(SchedulerError::AlreadyRunning);
        }
        state.is_running = true;
        drop(state);

        // Start worker threads
        for worker_id in 0..self.config.max_concurrent_tasks {
            let worker_handle = self.start_worker_thread(worker_id);
            self.worker_handles.push(worker_handle);
        }

        Ok(())
    }

    /// Stop the scheduler
    pub fn stop(&mut self) -> Result<(), SchedulerError> {
        let mut state = self.state.lock().unwrap();
        if !state.is_running {
            return Ok(());
        }
        state.is_running = false;
        drop(state);

        // Send shutdown messages
        for _ in 0..self.config.max_concurrent_tasks {
            let _ = self.task_sender.send(SchedulerMessage::Shutdown);
        }

        // Wait for workers to finish
        for handle in self.worker_handles.drain(..) {
            let _ = handle.join();
        }

        Ok(())
    }

    /// Submit a task for execution
    pub fn submit_task(&self, mut task: CookTask) -> Result<(), SchedulerError> {
        let state = self.state.lock().unwrap();
        if !state.is_running {
            return Err(SchedulerError::NotRunning);
        }
        drop(state);

        // Check if task is already completed or executing
        {
            let executing = self.executing_tasks.lock().unwrap();
            if executing.contains_key(&task.op_id) {
                return Err(SchedulerError::TaskAlreadyExecuting(task.op_id));
            }
        }

        {
            let completed = self.completed_tasks.lock().unwrap();
            if completed.contains_key(&task.op_id) {
                // Task already completed, no need to rerun
                return Ok(());
            }
        }

        // Set task status and add to queue
        task.status = TaskStatus::Queued;
        let mut queue = self.task_queue.lock().unwrap();
        queue.push(task);

        Ok(())
    }

    /// Get the next task to execute
    fn get_next_task(&self) -> Option<CookTask> {
        let mut queue = self.task_queue.lock().unwrap();

        while let Some(task) = queue.pop() {
            // Check if all dependencies are satisfied
            if self.are_dependencies_satisfied(&task) {
                return Some(task);
            } else {
                // Put task back in queue with lower priority
                // TODO: Implement proper dependency waiting
                queue.push(task);
                break;
            }
        }

        None
    }

    /// Check if task dependencies are satisfied
    fn are_dependencies_satisfied(&self, task: &CookTask) -> bool {
        let invalidation_manager = self.invalidation_manager.lock().unwrap();
        let dependencies = invalidation_manager.graph.get_dependencies(task.op_id);

        let completed = self.completed_tasks.lock().unwrap();
        dependencies.iter().all(|dep| completed.contains_key(dep))
    }

    /// Execute a task
    fn execute_task(&self, mut task: CookTask) -> TaskResult {
        let start_time = Instant::now();

        // Get operation from registry
        let operation = match self.operation_registry.get(&task.operation_type) {
            Some(op) => op,
            None => {
                return TaskResult {
                    task_id: task.op_id,
                    result: Err(format!("Unknown operation type: {}", task.operation_type)),
                    execution_time: start_time.elapsed(),
                    memory_used: 0,
                }
            }
        };

        // Prepare operation context
        let context = match self.prepare_operation_context(&task) {
            Ok(ctx) => ctx,
            Err(e) => {
                return TaskResult {
                    task_id: task.op_id,
                    result: Err(format!("Failed to prepare context: {}", e)),
                    execution_time: start_time.elapsed(),
                    memory_used: 0,
                }
            }
        };

        // Execute operation
        task.status = TaskStatus::Running;
        let result = operation.execute(context);

        TaskResult {
            task_id: task.op_id,
            result: result.map_err(|e| e.to_string()),
            execution_time: start_time.elapsed(),
            memory_used: 0, // TODO: Implement memory tracking
        }
    }

    /// Prepare operation context for execution
    fn prepare_operation_context(
        &self,
        task: &CookTask,
    ) -> Result<OperationContext, SchedulerError> {
        // Get input artifacts from dependencies
        let mut inputs = HashMap::new();
        let dependencies = {
            let invalidation_manager = self.invalidation_manager.lock().unwrap();
            invalidation_manager.graph.get_dependencies(task.op_id)
        };

        for dep_id in dependencies {
            let completed = self.completed_tasks.lock().unwrap();
            if let Some(task_result) = completed.get(&dep_id) {
                match &task_result.result {
                    Ok(artifact) => {
                        inputs.insert(format!("dep_{}", dep_id.data().as_ffi()), artifact.clone());
                    }
                    Err(_) => return Err(SchedulerError::DependencyFailed(dep_id)),
                }
            } else {
                return Err(SchedulerError::DependencyNotReady(dep_id));
            }
        }

        let work_dir = self
            .config
            .work_directory
            .join(format!("task_{}", task.op_id.data().as_ffi()));
        std::fs::create_dir_all(&work_dir)?;

        Ok(OperationContext {
            op_id: task.op_id,
            inputs,
            parameters: task.parameters.clone(),
            work_dir,
            progress_callback: None, // TODO: Implement progress reporting
        })
    }

    /// Start a worker thread
    fn start_worker_thread(&self, worker_id: usize) -> thread::JoinHandle<()> {
        let task_queue = self.task_queue.clone();
        let executing_tasks = self.executing_tasks.clone();
        let completed_tasks = self.completed_tasks.clone();
        let task_receiver = self.task_receiver.clone();
        let state = self.state.clone();
        let scheduler = self as *const CookScheduler;

        thread::spawn(move || {
            loop {
                // Check for shutdown message
                if let Ok(message) = task_receiver.lock().unwrap().try_recv() {
                    match message {
                        SchedulerMessage::Shutdown => break,
                        SchedulerMessage::TaskCompleted(result) => {
                            // Handle task completion
                            let mut completed = completed_tasks.lock().unwrap();
                            completed.insert(result.task_id, result);
                        }
                        SchedulerMessage::TaskFailed(task_id, error) => {
                            // Handle task failure
                            println!("Task {} failed: {}", task_id.data().as_ffi(), error);
                        }
                    }
                }

                // Get next task
                if let Some(task) = unsafe { &*scheduler }.get_next_task() {
                    // Add to executing tasks
                    {
                        let mut executing = executing_tasks.lock().unwrap();
                        executing.insert(
                            task.op_id,
                            TaskHandle {
                                task: task.clone(),
                                started_at: Instant::now(),
                                thread_handle: None,
                            },
                        );
                    }

                    // Execute task
                    let result = unsafe { &*scheduler }.execute_task(task.clone());

                    // Remove from executing and add to completed
                    {
                        let mut executing = executing_tasks.lock().unwrap();
                        executing.remove(&task.op_id);

                        let mut completed = completed_tasks.lock().unwrap();
                        completed.insert(result.task_id, result);
                    }

                    // Update scheduler state
                    {
                        let mut state = state.lock().unwrap();
                        state.total_tasks_executed += 1;
                        state.total_execution_time += Instant::now() - Instant::now();
                        // TODO: Fix timing
                    }
                } else {
                    // No tasks available, wait a bit
                    thread::sleep(Duration::from_millis(100));
                }
            }
        })
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let state = self.state.lock().unwrap();
        let queue = self.task_queue.lock().unwrap();
        let executing = self.executing_tasks.lock().unwrap();
        let completed = self.completed_tasks.lock().unwrap();

        SchedulerStats {
            is_running: state.is_running,
            queued_tasks: queue.len(),
            executing_tasks: executing.len(),
            completed_tasks: completed.len(),
            total_tasks_executed: state.total_tasks_executed,
            total_execution_time: state.total_execution_time,
            current_memory_usage: state.current_memory_usage,
            max_concurrent_tasks: self.config.max_concurrent_tasks,
        }
    }
}

impl Drop for CookScheduler {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub is_running: bool,
    pub queued_tasks: usize,
    pub executing_tasks: usize,
    pub completed_tasks: usize,
    pub total_tasks_executed: usize,
    pub total_execution_time: Duration,
    pub current_memory_usage: usize,
    pub max_concurrent_tasks: usize,
}

/// Scheduler errors
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Scheduler is already running")]
    AlreadyRunning,

    #[error("Scheduler is not running")]
    NotRunning,

    #[error("Task {0:?} is already executing")]
    TaskAlreadyExecuting(OpId),

    #[error("Dependency {0:?} failed")]
    DependencyFailed(OpId),

    #[error("Dependency {0:?} not ready")]
    DependencyNotReady(OpId),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
}
