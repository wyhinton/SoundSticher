use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tauri::State;

use crate::artifacts::{Artifact, AudioArtifact};
use crate::cook::{CookScheduler, CookTask, CookTaskPriority, TaskStatus};
use crate::error::Error;
use crate::graph::OpId;
use crate::log_info;
use crate::logging::{LogSystem, LoggingService};
use crate::ops::{MergeOperation, Operation, OperationContext};

/// Test the scheduler by submitting multiple tasks and observing execution
#[tauri::command]
pub async fn test_scheduler(
    scheduler_state: State<'_, Arc<Mutex<CookScheduler>>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<String, Error> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Cook,
            "Starting scheduler test"
        );
    }

    // Create test tasks
    let mut task_results = Vec::new();
    let start_time = std::time::Instant::now();

    // Submit test tasks in a scoped block to ensure lock is released
    {
        let scheduler = match scheduler_state.lock() {
            Ok(s) => s,
            Err(_) => {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to acquire scheduler lock"
                )));
            }
        };

        // Create a few test tasks
        for i in 0..3 {
            let mut op_map: slotmap::SlotMap<OpId, ()> = slotmap::SlotMap::new();
            let op_id = op_map.insert(());
            
            let task = CookTask {
                op_id,
                operation_type: "merge".to_string(),
                parameters: serde_json::json!({
                    "crossfade_ms": 100.0 * (i + 1) as f64,
                    "normalize": i == 2
                }),
                priority: CookTaskPriority::Normal,
                status: TaskStatus::Pending,
                created_at: SystemTime::now(),
                updated_at: SystemTime::now(),
                dependencies: Vec::new(),
                estimated_duration: Duration::from_millis(500 * (i + 1)),
                estimated_memory: 1024 * 1024 * (i + 1) as usize,
                metadata: HashMap::new(),
                parallelizable: true,
                timeout: None,
            };

            // Submit task
            match scheduler.submit_task(task) {
                Ok(_) => {
                    task_results.push(format!("✅ Task {} submitted successfully", i + 1));
                }
                Err(e) => {
                    task_results.push(format!("❌ Task {} failed to submit: {}", i + 1, e));
                }
            }
        }
    } // Lock is automatically dropped here
    
    // Wait a moment for tasks to potentially execute
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get final stats
    let stats = {
        let scheduler = match scheduler_state.lock() {
            Ok(s) => s,
            Err(_) => {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to acquire scheduler lock for stats"
                )));
            }
        };
        scheduler.get_stats()
    };

    let elapsed = start_time.elapsed();

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Cook,
            "Scheduler test completed"
        );
    }

    let result = format!(
        "🚀 Scheduler Test Results\n\n\
        ⏱️ Test Duration: {:?}\n\
        📊 Scheduler Stats:\n\
        • Running: {}\n\
        • Queued Tasks: {}\n\
        • Executing Tasks: {}\n\
        • Completed Tasks: {}\n\
        • Total Executed: {}\n\
        • Max Concurrent: {}\n\n\
        📝 Task Submission Results:\n{}\n\n\
        💡 Note: Tasks are executed asynchronously in worker threads.\n\
        Check the console logs for detailed scheduler activity.",
        elapsed,
        stats.is_running,
        stats.queued_tasks,
        stats.executing_tasks,
        stats.completed_tasks,
        stats.total_tasks_executed,
        stats.max_concurrent_tasks,
        task_results.join("\n")
    );

    Ok(result)
}

/// Test individual operations by executing them directly
#[tauri::command]
pub async fn test_operation(
    operation_name: String,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<String, Error> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!("Testing operation: {}", operation_name)
        );
    }

    // For basic testing, we'll simulate operations
    match operation_name.as_str() {
        "combine_active" | "combine" | "merge" => {
            // Create a test merge operation
            let operation = MergeOperation::new();
            
            // Create dummy audio artifacts for testing
            let audio1 = AudioArtifact {
                path: std::path::PathBuf::from("test1.wav"),
                format: "wav".to_string(),
                sample_rate: 44100,
                channels: 2,
                duration: 5.0,
                metadata: HashMap::new(),
            };
            
            let audio2 = AudioArtifact {
                path: std::path::PathBuf::from("test2.wav"),
                format: "wav".to_string(),
                sample_rate: 44100,
                channels: 2,
                duration: 3.0,
                metadata: HashMap::new(),
            };

            // Create inputs
            let mut inputs = HashMap::new();
            inputs.insert(
                "inputs".to_string(),
                Artifact::AudioList(vec![audio1, audio2])
            );

            // Create parameters
            let parameters = serde_json::json!({
                "crossfade_ms": 100.0,
                "normalize": false
            });

            // Create operation context
            let mut op_map: slotmap::SlotMap<OpId, ()> = slotmap::SlotMap::new();
            let op_id = op_map.insert(());
            
            // Create a unique work directory for this operation
            let base_artifacts_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));
            let work_dir = base_artifacts_dir.join(format!("test_op_{:?}", op_id));
            
            // Ensure the work directory exists
            if let Err(e) = std::fs::create_dir_all(&work_dir) {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create work directory: {}", e)
                )));
            }
            
            // Store work_dir display string before moving work_dir
            let work_dir_display = work_dir.display().to_string();
            
            let context = OperationContext {
                op_id,
                work_dir,
                inputs,
                parameters,
                progress_callback: None,
            };

            // Execute the operation
            match operation.execute(context) {
                Ok(result) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "Operation executed successfully"
                        );
                    }
                    
                    // Return a more user-friendly message
                    Ok(format!(
                        "✅ Operation '{}' completed successfully!\n\n📄 Result: {}\n🔧 Operation Type: Merge/Combine\n📊 Input Files: 2 test audio files\n⏱️ Estimated Duration: 8.0 seconds\n📁 Work Directory: {}",
                        operation_name,
                        match result {
                            Artifact::Audio(audio) => format!("Audio file: {}", audio.path.display()),
                            _ => "Processed successfully".to_string()
                        },
                        work_dir_display
                    ))
                }
                Err(e) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("Operation failed: {:?}", e)
                        );
                    }
                    Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Operation failed: {:?}", e)
                    )))
                }
            }
        }
        "master_pipeline" => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    "Simulating pipeline operation"
                );
            }
            Ok(format!(
                "✅ Pipeline operation '{}' simulated successfully!\n\n🔗 This would run a sequence of operations:\n  1. combine_active\n  2. normalize\n  3. export\n\n⚠️ Note: This is a simulation - actual pipeline execution not yet implemented.",
                operation_name
            ))
        }
        _ => {
            Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("❌ Unknown operation: {}\n\n💡 Available operations:\n  • combine_active\n  • master_pipeline", operation_name)
            )))
        }
    }
}
