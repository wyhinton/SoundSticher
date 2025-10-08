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
use crate::metadata::get_metadata;
use crate::state::AppState;
mod combine;
mod encoder;
mod error;
mod metadata;
mod sample_playback;
mod sorting;
mod state;
mod timeline_playback;

pub struct Song {
    pub title: String,
}

#[tauri::command]
fn get_file_paths_in_folder(
    folder_paths: Vec<String>,
) -> Result<HashMap<String, Vec<String>>, Error> {
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

        println!("{}: {} files", folder_path, valid_files.len());
        all_paths.insert(folder_path, valid_files);
    }

    Ok(all_paths)
}

#[tauri::command]
fn clear_audio_files(state: State<'_, Arc<AppState>>, app: AppHandle) {
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
fn open_in_explorer(state: State<'_, Arc<AppState>>, file_to_open: String) {
    println!("SHOWING IN EXP");
    showfile::show_path_in_file_manager(file_to_open);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard::init())
        .setup(|app| {
            {
                let window = app.get_webview_window("main").unwrap();
                // window.open_devtools();
                // window.close_devtools();
                app.listen("download-started", |event| {});
            }
            Ok(())
        })
        .manage(Arc::new(AppState {
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
        }))
        .invoke_handler(tauri::generate_handler![
            get_file_paths_in_folder,
            sample_playback::play_sample_preview,
            sample_playback::pause_sample_preview,
            timeline_playback::set_timeline_play_position,
            timeline_playback::get_current_play_progress,
            timeline_playback::play_timeline_audio,
            timeline_playback::pause_timeline_audio,
            timeline_playback::set_volume,
            get_metadata,
            combine::test_async,
            combine::update_inputs,
            combine::combine_all_cached_samples,
            combine::combine_all_cached_samples_with_custom_order,
            combine::get_custom_order,
            combine::cancel_combine,
            combine::export_combined_audio_as_wav,
            state::get_app_state,
            clear_audio_files,
            encoder::export_audio,
            open_in_explorer,
            sorting::update_sorting,
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
