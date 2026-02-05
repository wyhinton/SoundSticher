#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::metadata;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use tauri::Listener;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::duration_service::DurationService;
use crate::error::Error;
use crate::graph::OperationGraph;
use crate::logging::{LogSystem, LoggingConfig, LoggingService};
use crate::metadata::get_metadata;
use crate::sample_cache::SampleCacheService;
use crate::state::AppState;
use crate::waveform::WaveformService;

mod artifact_registry_commands;
mod artifacts;
mod artifacts_service;
mod audio_manager;
mod combine;
mod cook;
mod duration_cache;
mod duration_service;
mod encoder;
mod error;
mod favorites;
mod graph;
mod logging;
mod looping_samples_buffer;
mod macros;
mod metadata;
mod op_playback_commands;
mod playback;
mod playback_ops;
mod render_ops;
mod sample_cache;
mod sample_playback;
mod sorting;
mod state;
mod timeline_playback;
mod util;
mod waveform;

pub struct Song {
    pub title: String,
}

#[tauri::command]
fn get_file_paths_in_folder(
    folder_paths: Vec<String>,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<HashMap<String, Vec<String>>, Error> {
    if let Ok(logger) = logging_service.lock() {
        log_info!(
            logger,
            LogSystem::Combine,
            &format!("Scanning {} folder(s) for audio files", folder_paths.len())
        );
    }

    let mut all_paths: HashMap<String, Vec<String>> = HashMap::new();

    for folder_path in folder_paths {
        let mut valid_files = Vec::new();
        let entries = std::fs::read_dir(&folder_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    if file_name.starts_with("._") {
                        continue;
                    }
                }

                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_lowercase();
                    if [
                        "mp3", "wav", "flac", "ogg", "m4a", "aac", "aiff", "alac", "aif",
                    ]
                    .contains(&ext.as_str())
                    {
                        let path_str = path.to_str().ok_or(Error::InvalidPath)?;
                        valid_files.push(path_str.to_string());
                    }
                }
            }
        }

        if let Ok(logger) = logging_service.lock() {
            log_debug!(
                logger,
                LogSystem::Combine,
                &format!(
                    "Found {} valid audio files in: {}",
                    valid_files.len(),
                    folder_path
                )
            );
        }

        println!("{}: {} files", folder_path, valid_files.len());
        all_paths.insert(folder_path, valid_files);
    }

    Ok(all_paths)
}

#[tauri::command]
fn clear_audio_files(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) {
    if let Ok(logger) = logging_service.lock() {
        log_info!(logger, LogSystem::Combine, "Clearing all audio files");
    }

    let mut audio_files = state.audio_files.lock().unwrap();
    audio_files.clear();
    let mut combined_audio = state.combined_audio.lock().unwrap();
    *combined_audio = None;
    let mut custom_order = state.custom_order.lock().unwrap();
    custom_order.clear();
    let _ = app.emit("buffering-progress", 0.);
    println!("🗑️  All audio files have been cleared.");
}

#[tauri::command]
fn update_logging_config(
    config: LoggingConfig,
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<(), Error> {
    if let Ok(service) = logging_service.lock() {
        service.update_config(config);
    }
    Ok(())
}

#[tauri::command]
fn get_logging_config(
    logging_service: State<'_, Arc<Mutex<LoggingService>>>,
) -> Result<LoggingConfig, Error> {
    if let Ok(service) = logging_service.lock() {
        Ok(service.get_config())
    } else {
        Ok(LoggingConfig::default())
    }
}

#[tauri::command]
fn open_in_explorer(_state: State<'_, Arc<AppState>>, file_to_open: String) {
    println!("SHOWING IN EXP");
    showfile::show_path_in_file_manager(file_to_open);
}

#[tauri::command]
fn get_artifacts_directory() -> String {
    let artifacts_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));
    artifacts_dir.to_string_lossy().to_string()
}

/// Open a file in VS Code at a specific line
#[tauri::command]
fn open_file_in_editor(file_path: String, line_number: Option<u32>) -> Result<(), String> {
    use std::process::Command;

    let path = file_path.replace('\\', "/");
    println!("Normalized file path: {}", path);

    let args = if let Some(line) = line_number {
        println!("Opening at line: {}", line);
        vec!["--goto".to_string(), format!("{}:{}", path, line)]
    } else {
        println!("Opening without line number");
        vec![path.clone()]
    };
    println!("VS Code args: {:?}", args);

    #[cfg(target_os = "windows")]
    {
        // Common VS Code installation paths on Windows
        let possible_paths = vec![
            std::path::PathBuf::from("C:\\Program Files\\Microsoft VS Code\\bin\\code.cmd"),
            std::path::PathBuf::from("C:\\Program Files (x86)\\Microsoft VS Code\\bin\\code.cmd"),
            std::path::PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
                .join("Programs\\Microsoft VS Code\\bin\\code.cmd"),
        ];

        // Try each path
        for code_path in possible_paths {
            println!("Trying VS Code path: {:?}", code_path);
            if code_path.exists() {
                println!("Found VS Code at: {:?}", code_path);
                return Command::new(&code_path)
                    .args(&args)
                    .spawn()
                    .map(|_child| {
                        // Don't wait for the process to finish, just let it run
                        // let _ = child.kill();
                    })
                    .map_err(|e| {
                        println!("Failed to launch VS Code at {:?}: {}", code_path, e);
                        format!("Failed to launch VS Code: {}", e)
                    });
            }
        }

        // Try using 'code' from PATH as fallback
        println!("Attempting to run: code {:?}", args);
        match Command::new("code").args(&args).spawn() {
            Ok(_) => return Ok(()),
            Err(e) => println!("Failed to launch 'code' from PATH: {}", e),
        }

        Err("VS Code not found. Make sure VS Code is installed and 'code' command is available in PATH.".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("Attempting to run: code {:?}", args);
        Command::new("code").args(&args).spawn().map_err(|e| {
            println!("Failed to open file in VS Code: {}", e);
            format!("Failed to open file in VS Code: {}", e)
        })?;
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard::init());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

    builder
        .setup(|app| {
            // Initialize logging service
            let mut logging_service = LoggingService::new();
            logging_service.set_app_handle(app.handle().clone());
            let logging_service = Arc::new(Mutex::new(logging_service));
            app.manage(logging_service.clone());
            let shared_artifact_registry = Arc::new(artifacts::ArtifactRegistry::new());
            // Initialize cook scheduler
            {
                use crate::artifacts::ArtifactStorage;
                use crate::cook::{CookScheduler, SchedulerConfig};
                use crate::graph::{InvalidationManager, OperationNodeManager};
                use crate::render_ops::{MergeOpRender, OperationRegistry};

                // Create operation registry and register operations
                let mut operation_registry = OperationRegistry::new();
                operation_registry.register(MergeOpRender::new());
                let operation_registry = Arc::new(operation_registry);

                // Create other components
                let operation_graph = OperationGraph::new();
                let node_manager = Arc::new(Mutex::new(OperationNodeManager::new()));
                let invalidation_manager =
                    Arc::new(Mutex::new(InvalidationManager::new(operation_graph)));

                // Create artifact storage
                let storage_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));
                let artifact_storage = match ArtifactStorage::new(storage_dir, 100 * 1024 * 1024) {
                    Ok(storage) => Arc::new(storage),
                    Err(e) => {
                        eprintln!("Failed to create artifact storage: {}", e);
                        return Err(Box::new(e));
                    }
                };

                // Create scheduler configuration
                let config = SchedulerConfig::default();

                // Extract logger from mutex for scheduler
                let logger = {
                    if let Ok(_service) = logging_service.lock() {
                        // Create a new logging service instance for the scheduler
                        let mut scheduler_logger = LoggingService::new();
                        scheduler_logger.set_app_handle(app.handle().clone());
                        Arc::new(scheduler_logger)
                    } else {
                        Arc::new(LoggingService::new())
                    }
                };

                // Create and start scheduler
                let mut scheduler = CookScheduler::new(
                    operation_registry,
                    node_manager,
                    invalidation_manager,
                    artifact_storage,
                    shared_artifact_registry.clone(),
                    config,
                    logger,
                );

                if let Err(e) = scheduler.start() {
                    eprintln!("Failed to start scheduler: {}", e);
                    return Err(Box::new(e));
                }

                app.manage(Arc::new(Mutex::new(scheduler)));
            }

            // Initialize app state with shared artifact registry

            app.manage(Arc::new(AppState {
                current_song: Mutex::new(None),
                audio_files: Mutex::new(std::collections::BTreeMap::new()),
                combined_audio: Mutex::new(None),
                cancel_playback: AtomicBool::new(false),
                buffering_samples: AtomicBool::new(false),
                svg_path: Mutex::new(None),
                cancel_token: AtomicU64::new(0),
                combine_process: Arc::new(Mutex::new(0)),
                custom_order: Mutex::new(Vec::new()),
                current_play_progress: Mutex::new(0.0),
                seek_start_time: Mutex::new(0.0),
                artifact_registry: shared_artifact_registry.clone(),
            }));

            // Initialize waveform cache service
            app.manage(Arc::new(WaveformService::new()));

            // Initialize duration service for proportional waveform width calculation
            app.manage(Arc::new(DurationService::new()));

            // Initialize sample cache service
            app.manage(Arc::new(SampleCacheService::new()));

            // Initialize operation-based playback state
            app.manage(Arc::new(op_playback_commands::OpPlaybackState::new()));

            #[cfg(debug_assertions)] // Only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                app.listen("download-started", |_event| {});
            }
            #[cfg(not(debug_assertions))] // Only for release builds
            {
                app.listen("download-started", |event| {});
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            clear_audio_files,
            combine::cancel_combine,
            combine::combine_all_cached_samples_with_custom_order,
            combine::combine_all_cached_samples,
            combine::get_audio_file_active_status,
            combine::get_custom_order,
            combine::set_audio_file_active,
            combine::set_audio_files_active_batch,
            combine::test_async,
            combine::toggle_audio_file_active,
            duration_cache::get_duration_cache_stats,
            duration_service::clear_duration_cache,
            duration_service::get_duration,
            duration_service::get_durations_batch,
            duration_service::invalidate_duration,
            encoder::export_audio,
            favorites::count_audio_files_in_folders,
            get_artifacts_directory,
            get_file_paths_in_folder,
            get_logging_config,
            get_metadata,
            render_ops::render_graph_tests::render_all_auto_operations,
            render_ops::render_graph_tests::test_render_single_operation,
            render_ops::render_graph_tests::test_scheduler,
            sample_cache::clear_sample_cache,
            sample_cache::get_sample_cache_stats,
            sample_cache::invalidate_sample_cache,
            op_playback_commands::op_playback_build_graph,
            op_playback_commands::op_playback_build_graph_legacy,
            op_playback_commands::op_playback_clear_graph,
            op_playback_commands::op_playback_get_progress,
            op_playback_commands::op_playback_pause,
            op_playback_commands::op_playback_play,
            op_playback_commands::op_playback_resume,
            op_playback_commands::op_playback_seek,
            op_playback_commands::op_playback_set_loop,
            op_playback_commands::op_playback_set_volume,
            op_playback_commands::op_playback_stop,
            // Artifact registry commands
            artifact_registry_commands::get_artifact_registry_records,
            artifact_registry_commands::get_artifact_registry_stats,
            artifact_registry_commands::get_artifacts_by_operation,
            artifact_registry_commands::clear_artifact_registry,
            artifact_registry_commands::refresh_artifact_registry_status,
            // Artifacts service (debug) commands
            artifacts_service::get_artifact_debug_info,
            artifacts_service::get_filtered_artifacts,
            artifacts_service::clear_artifact_registry_debug,
            artifacts_service::refresh_artifacts_existence,
            artifacts_service::remove_artifacts_by_operation_debug,
            artifacts_service::get_artifact_details_debug,
            open_file_in_editor,
            open_in_explorer,
            sample_playback::pause_sample_preview,
            sample_playback::play_sample_preview,
            sorting::update_sorting,
            state::get_app_state,
            timeline_playback::get_current_play_progress,
            timeline_playback::pause_timeline_audio,
            timeline_playback::play_timeline_audio,
            timeline_playback::set_timeline_play_position,
            timeline_playback::set_volume,
            timeline_playback::stop_timeline_audio,
            update_logging_config,
            waveform::clear_waveform_cache,
            waveform::get_waveform_cache_stats,
            waveform::get_waveform,
            waveform::get_waveforms_batch,
            waveform::get_waveforms_for_operation,
            waveform::invalidate_waveform,
        ])
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .filter(|metadata| {
                    let target = metadata.target();
                    !target.contains("symphonia") && !target.contains("lofty")
                })
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        // .run(context::generate_context("../targets").into())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
