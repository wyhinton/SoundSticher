// Cook scheduler for managing operation execution

use crate::artifacts::{Artifact, ArtifactStorage};
use crate::cook::{CookTask, CookTaskPriority, TaskStatus};
use crate::graph::{InvalidationManager, OpId, OperationNodeManager};
use crate::logging::{LogSystem, LoggingService};
use crate::ops::{OperationContext, OperationRegistry, OperationResult};
use crate::util::id_utils;
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

    /// Logging service
    logger: Arc<LoggingService>,
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
        logger: Arc<LoggingService>,
    ) -> Self {
        let (task_sender, task_receiver) = mpsc::channel();

        logger.info(LogSystem::Cook, "Initializing scheduler", Some("init"));

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
            logger,
        }
    }

    /// Start the scheduler
    pub fn start(&mut self) -> Result<(), SchedulerError> {
        let mut state = self.state.lock().unwrap();
        if state.is_running {
            self.logger.warning(
                LogSystem::Cook,
                "Attempted to start scheduler that is already running",
                Some("start"),
            );
            return Err(SchedulerError::AlreadyRunning);
        }
        state.is_running = true;
        drop(state);

        self.logger.info(
            LogSystem::Cook,
            &format!(
                "Starting scheduler with {} worker threads",
                self.config.max_concurrent_tasks
            ),
            Some("start"),
        );

        // Start worker threads
        for worker_id in 0..self.config.max_concurrent_tasks {
            let worker_handle = self.start_worker_thread(worker_id);
            self.worker_handles.push(worker_handle);
        }

        self.logger.info(
            LogSystem::Cook,
            "Scheduler started successfully",
            Some("start"),
        );

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

        self.logger
            .info(LogSystem::Cook, "Stopping scheduler", Some("stop"));

        // Send shutdown messages
        for _ in 0..self.config.max_concurrent_tasks {
            let _ = self.task_sender.send(SchedulerMessage::Shutdown);
        }

        // Wait for workers to finish
        for handle in self.worker_handles.drain(..) {
            let _ = handle.join();
        }

        self.logger.info(
            LogSystem::Cook,
            "Scheduler stopped successfully",
            Some("stop"),
        );

        Ok(())
    }

    /// Submit a task for execution
    pub fn submit_task(&self, mut task: CookTask) -> Result<(), SchedulerError> {
        let state = self.state.lock().unwrap();
        if !state.is_running {
            self.logger.error(
                LogSystem::Cook,
                "Attempted to submit task to stopped scheduler",
                Some("submit"),
            );
            return Err(SchedulerError::NotRunning);
        }
        drop(state);

        // Check if task is already completed or executing
        {
            let executing = self.executing_tasks.lock().unwrap();
            if executing.contains_key(&task.op_id) {
                self.logger.warning(
                    LogSystem::Cook,
                    &format!(
                        "Task {} already executing",
                        id_utils::friendly_id(task.op_id, "task")
                    ),
                    Some("submit"),
                );
                return Err(SchedulerError::TaskAlreadyExecuting(task.op_id));
            }
        }

        {
            let completed = self.completed_tasks.lock().unwrap();
            if completed.contains_key(&task.op_id) {
                self.logger.debug(
                    LogSystem::Cook,
                    &format!(
                        "Task {} already completed, skipping",
                        id_utils::friendly_id(task.op_id, "task")
                    ),
                    Some("submit"),
                );
                return Ok(());
            }
        }

        // Set task status and add to queue
        task.status = TaskStatus::Queued;
        let mut queue = self.task_queue.lock().unwrap();
        queue.push(task.clone());

        self.logger.info(
            LogSystem::Cook,
            &format!(
                "Submitted task {} (operation: {})",
                id_utils::friendly_id(task.op_id, "task"),
                task.operation_type
            ),
            Some("submit"),
        );

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
        let dependencies = invalidation_manager.get_dependencies(task.op_id);

        if !dependencies.is_empty() {
            self.logger.debug(
                LogSystem::Cook,
                &format!(
                    "Checking {} dependencies for task {}",
                    dependencies.len(),
                    id_utils::friendly_id(task.op_id, "task")
                ),
                Some("dependencies"),
            );
        }

        let completed = self.completed_tasks.lock().unwrap();
        let satisfied = dependencies.iter().all(|dep| {
            let is_completed = completed.contains_key(dep);
            if !is_completed {
                self.logger.debug(
                    LogSystem::Cook,
                    &format!(
                        "Task {} waiting for dependency {}",
                        id_utils::friendly_id(task.op_id, "task"),
                        id_utils::friendly_id(*dep, "dep")
                    ),
                    Some("dependencies"),
                );
            }
            is_completed
        });

        satisfied
    }

    /// Execute a task
    fn execute_task(&self, mut task: CookTask) -> TaskResult {
        let start_time = Instant::now();

        self.logger.info(
            LogSystem::Cook,
            &format!(
                "Starting execution of task {} (operation: {})",
                id_utils::friendly_id(task.op_id, "task"),
                task.operation_type
            ),
            Some("execute"),
        );

        // Get operation from registry
        let operation = match self.operation_registry.get(&task.operation_type) {
            Some(op) => {
                self.logger.debug(
                    LogSystem::Cook,
                    &format!("Found operation '{}' in registry", task.operation_type),
                    Some("execute"),
                );
                op
            }
            None => {
                let error_msg = format!("Unknown operation type: {}", task.operation_type);
                self.logger
                    .error(LogSystem::Cook, &error_msg, Some("execute"));
                return TaskResult {
                    task_id: task.op_id,
                    result: Err(error_msg),
                    execution_time: start_time.elapsed(),
                    memory_used: 0,
                };
            }
        };

        // Prepare operation context
        let context = match self.prepare_operation_context(&task) {
            Ok(ctx) => {
                self.logger.debug(
                    LogSystem::Cook,
                    &format!(
                        "Prepared context for task {} with {} inputs",
                        id_utils::friendly_id(task.op_id, "task"),
                        ctx.inputs.len()
                    ),
                    Some("execute"),
                );
                ctx
            }
            Err(e) => {
                let error_msg = format!("Failed to prepare context: {}", e);
                self.logger
                    .error(LogSystem::Cook, &error_msg, Some("execute"));
                return TaskResult {
                    task_id: task.op_id,
                    result: Err(error_msg),
                    execution_time: start_time.elapsed(),
                    memory_used: 0,
                };
            }
        };

        // Execute operation
        task.status = TaskStatus::Running;
        self.logger.info(
            LogSystem::Cook,
            &format!(
                "Executing operation for task {}",
                id_utils::friendly_id(task.op_id, "task")
            ),
            Some("execute"),
        );

        let result = operation.execute(context);
        let execution_time = start_time.elapsed();

        match &result {
            Ok(_) => {
                self.logger.info(
                    LogSystem::Cook,
                    &format!(
                        "Task {} completed successfully in {:?}",
                        id_utils::friendly_id(task.op_id, "task"),
                        execution_time
                    ),
                    Some("execute"),
                );
            }
            Err(e) => {
                self.logger.error(
                    LogSystem::Cook,
                    &format!(
                        "Task {} failed after {:?}: {}",
                        id_utils::friendly_id(task.op_id, "task"),
                        execution_time,
                        e
                    ),
                    Some("execute"),
                );
            }
        }

        TaskResult {
            task_id: task.op_id,
            result: result.map_err(|e| e.to_string()),
            execution_time,
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
            invalidation_manager.get_dependencies(task.op_id)
        };

        for dep_id in dependencies {
            let completed = self.completed_tasks.lock().unwrap();
            if let Some(task_result) = completed.get(&dep_id) {
                match &task_result.result {
                    Ok(artifact) => {
                        inputs.insert(
                            format!("dep_{}", id_utils::friendly_id(dep_id, "dep")),
                            artifact.clone(),
                        );
                    }
                    Err(_) => return Err(SchedulerError::DependencyFailed(dep_id)),
                }
            } else {
                return Err(SchedulerError::DependencyNotReady(dep_id));
            }
        }

        let work_dir = self.config.work_directory.join(format!(
            "task_{}",
            id_utils::friendly_id(task.op_id, "task")
        ));
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
        let operation_registry = self.operation_registry.clone();
        let invalidation_manager = self.invalidation_manager.clone();
        let config = self.config.clone();
        let logger = self.logger.clone();

        logger.debug(
            LogSystem::Cook,
            &format!("Starting worker thread {}", worker_id),
            Some("worker"),
        );

        thread::spawn(move || {
            logger.info(
                LogSystem::Cook,
                &format!("Worker thread {} started", worker_id),
                Some("worker"),
            );

            loop {
                // Check for shutdown message
                if let Ok(message) = task_receiver.lock().unwrap().try_recv() {
                    match message {
                        SchedulerMessage::Shutdown => {
                            logger.info(
                                LogSystem::Cook,
                                &format!("Worker thread {} received shutdown signal", worker_id),
                                Some("worker"),
                            );
                            break;
                        }
                        SchedulerMessage::TaskCompleted(result) => {
                            logger.debug(
                                LogSystem::Cook,
                                &format!(
                                    "Worker {} handling task completion for {}",
                                    worker_id,
                                    id_utils::friendly_id(result.task_id, "task")
                                ),
                                Some("worker"),
                            );
                            // Handle task completion
                            let mut completed = completed_tasks.lock().unwrap();
                            completed.insert(result.task_id, result);
                        }
                        SchedulerMessage::TaskFailed(task_id, error) => {
                            logger.error(
                                LogSystem::Cook,
                                &format!(
                                    "Task {} failed: {}",
                                    id_utils::friendly_id(task_id, "task"),
                                    error
                                ),
                                Some("worker"),
                            );
                        }
                    }
                }

                // Get next task
                if let Some(task) = CookScheduler::get_next_task_static(
                    &task_queue,
                    &invalidation_manager,
                    &completed_tasks,
                ) {
                    logger.info(
                        LogSystem::Cook,
                        &format!(
                            "Worker {} picked up task {} (operation: {})",
                            worker_id,
                            id_utils::friendly_id(task.op_id, "task"),
                            task.operation_type
                        ),
                        Some("worker"),
                    );

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
                    let result = CookScheduler::execute_task_static(
                        task.clone(),
                        &operation_registry,
                        &invalidation_manager,
                        &completed_tasks,
                        &config,
                        &logger,
                    );

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

            logger.info(
                LogSystem::Cook,
                &format!("Worker thread {} stopped", worker_id),
                Some("worker"),
            );
        })
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let state = self.state.lock().unwrap();
        let queue = self.task_queue.lock().unwrap();
        let executing = self.executing_tasks.lock().unwrap();
        let completed = self.completed_tasks.lock().unwrap();

        let stats = SchedulerStats {
            is_running: state.is_running,
            queued_tasks: queue.len(),
            executing_tasks: executing.len(),
            completed_tasks: completed.len(),
            total_tasks_executed: state.total_tasks_executed,
            total_execution_time: state.total_execution_time,
            current_memory_usage: state.current_memory_usage,
            max_concurrent_tasks: self.config.max_concurrent_tasks,
        };

        self.logger.debug(
            LogSystem::Cook,
            &format!(
                "Scheduler stats: {} queued, {} executing, {} completed",
                stats.queued_tasks, stats.executing_tasks, stats.completed_tasks
            ),
            Some("stats"),
        );

        stats
    }

    /// Static version of get_next_task for use in worker threads
    fn get_next_task_static(
        task_queue: &Arc<Mutex<BinaryHeap<CookTask>>>,
        invalidation_manager: &Arc<Mutex<InvalidationManager>>,
        completed_tasks: &Arc<Mutex<HashMap<OpId, TaskResult>>>,
    ) -> Option<CookTask> {
        let mut queue = task_queue.lock().unwrap();

        while let Some(task) = queue.pop() {
            // Check if all dependencies are satisfied
            if CookScheduler::are_dependencies_satisfied_static(
                &task,
                invalidation_manager,
                completed_tasks,
            ) {
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

    /// Static version of are_dependencies_satisfied for use in worker threads
    fn are_dependencies_satisfied_static(
        task: &CookTask,
        invalidation_manager: &Arc<Mutex<InvalidationManager>>,
        completed_tasks: &Arc<Mutex<HashMap<OpId, TaskResult>>>,
    ) -> bool {
        let invalidation_manager = invalidation_manager.lock().unwrap();
        let dependencies = invalidation_manager.get_dependencies(task.op_id);

        let completed = completed_tasks.lock().unwrap();
        dependencies.iter().all(|dep| completed.contains_key(dep))
    }

    /// Static version of execute_task for use in worker threads
    fn execute_task_static(
        mut task: CookTask,
        operation_registry: &Arc<OperationRegistry>,
        invalidation_manager: &Arc<Mutex<InvalidationManager>>,
        completed_tasks: &Arc<Mutex<HashMap<OpId, TaskResult>>>,
        config: &SchedulerConfig,
        logger: &Arc<LoggingService>,
    ) -> TaskResult {
        let start_time = Instant::now();

        logger.info(
            LogSystem::Cook,
            &format!(
                "Starting static execution of task {} (operation: {})",
                id_utils::friendly_id(task.op_id, "task"),
                task.operation_type
            ),
            Some("execute_static"),
        );

        // Get operation from registry
        let operation = match operation_registry.get(&task.operation_type) {
            Some(op) => {
                logger.debug(
                    LogSystem::Cook,
                    &format!("Found operation '{}' in registry", task.operation_type),
                    Some("execute_static"),
                );
                op
            }
            None => {
                let error_msg = format!("Unknown operation type: {}", task.operation_type);
                logger.error(LogSystem::Cook, &error_msg, Some("execute_static"));
                return TaskResult {
                    task_id: task.op_id,
                    result: Err(error_msg),
                    execution_time: start_time.elapsed(),
                    memory_used: 0,
                };
            }
        };

        // Prepare operation context
        let context = match CookScheduler::prepare_operation_context_static(
            &task,
            invalidation_manager,
            completed_tasks,
            config,
        ) {
            Ok(ctx) => {
                logger.debug(
                    LogSystem::Cook,
                    &format!(
                        "Prepared context for task {} with {} inputs",
                        id_utils::friendly_id(task.op_id, "task"),
                        ctx.inputs.len()
                    ),
                    Some("execute_static"),
                );
                ctx
            }
            Err(e) => {
                let error_msg = format!("Failed to prepare context: {}", e);
                logger.error(LogSystem::Cook, &error_msg, Some("execute_static"));
                return TaskResult {
                    task_id: task.op_id,
                    result: Err(error_msg),
                    execution_time: start_time.elapsed(),
                    memory_used: 0,
                };
            }
        };

        // Execute operation
        task.status = TaskStatus::Running;
        logger.info(
            LogSystem::Cook,
            &format!(
                "Executing operation for task {}",
                id_utils::friendly_id(task.op_id, "task")
            ),
            Some("execute_static"),
        );

        let result = operation.execute(context);
        let execution_time = start_time.elapsed();

        match &result {
            Ok(_) => {
                logger.info(
                    LogSystem::Cook,
                    &format!(
                        "Task {} completed successfully in {:?}",
                        id_utils::friendly_id(task.op_id, "task"),
                        execution_time
                    ),
                    Some("execute_static"),
                );
            }
            Err(e) => {
                logger.error(
                    LogSystem::Cook,
                    &format!(
                        "Task {} failed after {:?}: {}",
                        id_utils::friendly_id(task.op_id, "task"),
                        execution_time,
                        e
                    ),
                    Some("execute_static"),
                );
            }
        }

        TaskResult {
            task_id: task.op_id,
            result: result.map_err(|e| e.to_string()),
            execution_time,
            memory_used: 0, // TODO: Implement memory tracking
        }
    }

    /// Static version of prepare_operation_context for use in worker threads
    fn prepare_operation_context_static(
        task: &CookTask,
        invalidation_manager: &Arc<Mutex<InvalidationManager>>,
        completed_tasks: &Arc<Mutex<HashMap<OpId, TaskResult>>>,
        config: &SchedulerConfig,
    ) -> Result<OperationContext, SchedulerError> {
        // Get input artifacts from dependencies
        let mut inputs = HashMap::new();
        let dependencies = {
            let invalidation_manager = invalidation_manager.lock().unwrap();
            invalidation_manager.get_dependencies(task.op_id)
        };

        for dep_id in dependencies {
            let completed = completed_tasks.lock().unwrap();
            if let Some(task_result) = completed.get(&dep_id) {
                match &task_result.result {
                    Ok(artifact) => {
                        inputs.insert(
                            format!("dep_{}", id_utils::friendly_id(dep_id, "dep")),
                            artifact.clone(),
                        );
                    }
                    Err(_) => return Err(SchedulerError::DependencyFailed(dep_id)),
                }
            } else {
                return Err(SchedulerError::DependencyNotReady(dep_id));
            }
        }

        let work_dir = config.work_directory.join(format!(
            "task_{}",
            id_utils::friendly_id(task.op_id, "task")
        ));
        std::fs::create_dir_all(&work_dir)?;

        Ok(OperationContext {
            op_id: task.op_id,
            inputs,
            parameters: task.parameters.clone(),
            work_dir,
            progress_callback: None, // TODO: Implement progress reporting
        })
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
