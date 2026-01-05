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
use serde::{Deserialize, Serialize};

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

/// Parameters for testing operations with custom values
/// This struct accepts any JSON parameters for different operation types
/// 
/// Supported operation types and their parameters:
/// 
/// **merge/combine/combine_active:**
/// - crossfade_ms: f64 (milliseconds for crossfade, default: 100.0)
/// - normalize: bool (normalize output, default: false)
/// - gap_seconds: f64 (gap between tracks, default: 0.0)
/// - output_format: string ("wav", "mp3", "flac", default: "wav")
/// - sample_rate: u32 (Hz, default: 44100)
/// - bit_depth: u32 (bits, default: 16)
/// 
/// **master_pipeline:**
/// - operations: array of strings (pipeline steps, default: ["combine", "normalize", "export"])
/// - parallel_execution: bool (run steps in parallel when possible, default: false)
/// 
/// **normalize:**
/// - target_db: f64 (target level in dB, default: -12.0)
/// - preserve_peaks: bool (preserve peak levels, default: true)
/// 
/// **export:**
/// - format: string (output format, default: "wav")
/// - quality: string ("low", "medium", "high", default: "high")
/// - output_path: string (output directory, default: "./output")
#[derive(Debug, Serialize, Deserialize)]
pub struct TestOperationParams {
    /// Generic parameters map that can hold any operation-specific parameters
    pub parameters: serde_json::Value,
    /// Optional operation type hint for better parameter handling
    pub operation_type: Option<String>,
}

impl Default for TestOperationParams {
    fn default() -> Self {
        Self {
            parameters: serde_json::json!({
                "crossfade_ms": 100.0,
                "normalize": false,
                "gap_seconds": 0.0,
                "output_format": "wav",
                "sample_rate": 44100,
                "bit_depth": 16
            }),
            operation_type: Some("merge".to_string()),
        }
    }
}

/// Test individual operations with custom parameters from the UI
/// 
/// TODO: BACKEND VALIDATION IMPLEMENTATION NEEDED
/// Currently, parameter validation is handled primarily on the frontend side.
/// Future work should implement comprehensive backend validation including:
/// 
/// 1. **Parameter Type Validation:**
///    - Ensure numeric parameters are within valid ranges
///    - Validate string parameters against allowed values (e.g., formats)
///    - Check required vs optional parameters per operation type
/// 
/// 2. **Operation-Specific Validation:**
///    - merge/combine: validate crossfade_ms >= 0, sample_rate > 0, etc.
///    - pipeline: validate operation steps exist and can be chained
///    - normalize: validate target_db is reasonable (e.g., -60 to 0 dB)
///    - export: validate output paths, format compatibility
/// 
/// 3. **Cross-Parameter Validation:**
///    - Ensure parameter combinations are valid
///    - Check for conflicting settings
///    - Validate resource requirements (file paths, memory usage, etc.)
/// 
/// 4. **Schema-Based Validation:**
///    - Define JSON schemas for each operation type
///    - Use a validation library like `jsonschema` or `serde_valid`
///    - Provide detailed error messages for invalid parameters
/// 
/// 5. **Security Validation:**
///    - Sanitize file paths to prevent directory traversal
///    - Validate parameter sizes to prevent DoS attacks
///    - Check permissions for file operations
#[tauri::command]
pub async fn test_operation_with_params(
    operation_name: String,
    params: TestOperationParams,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<String, Error> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!("Testing operation: {} with params: {:?}", operation_name, params)
        );
    }

    // TODO: Implement comprehensive backend validation for different operation types
    // For now, validation is handled on the frontend side, but we need proper backend validation

    // For basic testing, we'll simulate operations with enhanced parameter extraction
    match operation_name.as_str() {
        "combine_active" | "combine" | "merge" => {
            // Create a test merge operation
            let operation = MergeOperation::new();
            
            // Extract and validate merge-specific parameters with improved error handling
            let crossfade_ms = match params.parameters.get("crossfade_ms") {
                Some(v) => v.as_f64().unwrap_or(100.0).max(0.0), // Ensure non-negative
                None => 100.0
            };
            
            let normalize = params.parameters.get("normalize")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
                
            let gap_seconds = match params.parameters.get("gap_seconds") {
                Some(v) => v.as_f64().unwrap_or(0.0).max(0.0), // Ensure non-negative
                None => 0.0
            };
            
            let sample_rate = match params.parameters.get("sample_rate") {
                Some(v) => v.as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(44100)
                    .max(8000)  // Minimum reasonable sample rate
                    .min(192000), // Maximum reasonable sample rate
                None => 44100
            };
            
            let bit_depth = match params.parameters.get("bit_depth") {
                Some(v) => {
                    let depth = v.as_u64().map(|v| v as u32).unwrap_or(16);
                    // Only allow standard bit depths
                    match depth {
                        8 | 16 | 24 | 32 => depth,
                        _ => 16 // Default to 16-bit for invalid values
                    }
                },
                None => 16
            };
            
            let output_format = params.parameters.get("output_format")
                .and_then(|v| v.as_str())
                .filter(|&s| ["wav", "mp3", "flac", "ogg", "m4a"].contains(&s)) // Validate against known formats
                .unwrap_or("wav");

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "Processing merge operation with validated parameters: crossfade={}ms, normalize={}, gap={}s, format={}, rate={}Hz, depth={}bit",
                        crossfade_ms, normalize, gap_seconds, output_format, sample_rate, bit_depth
                    )
                );
            }

            // Create dummy audio artifacts for testing
            let audio1 = AudioArtifact {
                path: std::path::PathBuf::from("test1.wav"),
                format: "wav".to_string(),
                sample_rate,
                channels: 2,
                duration: 5.0,
                metadata: HashMap::new(),
            };
            
            let audio2 = AudioArtifact {
                path: std::path::PathBuf::from("test2.wav"),
                format: "wav".to_string(),
                sample_rate,
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

            // Create operation context
            let mut op_map: slotmap::SlotMap<OpId, ()> = slotmap::SlotMap::new();
            let op_id = op_map.insert(());
            
            // Create a unique work directory for this operation
            let base_artifacts_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));
            let work_dir = base_artifacts_dir.join(format!("test_op_params_{:?}", op_id));
            
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
                parameters: params.parameters.clone(),
                progress_callback: None,
            };

            // Execute the operation
            match operation.execute(context) {
                Ok(result) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "Operation with custom parameters executed successfully"
                        );
                    }
                    
                    // Return a detailed message with the parameters used
                    Ok(format!(
                        "✅ Operation '{}' completed successfully!\n\n📄 Result: {}\n🔧 Operation Type: Merge/Combine\n📊 Input Files: 2 test audio files\n⚙️ Parameters:\n  • Crossfade: {:.1}ms\n  • Normalize: {}\n  • Gap: {:.2}s\n  • Format: {}\n  • Sample Rate: {}Hz\n  • Bit Depth: {}bit\n📁 Work Directory: {}\n\n🔍 Raw Parameters: {}",
                        operation_name,
                        match result {
                            Artifact::Audio(audio) => format!("Audio file: {}", audio.path.display()),
                            _ => "Processed successfully".to_string()
                        },
                        crossfade_ms,
                        normalize,
                        gap_seconds,
                        output_format,
                        sample_rate,
                        bit_depth,
                        work_dir_display,
                        serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string())
                    ))
                }
                Err(e) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("Operation with custom parameters failed: {:?}", e)
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
                    "Processing pipeline operation with custom parameters"
                );
            }

            // Extract and validate pipeline-specific parameters
            let operations = params.parameters.get("operations")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    let steps: Vec<String> = arr.iter()
                        .filter_map(|v| v.as_str())
                        .filter(|&s| ["combine", "normalize", "export", "merge", "compress"].contains(&s)) // Validate steps
                        .map(|s| s.to_string())
                        .collect();
                    
                    if steps.is_empty() {
                        vec!["combine".to_string(), "normalize".to_string(), "export".to_string()]
                    } else {
                        steps
                    }
                })
                .unwrap_or_else(|| vec!["combine".to_string(), "normalize".to_string(), "export".to_string()]);
            
            let parallel_execution = params.parameters.get("parallel_execution")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
                
            let batch_size = params.parameters.get("batch_size")
                .and_then(|v| v.as_u64())
                .map(|v| (v as u32).max(1).min(100)) // Reasonable batch size limits
                .unwrap_or(10);

            let operations_display = operations.join(" → ");

            Ok(format!(
                "✅ Pipeline operation '{}' simulated successfully!\n\n🔗 Pipeline Steps: {}\n⚙️ Parameters:\n  • Parallel Execution: {}\n  • Batch Size: {}\n  • Steps Count: {}\n  • Custom Parameters: {}\n\n⚠️ Note: This is a simulation - actual pipeline execution not yet implemented.",
                operation_name,
                operations_display,
                parallel_execution,
                batch_size,
                operations.len(),
                serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string())
            ))
        }
        "normalize" => {
            // Extract and validate normalize operation parameters
            let target_db = match params.parameters.get("target_db") {
                Some(v) => v.as_f64()
                    .unwrap_or(-12.0)
                    .max(-60.0) // Minimum reasonable target
                    .min(0.0),  // Maximum reasonable target (0dB)
                None => -12.0
            };
            
            let preserve_peaks = params.parameters.get("preserve_peaks")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
                
            let target_lufs = match params.parameters.get("target_lufs") {
                Some(v) => Some(v.as_f64()
                    .unwrap_or(-23.0)
                    .max(-40.0) // Minimum reasonable LUFS
                    .min(-6.0)), // Maximum reasonable LUFS
                None => None
            };
            
            let true_peak_limit = match params.parameters.get("true_peak_limit") {
                Some(v) => Some(v.as_f64()
                    .unwrap_or(-1.0)
                    .max(-6.0) // Minimum reasonable limit
                    .min(0.0)),  // Maximum (0dB)
                None => None
            };

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "Processing normalize operation: target_db={:.1}, preserve_peaks={}, lufs={:?}, peak_limit={:?}",
                        target_db, preserve_peaks, target_lufs, true_peak_limit
                    )
                );
            }

            let mut parameter_lines = vec![
                format!("  • Target dB: {:.1}", target_db),
                format!("  • Preserve Peaks: {}", preserve_peaks),
            ];
            
            if let Some(lufs) = target_lufs {
                parameter_lines.push(format!("  • Target LUFS: {:.1}", lufs));
            }
            
            if let Some(peak_limit) = true_peak_limit {
                parameter_lines.push(format!("  • True Peak Limit: {:.1} dB", peak_limit));
            }

            Ok(format!(
                "✅ Normalize operation '{}' simulated successfully!\n\n🔊 Normalization Settings:\n{}\n⚙️ Raw Parameters: {}\n\n⚠️ Note: This is a simulation - actual normalization not yet implemented.",
                operation_name,
                parameter_lines.join("\n"),
                serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string())
            ))
        }
        "export" => {
            // Extract and validate export operation parameters
            let format = params.parameters.get("format")
                .and_then(|v| v.as_str())
                .filter(|&s| ["wav", "mp3", "flac", "ogg", "m4a", "aac"].contains(&s)) // Validate format
                .unwrap_or("wav");
                
            let quality = params.parameters.get("quality")
                .and_then(|v| v.as_str())
                .filter(|&s| ["low", "medium", "high", "lossless"].contains(&s)) // Validate quality
                .unwrap_or("high");
                
            let output_path = params.parameters.get("output_path")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty()) // Ensure non-empty path
                .unwrap_or("./output");
                
            let bit_rate = match params.parameters.get("bit_rate") {
                Some(v) => Some(v.as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(320)
                    .max(64)    // Minimum reasonable bitrate
                    .min(2048)), // Maximum reasonable bitrate
                None => None
            };
            
            let sample_rate = match params.parameters.get("sample_rate") {
                Some(v) => Some(v.as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(44100)
                    .max(8000)   // Minimum reasonable sample rate
                    .min(192000)), // Maximum reasonable sample rate
                None => None
            };
            
            let normalize_before_export = params.parameters.get("normalize_before_export")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "Processing export operation: format={}, quality={}, path={}, bitrate={:?}, sample_rate={:?}",
                        format, quality, output_path, bit_rate, sample_rate
                    )
                );
            }

            let mut parameter_lines = vec![
                format!("  • Format: {}", format),
                format!("  • Quality: {}", quality),
                format!("  • Output Path: {}", output_path),
                format!("  • Normalize Before Export: {}", normalize_before_export),
            ];
            
            if let Some(br) = bit_rate {
                parameter_lines.push(format!("  • Bit Rate: {} kbps", br));
            }
            
            if let Some(sr) = sample_rate {
                parameter_lines.push(format!("  • Sample Rate: {} Hz", sr));
            }

            Ok(format!(
                "✅ Export operation '{}' simulated successfully!\n\n📤 Export Settings:\n{}\n⚙️ Raw Parameters: {}\n\n⚠️ Note: This is a simulation - actual export not yet implemented.",
                operation_name,
                parameter_lines.join("\n"),
                serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string())
            ))
        }
        _ => {
            // Generic operation handler for unknown types
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!("Processing generic operation '{}' with parameters", operation_name)
                );
            }
            
            // Try to provide helpful information based on parameter structure
            let param_summary = if let Some(obj) = params.parameters.as_object() {
                let keys: Vec<String> = obj.keys().cloned().collect();
                if keys.is_empty() {
                    "No parameters provided".to_string()
                } else {
                    format!("Parameters: {}", keys.join(", "))
                }
            } else {
                "Parameters provided as non-object".to_string()
            };
            
            Ok(format!(
                "⚙️ Generic operation '{}' simulated!\n\n📋 {}\n📝 Raw Parameters:\n{}\n\n💡 Supported operations with specific parameter handling:\n  • combine_active / merge / combine\n  • master_pipeline\n  • normalize\n  • export\n\n🔧 To add support for '{}', update the match statement in test_operation_with_params.\n\n⚠️ Note: This is a generic simulation - no specific operation logic implemented.",
                operation_name,
                param_summary,
                serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string()),
                operation_name
            ))
        }
    }
}
