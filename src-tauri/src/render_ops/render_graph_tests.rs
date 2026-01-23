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
use crate::playback::op_playback::AudioSpec;
use crate::render_ops::{MergeOpRender, OperationContext, RenderOperation, SampleOpRender};
use serde::{Deserialize, Serialize};

/// Test the scheduler by submitting multiple tasks and observing execution
#[tauri::command]
pub async fn test_scheduler(
    scheduler_state: State<'_, Arc<Mutex<CookScheduler>>>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<String, Error> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(logger, LogSystem::Cook, "Starting scheduler test");
    }

    // Create test tasks
    let mut task_results = Vec::new();
    let start_time = std::time::Instant::now();

    // Submit test tasks in a scoped block to ensure lock is released
    {
        let scheduler = match scheduler_state.lock() {
            Ok(s) => s,
            Err(_) => {
                return Err(Error::Io(std::io::Error::other(
                    "Failed to acquire scheduler lock",
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
                parameters: serde_json::json!({}),
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
                return Err(Error::Io(std::io::Error::other(
                    "Failed to acquire scheduler lock for stats",
                )));
            }
        };
        scheduler.get_stats()
    };

    let elapsed = start_time.elapsed();

    if let Ok(logger) = logging_service.lock() {
        log_info!(logger, LogSystem::Cook, "Scheduler test completed");
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

/// Parameters for testing operations with custom values
/// This struct accepts any JSON parameters for different operation types
///
/// Supported operation types and their parameters:
///
/// **merge/combine/combine_active:**
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
    let start_time = std::time::Instant::now();

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!(
                "🚀 Starting test_operation_with_params - Operation: '{}', Type: {:?}",
                operation_name,
                params.operation_type.as_deref().unwrap_or("unknown")
            )
        );
        log_info!(
            logger,
            LogSystem::Combine,
            &format!(
                "📄 Input parameters structure: {}",
                serde_json::to_string_pretty(&params.parameters)
                    .unwrap_or_else(|_| "Invalid JSON".to_string())
            )
        );

        // Log parameter count and types for debugging
        if let Some(obj) = params.parameters.as_object() {
            let param_summary: Vec<String> = obj
                .iter()
                .map(|(key, value)| {
                    let value_type = if value.is_string() {
                        format!("string({})", value.as_str().unwrap().len())
                    } else if value.is_number() {
                        "number".to_string()
                    } else if value.is_boolean() {
                        "boolean".to_string()
                    } else if value.is_array() {
                        format!("array[{}]", value.as_array().unwrap().len())
                    } else if value.is_object() {
                        format!("object({})", value.as_object().unwrap().len())
                    } else {
                        "null".to_string()
                    };
                    format!("{}={}", key, value_type)
                })
                .collect();

            log_info!(
                logger,
                LogSystem::Combine,
                &format!("📊 Parameter analysis: {}", param_summary.join(", "))
            );
        }
    }

    // For now, validation is handled on the frontend side, but we need proper backend validation

    // For basic testing, we'll simulate operations with enhanced parameter extraction
    match operation_name.as_str() {
        "merge" => {
            // Create a test merge operation
            let operation = MergeOpRender::new();

            // Extract and validate merge-specific parameters with improved error handling
            let sample_rate = match params.parameters.get("sample_rate") {
                Some(v) => {
                    let rate = v
                        .as_u64()
                        .map(|v| v as u32)
                        .unwrap_or(44100)
                        .max(8000) // Minimum reasonable sample rate
                        .min(192000); // Maximum reasonable sample rate

                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "🎵 Sample rate parameter: raw={:?} -> validated={}",
                                v, rate
                            )
                        );
                    }
                    rate
                }
                None => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "🎵 Sample rate parameter: missing -> using default 44100"
                        );
                    }
                    44100
                }
            };

            let bit_depth = match params.parameters.get("bit_depth") {
                Some(v) => {
                    let raw_depth = v.as_u64().map(|v| v as u32).unwrap_or(16);
                    // Only allow standard bit depths
                    let validated_depth = match raw_depth {
                        8 | 16 | 24 | 32 => raw_depth,
                        _ => {
                            if let Ok(logger) = logging_service.lock() {
                                log_info!(
                                    logger,
                                    LogSystem::Combine,
                                    &format!(
                                        "⚠️ Invalid bit depth {} -> defaulting to 16",
                                        raw_depth
                                    )
                                );
                            }
                            16 // Default to 16-bit for invalid values
                        }
                    };

                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "🎵 Bit depth parameter: raw={:?} -> validated={}",
                                v, validated_depth
                            )
                        );
                    }
                    validated_depth
                }
                None => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "🎵 Bit depth parameter: missing -> using default 16"
                        );
                    }
                    16
                }
            };

            let output_format = params
                .parameters
                .get("output_format")
                .and_then(|v| v.as_str())
                .filter(|&s| {
                    let valid = ["wav", "mp3", "flac", "ogg", "m4a"].contains(&s);
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "🎵 Output format validation: '{}' -> {}",
                                s,
                                if valid { "valid" } else { "invalid" }
                            )
                        );
                    }
                    valid
                })
                .unwrap_or_else(|| {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "🎵 Output format parameter: missing/invalid -> using default 'wav'"
                        );
                    }
                    "wav"
                });

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "Processing merge operation with validated parameters: format={}, rate={}Hz, depth={}bit",
                        output_format, sample_rate, bit_depth
                    )
                );
            }

            // Create a unique work directory for this operation
            let base_artifacts_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));
            let mut op_map: slotmap::SlotMap<OpId, ()> = slotmap::SlotMap::new();

            // Check if child operations are provided
            let input_artifacts = if let Some(child_ops) = params.parameters.get("child_operations")
            {
                if let Some(ops_array) = child_ops.as_array() {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("Executing {} child operations for merge", ops_array.len())
                        );
                    }

                    let mut artifacts = Vec::new();

                    // Execute each child operation
                    for (idx, child_op_data) in ops_array.iter().enumerate() {
                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!(
                                    "🔄 Processing child operation {}/{}: {}",
                                    idx + 1,
                                    ops_array.len(),
                                    serde_json::to_string_pretty(child_op_data)
                                        .unwrap_or_else(|_| "Invalid JSON".to_string())
                                )
                            );
                        }

                        let op_type = child_op_data
                            .get("type")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                if let Ok(logger) = logging_service.lock() {
                                    log_info!(
                                        logger,
                                        LogSystem::Combine,
                                        &format!("❌ Child operation {} missing 'type' field", idx)
                                    );
                                }
                                Error::Io(std::io::Error::other(format!(
                                    "Child operation {} missing 'type' field",
                                    idx
                                )))
                            })?;

                        let op_params = child_op_data
                            .get("parameters")
                            .cloned()
                            .unwrap_or_else(|| {
                                if let Ok(logger) = logging_service.lock() {
                                    log_info!(
                                        logger,
                                        LogSystem::Combine,
                                        &format!("⚠️ Child operation {} has no parameters, using empty object", idx)
                                    );
                                }
                                serde_json::json!({})
                            });

                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!(
                                    "🎯 Child operation {}: type='{}', params={}",
                                    idx,
                                    op_type,
                                    serde_json::to_string_pretty(&op_params)
                                        .unwrap_or_else(|_| "Invalid JSON".to_string())
                                )
                            );
                        }

                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!("Executing child operation {}: {}", idx + 1, op_type)
                            );
                        }

                        // Execute the child operation based on its type
                        let artifact = execute_child_operation(
                            op_type,
                            op_params,
                            &base_artifacts_dir,
                            &mut op_map,
                            sample_rate,
                        )?;

                        artifacts.push(artifact);
                    }

                    artifacts
                } else {
                    return Err(Error::Io(std::io::Error::other(
                        "child_operations must be an array",
                    )));
                }
            } else {
                // Fallback: Create dummy audio artifacts for testing if no child ops
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        "No child operations provided, using dummy test artifacts"
                    );
                }

                vec![
                    AudioArtifact {
                        path: std::path::PathBuf::from("test1.wav"),
                        format: "wav".to_string(),
                        sample_rate,
                        channels: 2,
                        duration: 5.0,
                        metadata: HashMap::new(),
                    },
                    AudioArtifact {
                        path: std::path::PathBuf::from("test2.wav"),
                        format: "wav".to_string(),
                        sample_rate,
                        channels: 2,
                        duration: 3.0,
                        metadata: HashMap::new(),
                    },
                ]
            };

            // Create inputs for merge operation
            let mut inputs = HashMap::new();
            inputs.insert(
                "inputs".to_string(),
                Artifact::AudioList(input_artifacts.clone()),
            );

            let op_id = op_map.insert(());
            let work_dir = base_artifacts_dir.join(format!("test_op_merge_{:?}", op_id));

            // Ensure the work directory exists
            if let Err(e) = std::fs::create_dir_all(&work_dir) {
                return Err(Error::Io(std::io::Error::other(format!(
                    "Failed to create work directory: {}",
                    e
                ))));
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

            // Execute the merge operation
            match operation.execute(context) {
                Ok(result) => {
                    let elapsed = start_time.elapsed();

                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "✅ Merge operation with child operations executed successfully in {:?}. Processed {} input files.",
                                elapsed, input_artifacts.len()
                            )
                        );
                    }

                    // Return a detailed message with the parameters used
                    Ok(format!(
                        "✅ Operation '{}' completed successfully!\n\n📄 Result: {}\n🔧 Operation Type: Merge/Combine\n📊 Input Files: {} audio files\n⚙️ Parameters:\n  • Format: {}\n  • Sample Rate: {}Hz\n  • Bit Depth: {}bit\n📁 Work Directory: {}\n⏱️ Execution Time: {:?}\n\n🔍 Raw Parameters: {}",
                        operation_name,
                        match result {
                            Artifact::Audio(audio) => format!("Audio file: {}", audio.path.display()),
                            _ => "Processed successfully".to_string()
                        },
                        input_artifacts.len(),
                        output_format,
                        sample_rate,
                        bit_depth,
                        work_dir_display,
                        elapsed,
                        serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string())
                    ))
                }
                Err(e) => {
                    let elapsed = start_time.elapsed();

                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "❌ Merge operation with child operations failed after {:?}: {:?}",
                                elapsed, e
                            )
                        );
                    }
                    Err(Error::Io(std::io::Error::other(format!(
                        "Operation failed after {:?}: {:?}",
                        elapsed, e
                    ))))
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
            let operations = params
                .parameters
                .get("operations")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    let steps: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .filter(|&s| {
                            ["combine", "normalize", "export", "merge", "compress"].contains(&s)
                        }) // Validate steps
                        .map(|s| s.to_string())
                        .collect();

                    if steps.is_empty() {
                        vec![
                            "combine".to_string(),
                            "normalize".to_string(),
                            "export".to_string(),
                        ]
                    } else {
                        steps
                    }
                })
                .unwrap_or_else(|| {
                    vec![
                        "combine".to_string(),
                        "normalize".to_string(),
                        "export".to_string(),
                    ]
                });

            let parallel_execution = params
                .parameters
                .get("parallel_execution")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let batch_size = params
                .parameters
                .get("batch_size")
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
                Some(v) => {
                    let db = v.as_f64().unwrap_or(-12.0).clamp(-60.0, 0.0);
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("🎚️ Target dB parameter: raw={:?} -> validated={:.1}", v, db)
                        );
                    }
                    db
                }
                None => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "🎚️ Target dB parameter: missing -> using default -12.0"
                        );
                    }
                    -12.0
                }
            };

            let preserve_peaks = params
                .parameters
                .get("preserve_peaks")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            "🎚️ Preserve peaks parameter: missing -> using default true"
                        );
                    }
                    true
                });

            let target_lufs = params.parameters.get("target_lufs").map(|v| {
                let lufs = v.as_f64().unwrap_or(-23.0).clamp(-40.0, -6.0);
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "🎚️ Target LUFS parameter: raw={:?} -> validated={:.1}",
                            v, lufs
                        )
                    );
                }
                lufs
            });

            let true_peak_limit = params.parameters.get("true_peak_limit").map(|v| {
                let peak = v.as_f64().unwrap_or(-1.0).clamp(-6.0, 0.0);
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "🎚️ True peak limit parameter: raw={:?} -> validated={:.1}",
                            v, peak
                        )
                    );
                }
                peak
            });

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
            let format = params
                .parameters
                .get("format")
                .and_then(|v| v.as_str())
                .filter(|&s| ["wav", "mp3", "flac", "ogg", "m4a", "aac"].contains(&s)) // Validate format
                .unwrap_or("wav");

            let quality = params
                .parameters
                .get("quality")
                .and_then(|v| v.as_str())
                .filter(|&s| ["low", "medium", "high", "lossless"].contains(&s)) // Validate quality
                .unwrap_or("high");

            let output_path = params
                .parameters
                .get("output_path")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty()) // Ensure non-empty path
                .unwrap_or("./output");

            let bit_rate = params
                .parameters
                .get("bit_rate")
                .map(|v| v.as_u64().map(|v| v as u32).unwrap_or(320).clamp(64, 2048));

            let sample_rate = params.parameters.get("sample_rate").map(|v| {
                v.as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(44100)
                    .clamp(8000, 19200)
            });

            let normalize_before_export = params
                .parameters
                .get("normalize_before_export")
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
                    &format!(
                        "Processing generic operation '{}' with parameters",
                        operation_name
                    )
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

            let elapsed = start_time.elapsed();

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "✅ Generic operation '{}' completed in {:?}. Parameters processed: {}",
                        operation_name, elapsed, param_summary
                    )
                );
            }

            Ok(format!(
                "⚙️ Generic operation '{}' simulated!\n\n📋 {}\n📝 Raw Parameters:\n{}\n\n💡 Supported operations with specific parameter handling:\n  • combine_active / merge / combine\n  • master_pipeline\n  • normalize\n  • export\n\n🔧 To add support for '{}', update the match statement in test_operation_with_params.\n\n⚠️ Note: This is a generic simulation - no specific operation logic implemented.\n\n⏱️ Execution time: {:?}",
                operation_name,
                param_summary,
                serde_json::to_string_pretty(&params.parameters).unwrap_or_else(|_| "Invalid JSON".to_string()),
                operation_name,
                elapsed
            ))
        }
    }
}

/// Helper function to execute a child operation and return its artifact
fn execute_child_operation(
    op_type: &str,
    parameters: serde_json::Value,
    base_artifacts_dir: &std::path::Path,
    op_map: &mut slotmap::SlotMap<OpId, ()>,
    default_sample_rate: u32,
) -> Result<AudioArtifact, Error> {
    match op_type {
        "sample" => {
            // Create a SampleOpRender operation
            let operation = SampleOpRender::new(
                Vec::new(), // Empty samples - will be loaded from file
                AudioSpec {
                    sample_rate: default_sample_rate,
                    channels: 2,
                },
            );

            // Validate parameters
            operation.validate_parameters(&parameters).map_err(|e| {
                Error::Io(std::io::Error::other(format!(
                    "Parameter validation failed for {}: {:?}",
                    op_type, e
                )))
            })?;

            // Create operation context
            let op_id = op_map.insert(());
            let work_dir = base_artifacts_dir.join(format!("child_op_{:?}", op_id));

            if let Err(e) = std::fs::create_dir_all(&work_dir) {
                return Err(Error::Io(std::io::Error::other(format!(
                    "Failed to create work directory for child op: {}",
                    e
                ))));
            }

            let context = OperationContext {
                op_id,
                work_dir,
                inputs: HashMap::new(), // No inputs for load operations
                parameters,
                progress_callback: None,
            };

            // Execute the operation
            let artifact = operation.execute(context).map_err(|e| {
                Error::Io(std::io::Error::other(format!(
                    "Child operation {} failed: {:?}",
                    op_type, e
                )))
            })?;

            // Extract AudioArtifact from result
            match artifact {
                Artifact::Audio(audio) => Ok(audio),
                _ => Err(Error::Io(std::io::Error::other(
                    "Child operation did not return an audio artifact",
                ))),
            }
        }
        // Add more operation types as they are implemented
        _ => Err(Error::Io(std::io::Error::other(format!(
            "Unsupported child operation type: {}. Supported types: sample",
            op_type
        )))),
    }
}
