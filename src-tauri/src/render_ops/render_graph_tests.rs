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
use crate::render_ops::generated_operation_defs::{FrontendOperationsState, FrontendOperationDef};
use crate::render_ops::{MergeOpRender, OperationContext, RenderOperation, SampleOpRender};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

// ============================================================================
// FRONTEND OPERATION TYPES (matching src/lib/state/operation.ts)
// ============================================================================

/// Unique identifier for operations (matches TypeScript OperationId)
pub type OperationId = String;

/// Render policy for operations
#[derive(Debug, Clone, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RenderPolicy {
    Auto,
    Manual,
    Frozen,
}


/// Source types for operations (matches TypeScript OperationSource)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OperationSource {
    #[serde(rename = "group")]
    Group {
        #[serde(rename = "groupRef")]
        group_ref: String,
    },
    #[serde(rename = "file")]
    File {
        #[serde(rename = "fileId")]
        file_id: String,
    },
    #[serde(rename = "files")]
    Files {
        #[serde(rename = "fileIds")]
        file_ids: Vec<String>,
    },
    #[serde(rename = "all")]
    All,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "section")]
    Section {
        #[serde(rename = "sectionIndex")]
        section_index: i32,
    },
    #[serde(rename = "operation")]
    Operation {
        #[serde(rename = "operationId")]
        operation_id: OperationId,
    },
    #[serde(rename = "previousOperation")]
    PreviousOperation {
        #[serde(rename = "operationId")]
        operation_id: OperationId,
    },
}


// ============================================================================
// TEST SCHEDULER
// ============================================================================

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
pub async fn test_render_single_operation(
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
                "🚀 Starting test_render_single_operation - Operation: '{}', Type: {:?}",
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

    // Extract operations context from parameters (sent from frontend)
    let operations_context: Option<FrontendOperationsState> =
        params.parameters.get("__operations_context").and_then(|v| {
            match serde_json::from_value::<FrontendOperationsState>(v.clone()) {
                Ok(ctx) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "📦 Operations context loaded: {} operations, order: {:?}",
                                ctx.defs.len(),
                                ctx.order
                            )
                        );
                    }
                    Some(ctx)
                }
                Err(e) => {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("⚠️ Failed to parse operations context: {:?}", e)
                        );
                    }
                    None
                }
            }
        });

    let target_operation_id: Option<String> = params
        .parameters
        .get("__target_operation_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!(
                "🎯 Target operation ID: {:?}, Has context: {}",
                target_operation_id,
                operations_context.is_some()
            )
        );
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

            // Resolve input artifacts from operations context
            let input_artifacts = if let (Some(ctx), Some(target_id)) =
                (&operations_context, &target_operation_id)
            {
                // Get the target operation from the context
                if let Some(target_op) = ctx.defs.get(target_id) {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "🎯 Found target operation '{}' (kind: {}) in context",
                                target_op.name(),
                                target_op.kind()
                            )
                        );
                    }

                    // Resolve sources from the operation
                    let sources = target_op.sources();
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("📋 Operation has {} sources to resolve", sources.len())
                        );
                    }

                    let mut artifacts = Vec::new();
                    for (idx, source) in sources.iter().enumerate() {
                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!(
                                    "🔄 Resolving source {}/{}: {:?}",
                                    idx + 1,
                                    sources.len(),
                                    source
                                )
                            );
                        }

                        // Resolve source to artifact(s)
                        match resolve_operation_source(
                            source,
                            ctx,
                            &base_artifacts_dir,
                            &mut op_map,
                            sample_rate,
                            &logging_service,
                        ) {
                            Ok(resolved_artifacts) => {
                                if let Ok(logger) = logging_service.lock() {
                                    log_info!(
                                        logger,
                                        LogSystem::Combine,
                                        &format!(
                                            "✅ Source {} resolved to {} artifact(s)",
                                            idx + 1,
                                            resolved_artifacts.len()
                                        )
                                    );
                                }
                                artifacts.extend(resolved_artifacts);
                            }
                            Err(e) => {
                                if let Ok(logger) = logging_service.lock() {
                                    log_info!(
                                        logger,
                                        LogSystem::Combine,
                                        &format!(
                                            "⚠️ Failed to resolve source {}: {:?}",
                                            idx + 1,
                                            e
                                        )
                                    );
                                }
                                // Continue with other sources
                            }
                        }
                    }

                    if artifacts.is_empty() {
                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                "⚠️ No artifacts resolved from sources, using dummy test artifacts"
                            );
                        }
                        // Fall back to dummy artifacts
                        vec![AudioArtifact::new(
                            std::path::PathBuf::from("test1.wav"),
                            "wav".to_string(),
                            sample_rate,
                            2,
                            5.0,
                        )]
                    } else {
                        artifacts
                    }
                } else {
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("⚠️ Target operation '{}' not found in context", target_id)
                        );
                    }
                    // Fall back to dummy artifacts
                    vec![AudioArtifact::new(
                        std::path::PathBuf::from("test1.wav"),
                        "wav".to_string(),
                        sample_rate,
                        2,
                        5.0,
                    )]
                }
            } else if let Some(child_ops) = params.parameters.get("child_operations") {
                // Legacy: handle child_operations array if provided
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
                    AudioArtifact::new(
                        std::path::PathBuf::from("test1.wav"),
                        "wav".to_string(),
                        sample_rate,
                        2,
                        5.0,
                    ),
                    AudioArtifact::new(
                        std::path::PathBuf::from("test2.wav"),
                        "wav".to_string(),
                        sample_rate,
                        2,
                        3.0,
                    ),
                ]
            };

            // 🔍 DETAILED INPUT ARTIFACTS DEBUGGING
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔍 DETAILED ARTIFACT DEBUGGING: Total {} input artifacts resolved",
                        input_artifacts.len()
                    )
                );
                
                for (idx, artifact) in input_artifacts.iter().enumerate() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "🎵 Artifact {}: path='{}', format='{}', sample_rate={}Hz, channels={}, duration={}s, metadata_keys=[{}]",
                            idx + 1,
                            artifact.path.display(),
                            artifact.format,
                            artifact.sample_rate,
                            artifact.channels,
                            artifact.duration,
                            artifact.metadata.keys().cloned().collect::<Vec<String>>().join(", ")
                        )
                    );
                    
                    // Check if file actually exists and get its size
                    if artifact.path.exists() {
                        if let Ok(metadata) = std::fs::metadata(&artifact.path) {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!(
                                    "📁 File {} exists: size={}KB",
                                    idx + 1,
                                    metadata.len() / 1024
                                )
                            );
                        } else {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!("⚠️ File {} exists but metadata read failed", idx + 1)
                            );
                        }
                    } else {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!("❌ File {} DOES NOT EXIST on disk: {}", idx + 1, artifact.path.display())
                        );
                    }
                }

                // Log what's being passed to the merge operation
                let total_expected_duration: f64 = input_artifacts.iter().map(|a| a.duration).sum();
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "📊 Input summary: {} files, total expected duration: {}s",
                        input_artifacts.len(),
                        total_expected_duration
                    )
                );
            }

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

            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 About to execute merge operation with work_dir: {}, op_id: {:?}",
                        work_dir_display, op_id
                    )
                );
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 Operation context inputs contains: {} entries",
                        inputs.len()
                    )
                );
                if let Some(Artifact::AudioList(list)) = inputs.get("inputs") {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "🔧 AudioList in inputs contains {} audio artifacts",
                            list.len()
                        )
                    );
                }
            }

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
                                "✅ Merge operation executed successfully in {:?}. Processed {} input files.",
                                elapsed, input_artifacts.len()
                            )
                        );

                        // 🔍 DETAILED RESULT DEBUGGING
                        match &result {
                            Artifact::Audio(audio) => {
                                log_info!(
                                    logger,
                                    LogSystem::Combine,
                                    &format!(
                                        "🎵 Result audio artifact: path='{}', format='{}', sample_rate={}Hz, channels={}, duration={}s",
                                        audio.path.display(),
                                        audio.format,
                                        audio.sample_rate,
                                        audio.channels,
                                        audio.duration
                                    )
                                );

                                // Check the actual output file size
                                if audio.path.exists() {
                                    if let Ok(metadata) = std::fs::metadata(&audio.path) {
                                        log_info!(
                                            logger,
                                            LogSystem::Combine,
                                            &format!(
                                                "📁 Output file created: size={}KB ({}bytes)",
                                                metadata.len() / 1024,
                                                metadata.len()
                                            )
                                        );
                                        
                                        if metadata.len() < 1024 {
                                            log_info!(
                                                logger,
                                                LogSystem::Combine,
                                                "⚠️ WARNING: Output file is very small (< 1KB)! This indicates a problem with audio data."
                                            );
                                        }
                                    } else {
                                        log_info!(
                                            logger,
                                            LogSystem::Combine,
                                            "❌ Output file exists but metadata read failed"
                                        );
                                    }
                                } else {
                                    log_info!(
                                        logger,
                                        LogSystem::Combine,
                                        &format!("❌ Output file DOES NOT EXIST: {}", audio.path.display())
                                    );
                                }

                                // Log metadata from result
                                if !audio.metadata.is_empty() {
                                    log_info!(
                                        logger,
                                        LogSystem::Combine,
                                        &format!(
                                            "📊 Result metadata: {}",
                                            audio.metadata.iter()
                                                .map(|(k, v)| format!("{}={}", k, v))
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        )
                                    );
                                }
                            }
                            Artifact::AudioList(list) => {
                                log_info!(
                                    logger,
                                    LogSystem::Combine,
                                    &format!("🎵 Result is AudioList with {} items", list.len())
                                );
                            }
                            _ => {
                                log_info!(
                                    logger,
                                    LogSystem::Combine,
                                    "🎵 Result is not an audio artifact type"
                                );
                            }
                        }
                    }

                    // Return a detailed message with the parameters used
                    Ok(format!(
                        "✅ Operation '{}' completed successfully!\n\n📄 Result: {}\n🔧 Operation Type: Merge/Combine\n📊 Input Files: {} audio files\n⚙️ Parameters:\n  • Format: {}\n  • Sample Rate: {}Hz\n  • Bit Depth: {}bit\n📁 Work Directory: {}\n⏱️ Execution Time: {:?}\n\n🔍 Raw Parameters: {}",
                        operation_name,
                        match result {
                            Artifact::Audio(audio) => {
                                let size_info = if audio.path.exists() {
                                    match std::fs::metadata(&audio.path) {
                                        Ok(metadata) => format!(" ({}KB)", metadata.len() / 1024),
                                        Err(_) => " (size unknown)".to_string(),
                                    }
                                } else {
                                    " (FILE NOT FOUND)".to_string()
                                };
                                format!("Audio file: {}{}", audio.path.display(), size_info)
                            },
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
        },
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

            let bit_rate  = params
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
                "⚙️ Generic operation '{}' simulated!\n\n📋 {}\n📝 Raw Parameters:\n{}\n\n💡 Supported operations with specific parameter handling:\n  • combine_active / merge / combine\n  • master_pipeline\n  • normalize\n  • export\n\n🔧 To add support for '{}', update the match statement in test_render_single_operation.\n\n⚠️ Note: This is a generic simulation - no specific operation logic implemented.\n\n⏱️ Execution time: {:?}",
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

/// Resolve an OperationSource to a list of AudioArtifacts
///
/// This function handles the different source types that can be specified
/// in an operation definition, recursively resolving dependencies as needed.
fn resolve_operation_source(
    source: &OperationSource,
    ctx: &FrontendOperationsState,
    base_artifacts_dir: &std::path::Path,
    op_map: &mut slotmap::SlotMap<OpId, ()>,
    sample_rate: u32,
    logging_service: &State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<Vec<AudioArtifact>, Error> {
    match source {
        OperationSource::File { file_id } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!("📁 Resolving single file source: {}", file_id)
                );
            }
            // For now, create a placeholder artifact with reasonable duration for testing
            // TODO: Resolve actual file path from timeline state
            let placeholder_duration = 3.0; // 3 seconds for testing
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 Creating placeholder audio artifact for file '{}' with duration={}s",
                        file_id, placeholder_duration
                    )
                );
            }
            
            Ok(vec![AudioArtifact {
                path: std::path::PathBuf::from(file_id),
                format: "wav".to_string(),
                sample_rate,
                channels: 2,
                duration: placeholder_duration,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source_type".to_string(), "file".to_string());
                    meta.insert("file_id".to_string(), file_id.clone());
                    meta.insert("is_placeholder".to_string(), "true".to_string());
                    meta
                },
                data: None,
            }])
        }
        OperationSource::Files { file_ids } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!("📁 Resolving {} file sources", file_ids.len())
                );
            }
            // Create placeholder artifacts for each file with reasonable durations
            let artifacts: Vec<AudioArtifact> = file_ids
                .iter()
                .enumerate()
                .map(|(idx, file_id)| {
                    // Vary the duration for testing (2-5 seconds)
                    let placeholder_duration = 2.0 + (idx as f64 * 0.5);
                    if let Ok(logger) = logging_service.lock() {
                        log_info!(
                            logger,
                            LogSystem::Combine,
                            &format!(
                                "🔧 Creating placeholder audio artifact {}/{} for file '{}' with duration={}s",
                                idx + 1, file_ids.len(), file_id, placeholder_duration
                            )
                        );
                    }
                    
                    AudioArtifact {
                        path: std::path::PathBuf::from(file_id),
                        format: "wav".to_string(),
                        sample_rate,
                        channels: 2,
                        duration: placeholder_duration,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("source_type".to_string(), "files".to_string());
                            meta.insert("file_id".to_string(), file_id.clone());
                            meta.insert("is_placeholder".to_string(), "true".to_string());
                            meta.insert("placeholder_index".to_string(), idx.to_string());
                            meta
                        },
                        data: None,
                    }
                })
                .collect();
                
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "📊 Created {} placeholder artifacts with total duration: {}s",
                        artifacts.len(),
                        artifacts.iter().map(|a| a.duration).sum::<f64>()
                    )
                );
            }
            
            Ok(artifacts)
        }
        OperationSource::Group { group_ref } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!("👥 Resolving group source: {}", group_ref)
                );
            }
            // TODO: Resolve actual group files from groups state
            let placeholder_duration = 4.0; // 4 seconds for testing
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 Creating placeholder group artifact for '{}' with duration={}s",
                        group_ref, placeholder_duration
                    )
                );
            }
            
            Ok(vec![AudioArtifact {
                path: std::path::PathBuf::from(format!("group_{}.wav", group_ref)),
                format: "wav".to_string(),
                sample_rate,
                channels: 2,
                duration: placeholder_duration,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source_type".to_string(), "group".to_string());
                    meta.insert("group_ref".to_string(), group_ref.clone());
                    meta.insert("is_placeholder".to_string(), "true".to_string());
                    meta
                },
                data: None,
            }])
        }
        OperationSource::Operation { operation_id } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!("🔗 Resolving operation dependency: {}", operation_id)
                );
            }
            // Recursively resolve the dependent operation
            if let Some(dep_op) = ctx.defs.get(operation_id) {
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "🔄 Recursively resolving operation '{}' (kind: {})",
                            dep_op.name(),
                            dep_op.kind()
                        )
                    );
                }
                // Get sources from the dependent operation
                let dep_sources = dep_op.sources();
                let mut all_artifacts = Vec::new();
                for dep_source in dep_sources {
                    let artifacts = resolve_operation_source(
                        &dep_source,
                        ctx,
                        base_artifacts_dir,
                        op_map,
                        sample_rate,
                        logging_service,
                    )?;
                    all_artifacts.extend(artifacts);
                }
                Ok(all_artifacts)
            } else {
                Err(Error::Io(std::io::Error::other(format!(
                    "Dependent operation '{}' not found in context",
                    operation_id
                ))))
            }
        }
        OperationSource::PreviousOperation { operation_id } => {
            // Same as Operation source
            resolve_operation_source(
                &OperationSource::Operation {
                    operation_id: operation_id.clone(),
                },
                ctx,
                base_artifacts_dir,
                op_map,
                sample_rate,
                logging_service,
            )
        }
        OperationSource::All => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    "📚 Resolving 'all' source - TODO: needs timeline state"
                );
            }
            // TODO: Resolve all files from timeline state
            let placeholder_duration = 6.0; // 6 seconds for testing
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 Creating placeholder 'all' artifact with duration={}s",
                        placeholder_duration
                    )
                );
            }
            
            Ok(vec![AudioArtifact {
                path: std::path::PathBuf::from("all_files_placeholder.wav"),
                format: "wav".to_string(),
                sample_rate,
                channels: 2,
                duration: placeholder_duration,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source_type".to_string(), "all".to_string());
                    meta.insert("is_placeholder".to_string(), "true".to_string());
                    meta
                },
                data: None,
            }])
        }
        OperationSource::Active => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    "🎯 Resolving 'active' source - TODO: needs timeline state"
                );
            }
            // TODO: Resolve active files from timeline state
            let placeholder_duration = 5.0; // 5 seconds for testing
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 Creating placeholder 'active' artifact with duration={}s",
                        placeholder_duration
                    )
                );
            }
            
            Ok(vec![AudioArtifact {
                path: std::path::PathBuf::from("active_files_placeholder.wav"),
                format: "wav".to_string(),
                sample_rate,
                channels: 2,
                duration: placeholder_duration,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source_type".to_string(), "active".to_string());
                    meta.insert("is_placeholder".to_string(), "true".to_string());
                    meta
                },
                data: None,
            }])
        }
        OperationSource::Section { section_index } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "📍 Resolving section source: {} - TODO: needs timeline state",
                        section_index
                    )
                );
            }
            // TODO: Resolve section files from timeline state
            let placeholder_duration = 3.5; // 3.5 seconds for testing
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔧 Creating placeholder section artifact for section {} with duration={}s",
                        section_index, placeholder_duration
                    )
                );
            }
            
            Ok(vec![AudioArtifact {
                path: std::path::PathBuf::from(format!("section_{}.wav", section_index)),
                format: "wav".to_string(),
                sample_rate,
                channels: 2,
                duration: placeholder_duration,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("source_type".to_string(), "section".to_string());
                    meta.insert("section_index".to_string(), section_index.to_string());
                    meta.insert("is_placeholder".to_string(), "true".to_string());
                    meta
                },
                data: None,
            }])
        }
    }
}

// ============================================================================
// AUTO-RENDER ALL OPERATIONS
// ============================================================================

/// Result for a single operation render
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationRenderResult {
    /// The operation ID
    pub operation_id: OperationId,
    /// The operation name
    pub operation_name: String,
    /// Whether the render was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Duration of the render in milliseconds
    pub duration_ms: u64,
}

/// Result for the batch render operation
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRenderResult {
    /// Total number of operations processed
    pub total_operations: usize,
    /// Number of successful renders
    pub successful_renders: usize,
    /// Number of failed renders
    pub failed_renders: usize,
    /// Number of skipped operations (non-auto policy)
    pub skipped_operations: usize,
    /// Individual results for each operation
    pub results: Vec<OperationRenderResult>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// The revision number this render was triggered by
    pub triggered_by_rev: i64,
}

/// Parameters for the batch render operation
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRenderParams {
    /// The current operations state from frontend
    pub operations_state: FrontendOperationsState,
    /// The current revision number
    pub current_rev: i64,
    /// Optional list of specific operation IDs to render (if empty, renders all auto ops)
    pub specific_operation_ids: Option<Vec<OperationId>>,
    /// Whether to force render even if policy is not 'auto'
    pub force_render: Option<bool>,
}

/// Render all operations with renderPolicy: 'auto'
/// 
/// This command is designed to be called from the frontend when `_rev` changes.
/// It processes all operations in dependency order, respecting the render policy:
/// - 'auto': Will be re-rendered
/// - 'manual': Will be skipped unless force_render is true
/// - 'frozen': Will be skipped (output is treated as immutable)
/// 
/// The command returns detailed results for each operation, including timing
/// information useful for debugging and performance monitoring.
#[tauri::command]
pub async fn render_all_auto_operations(
    params: BatchRenderParams,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>
) -> Result<BatchRenderResult, Error> {
    let start_time = std::time::Instant::now();
    let force_render = params.force_render.unwrap_or(false);
    
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!(
                "🚀 Starting batch render for rev {} with {} operations defined",
                params.current_rev,
                params.operations_state.defs.len()
            )
        );
    }

    // Get operations to process
    let operations_to_process: Vec<(&OperationId, &FrontendOperationDef)> = 
        if let Some(ref specific_ids) = params.specific_operation_ids {
            // Only process specified operations
            params.operations_state.defs.iter()
                .filter(|(id, _)| specific_ids.contains(id))
                .collect()
        } else {
            // Process all operations
            params.operations_state.defs.iter().collect()
        };

    // Build dependency graph and compute render order (topological sort)
    let render_order = compute_render_order(&operations_to_process, &params.operations_state);
    
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!(
                "📊 Computed render order: {} operations to process",
                render_order.len()
            )
        );
        for (idx, op_id) in render_order.iter().enumerate() {
            if let Some(op) = params.operations_state.defs.get(op_id) {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "  {}. {} '{}' (policy: )",
                        idx + 1,
                        op.kind(),
                        op.name(),
                    )
                );
            }
        }
    }

    let mut results: Vec<OperationRenderResult> = Vec::new();
    let mut successful_renders = 0;
    let mut failed_renders = 0;
    let mut skipped_operations = 0;

    // Process operations in dependency order
    for op_id in render_order {
        let op = match params.operations_state.defs.get(&op_id) {
            Some(op) => op,
            None => {
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!("⚠️ Operation '{}' not found in state, skipping", op_id)
                    );
                }
                continue;
            }
        };

        let policy = op.render_policy();


        // Check if we should render this operation based on policy
        let should_render = match policy {
            Some(RenderPolicy::Auto) => true,
            Some(RenderPolicy::Manual) => force_render,
            Some(RenderPolicy::Frozen) => false,
            _ => false, // Default to auto behavior
        };

        if !should_render {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "⏭️ Skipping '{}' (policy: {})",
                        op.name(),
                        match policy {
                            Some(RenderPolicy::Auto) => "auto",
                            Some(RenderPolicy::Manual) => "manual",
                            Some(RenderPolicy::Frozen) => "frozen",
                            None => "auto (default)",
                        }
                    )
                );
            }
            skipped_operations += 1;
            results.push(OperationRenderResult {
                operation_id: op_id.clone(),
                operation_name: op.name().to_string(),
                success: true, // Skipped is not a failure
                error: Some(format!("Skipped due to render policy: {}",       
                match policy {
                            Some(RenderPolicy::Auto) => "auto",
                            Some(RenderPolicy::Manual) => "manual",
                            Some(RenderPolicy::Frozen) => "frozen",
                            None => "auto (default)",
                        })),
                duration_ms: 0,
            });
            continue;
        }

        // Render this operation
        let op_start = std::time::Instant::now();
        
        if let Ok(logger) = logging_service.lock() {
            log_info!(
                logger,
                LogSystem::Combine,
                &format!(
                    "🔄 Rendering {} '{}' (id: {})",
                    op.kind(),
                    op.name(),
                    op_id
                )
            );
        }

        let render_result = render_single_operation_internal(
            op,
            &params.operations_state,
            &logging_service,
        ).await;

        let op_duration = op_start.elapsed();

        match render_result {
            Ok(_message) => {
                successful_renders += 1;
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "✅ Successfully rendered '{}' in {:?}",
                            op.name(),
                            op_duration
                        )
                    );
                }
                results.push(OperationRenderResult {
                    operation_id: op_id.clone(),
                    operation_name: op.name().to_string(),
                    success: true,
                    error: None,
                    duration_ms: op_duration.as_millis() as u64,
                });
            }
            Err(e) => {
                failed_renders += 1;
                let error_msg = format!("{:?}", e);
                if let Ok(logger) = logging_service.lock() {
                    log_info!(
                        logger,
                        LogSystem::Combine,
                        &format!(
                            "❌ Failed to render '{}': {}",
                            op.name(),
                            error_msg
                        )
                    );
                }
                results.push(OperationRenderResult {
                    operation_id: op_id.clone(),
                    operation_name: op.name().to_string(),
                    success: false,
                    error: Some(error_msg),
                    duration_ms: op_duration.as_millis() as u64,
                });
            }
        }
    }

    let total_duration = start_time.elapsed();

    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!(
                "🏁 Batch render complete: {} successful, {} failed, {} skipped in {:?}",
                successful_renders,
                failed_renders,
                skipped_operations,
                total_duration
            )
        );
    }

    Ok(BatchRenderResult {
        total_operations: results.len(),
        successful_renders,
        failed_renders,
        skipped_operations,
        results,
        total_duration_ms: total_duration.as_millis() as u64,
        triggered_by_rev: params.current_rev,
    })
}

/// Compute the render order using topological sort based on operation dependencies
fn compute_render_order(
    operations: &[(&OperationId, &FrontendOperationDef)],
    _state: &FrontendOperationsState,
) -> Vec<OperationId> {
    use std::collections::{HashSet, VecDeque};

    let op_ids: HashSet<&OperationId> = operations.iter().map(|(id, _)| *id).collect();
    let mut in_degree: HashMap<OperationId, usize> = HashMap::new();
    let mut dependents: HashMap<OperationId, Vec<OperationId>> = HashMap::new();

    // Initialize in-degree for all operations
    for (id, _) in operations {
        in_degree.insert((*id).clone(), 0);
        dependents.insert((*id).clone(), Vec::new());
    }

    // Build dependency graph
    for (id, op) in operations {
        for source in op.sources() {
            match source {
                OperationSource::Operation { operation_id }
                | OperationSource::PreviousOperation { operation_id } => {
                    // Only count dependencies that are in our processing set
                    if op_ids.contains(&operation_id) {
                        *in_degree.entry((*id).clone()).or_insert(0) += 1;
                        dependents
                            .entry(operation_id.clone())
                            .or_insert_with(Vec::new)
                            .push((*id).clone());
                    }
                }
                _ => {}
            }
        }
    }

    // Kahn's algorithm for topological sort
    let mut queue: VecDeque<OperationId> = VecDeque::new();
    let mut result: Vec<OperationId> = Vec::new();

    // Start with operations that have no dependencies
    for (id, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(id.clone());
        }
    }

    while let Some(current) = queue.pop_front() {
        result.push(current.clone());

        if let Some(deps) = dependents.get(&current) {
            for dependent in deps {
                if let Some(degree) = in_degree.get_mut(dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }

    // If result doesn't contain all operations, there's a cycle
    // In that case, add remaining operations in arbitrary order
    for (id, _) in operations {
        if !result.contains(id) {
            result.push((*id).clone());
        }
    }

    result
}

/// Internal function to render a single operation
async fn render_single_operation_internal(
    op: &FrontendOperationDef,
    operations_state: &FrontendOperationsState,
    logging_service: &State<'_, Arc<Mutex<LoggingService>>>
) -> Result<String, Error> {
    let base_artifacts_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));
    let mut op_map: slotmap::SlotMap<OpId, ()> = slotmap::SlotMap::new();
    let default_sample_rate: u32 = 44100;

    match op {
        FrontendOperationDef::Merge {
            id,
            name,
            sources,
            output_path,
            ..
        } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔄 Rendering merge operation '{}' -> '{}'",
                        name, output_path
                    )
                );
            }

            // Resolve input artifacts
            let mut input_artifacts = Vec::new();
            for source in sources {
                match resolve_operation_source(
                    source,
                    operations_state,
                    &base_artifacts_dir,
                    &mut op_map,
                    default_sample_rate,
                    logging_service,
                ) {
                    Ok(artifacts) => input_artifacts.extend(artifacts),
                    Err(e) => {
                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!("⚠️ Failed to resolve source: {:?}", e)
                            );
                        }
                    }
                }
            }

            if input_artifacts.is_empty() {
                return Err(Error::Io(std::io::Error::other(
                    "No input artifacts resolved for merge operation",
                )));
            }

            // Create and execute merge operation
            let operation = MergeOpRender::new();
            let mut inputs = HashMap::new();
            inputs.insert(
                "inputs".to_string(),
                Artifact::AudioList(input_artifacts.clone()),
            );

            let op_id = op_map.insert(());
            let work_dir = base_artifacts_dir.join(format!("auto_merge_{:?}", op_id));
    
            if let Err(e) = std::fs::create_dir_all(&work_dir) {
                return Err(Error::Io(std::io::Error::other(format!(
                    "Failed to create work directory: {}",
                    e
                ))));
            }

            let context = OperationContext {
                op_id,
                work_dir,
                inputs,
                parameters: serde_json::json!({
                    "output_path": output_path,
                }),
                progress_callback: None,
            };

            operation.execute(context).map_err(|e| {
                Error::Io(std::io::Error::other(format!(
                    "Merge operation failed: {:?}",
                    e
                )))
            })?;
            Ok(format!("Merge '{}' completed with {} inputs", name, input_artifacts.len()))
        }
        FrontendOperationDef::Sample {
            id,
            name,
            sources,
            ..
        } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!("🔄 Rendering sample operation '{}'", name)
                );
            }

            // For sample operations, we need to resolve and process each source
            let mut processed_count = 0;
            for source in sources {
                match resolve_operation_source(
                    source,
                    operations_state,
                    &base_artifacts_dir,
                    &mut op_map,
                    default_sample_rate,
                    logging_service,
                ) {
                    Ok(artifacts) => {
                        processed_count += artifacts.len();
                    }
                    Err(e) => {
                        if let Ok(logger) = logging_service.lock() {
                            log_info!(
                                logger,
                                LogSystem::Combine,
                                &format!("⚠️ Failed to resolve source: {:?}", e)
                            );
                        }
                    }
                }
            }

            Ok(format!("Sample '{}' processed {} artifacts", name, processed_count))
        }
        FrontendOperationDef::Pipeline {
            id,
            name,
            operations,
            sources,
            ..
        } => {
            if let Ok(logger) = logging_service.lock() {
                log_info!(
                    logger,
                    LogSystem::Combine,
                    &format!(
                        "🔄 Rendering pipeline operation '{}' with {} steps",
                        name,
                        operations.len()
                    )
                );
            }

            // Pipeline operations chain multiple operations together
            // For now, we just report on what would be done
            Ok(format!(
                "Pipeline '{}' with {} operations: {:?}",
                name,
                operations.len(),
                operations
            ))
        }
            FrontendOperationDef::Export { id, name, render_policy, sources, output_path, params } => todo!(),
    }
}

// Helper trait to get render policy as string
// impl FrontendOperationDef {
//     fn render_policy_str(&self) -> &str {
//         match self {
//             FrontendOperationDef::Merge { render_policy, .. }
//             | FrontendOperationDef::Sample { render_policy, .. }
//             | FrontendOperationDef::Pipeline { render_policy, .. } => {
//                 match render_policy {
//                     Some(RenderPolicy::Auto) => "auto",
//                     Some(RenderPolicy::Manual) => "manual",
//                     Some(RenderPolicy::Frozen) => "frozen",
//                     None => "auto", // Default
//                 }
//             }
//         }
//     }
// }
