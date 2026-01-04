use log;
use std::collections::HashMap;
use std::fs::{metadata, File};
use std::io::BufReader;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, thread};
use tauri::Listener;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::Error;
use crate::logging::{LogSystem, LoggingConfig, LoggingService};
use crate::metadata::get_metadata;
use crate::state::AppState;

mod audio_manager;
mod combine;
mod encoder;
mod error;
mod logging;
mod looping_samples_buffer;
mod macros;
mod metadata;
mod sample_playback;
mod sorting;
mod state;
mod timeline_playback;
mod artifacts;
mod cook;
mod graph;
mod ops;
mod util;

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
fn open_in_explorer(state: State<'_, Arc<AppState>>, file_to_open: String) {
    println!("SHOWING IN EXP");
    showfile::show_path_in_file_manager(file_to_open);
}

#[tauri::command]
async fn test_operation(
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
            use crate::artifacts::{Artifact, AudioArtifact};
            use crate::ops::{MergeOperation, Operation, OperationContext};
            use crate::graph::OpId;
            use std::collections::HashMap;

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
            
            let context = OperationContext {
                op_id,
                work_dir: std::env::temp_dir(),
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
                        "✅ Operation '{}' completed successfully!\n\n📄 Result: {}\n🔧 Operation Type: Merge/Combine\n📊 Input Files: 2 test audio files\n⏱️ Estimated Duration: 8.0 seconds",
                        operation_name,
                        match result {
                            Artifact::Audio(audio) => format!("Audio file: {}", audio.path.display()),
                            _ => "Processed successfully".to_string()
                        }
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard::init())
        .setup(|app| {
            // Initialize logging service
            let mut logging_service = LoggingService::new();
            logging_service.set_app_handle(app.handle().clone());
            app.manage(Arc::new(Mutex::new(logging_service)));

            // Initialize app state
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
            }));

            #[cfg(debug_assertions)] // Only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                app.listen("download-started", |event| {});
            }
            #[cfg(not(debug_assertions))] // Only for release builds
            {
                app.listen("download-started", |event| {});
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_file_paths_in_folder,
            sample_playback::play_sample_preview,
            sample_playback::pause_sample_preview,
            timeline_playback::set_timeline_play_position,
            timeline_playback::get_current_play_progress,
            timeline_playback::play_timeline_audio,
            timeline_playback::pause_timeline_audio,
            timeline_playback::stop_timeline_audio,
            timeline_playback::set_volume,
            get_metadata,
            combine::test_async,
            combine::update_inputs,
            combine::combine_all_cached_samples,
            combine::combine_all_cached_samples_with_custom_order,
            combine::get_custom_order,
            combine::cancel_combine,
            combine::toggle_audio_file_active,
            combine::set_audio_file_active,
            combine::set_audio_files_active_batch,
            combine::get_audio_file_active_status,
            state::get_app_state,
            clear_audio_files,
            encoder::export_audio,
            open_in_explorer,
            sorting::update_sorting,
            update_logging_config,
            get_logging_config,
            test_operation,
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
