use log;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::{metadata, File};
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fs, thread};
use tauri::Listener;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_log::{Target, TargetKind};

use crate::error::{Error, ErrorKind};
use crate::metadata::get_metadata;
mod combine;
mod error;
mod metadata;

pub struct AppState {
    current_song: Mutex<Option<Arc<Sink>>>,
}

pub struct Song {
    pub title: String,
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_songs() -> Vec<Song> {
    let mut music_files = Vec::new();
    let entries = fs::read_dir("../../src/assets/test_audio").unwrap();

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    let song = Song {
                        title: file_name_str.to_string(),
                    };
                    music_files.push(song);
                }
            }
        }
    }
    return music_files;
}

#[tauri::command]
fn get_file_paths_in_folder(folder_path: &str) -> Result<Vec<String>, Error> {
    let mut paths = Vec::new();

    let entries = std::fs::read_dir(folder_path)?; // Uses `From` to convert into AppError

    for entry in entries {
        let entry = entry?; // Also converts into AppError
        let path = entry.path();

        println!("entry {}", &path.display());

        if path.is_file() {
            // ❌ Skip hidden metadata files like "._track.mp3"
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.starts_with("._") {
                    continue;
                }
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();

                // ✅ Allow common audio file extensions
                if [
                    "mp3", "wav", "flac", "ogg", "m4a", "aac", "aiff", "alac", "aif",
                ]
                .contains(&ext.as_str())
                {
                    let path_str = path.to_str().ok_or(Error::InvalidPath)?;
                    paths.push(path_str.to_string());
                }
            }
        }
    }

    println!("Total valid files: {}", paths.len());
    Ok(paths)
}
#[tauri::command]
fn play_song(title: String, state: State<'_, Arc<AppState>>, app: AppHandle) {
    let path = title.clone();
    // let path = format!("../assets/test_audio/{}", title);
    let state = state.inner().clone();
    log::info!("Got request to play_song {}", title);

    match metadata(&path) {
        Ok(meta) => {
            if !meta.is_file() {
                eprintln!("Path exists but is not a file: {}", path);
                return;
            }
        }
        Err(e) => {
            eprintln!("Error accessing file metadata for {}: {}", path, e);
            return;
        }
    }

    thread::spawn(move || {
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(e) => {
                eprint!("Error opening file {}: {}", path, e);
                return;
            }
        };

        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Error making stream {}:{}", title, e);
                return;
            }
        };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(sink) => Arc::new(sink),
            Err(e) => {
                eprintln!("Error creating sink: {}", e);
                return;
            }
        };

        match Decoder::new(BufReader::new(file)) {
            Ok(source) => sink.append(source),
            Err(e) => {
                eprintln!("Error decoding audio file: {}", e);
                return;
            }
        }

        let duration = metadata::get_duration(&path);
        let start = Instant::now();

        // Progress emit thread
        let sink_clone = Arc::clone(&sink);

        thread::spawn(move || {
            while !sink_clone.empty() {
                let elapsed = start.elapsed();
                let elapsed_secs = elapsed.as_secs_f32();

                if let Some(duration) = duration {
                    let progress = (elapsed_secs / duration).min(1.0);

                    // 🔥 Emit progress event
                    let _ = app.emit("song-progress", progress);
                }

                std::thread::sleep(Duration::from_millis(200));
            }

            // Final emit at end
            let _ = app.emit("song-progress", 1.0);
        });

        {
            let mut current_song = state.current_song.lock().unwrap();

            if let Some(ref current) = *current_song {
                current.stop(); // Stop the previous song
            }

            *current_song = Some(Arc::clone(&sink)); // Track the new one
        }

        sink.set_volume(1.0);
        sink.sleep_until_end();
    });
}

#[tauri::command]
fn pause_song(state: State<'_, Arc<AppState>>) {
    let mut current_song = state.current_song.lock().unwrap();
    if let Some(ref sink) = *current_song {
        println!("PAUSING!!!!");
        sink.pause();
    } else {
        println!("FAILED!!")
    }
}

#[tauri::command]
fn set_volume(vol: f32, state: State<'_, Arc<AppState>>) {
    let mut current_song = state.current_song.lock().unwrap();
    if let Some(ref sink) = *current_song {
        sink.set_volume(vol);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
                app.listen("download-started", |event| {});
            }
            Ok(())
        })
        .manage(Arc::new(AppState {
            current_song: Mutex::new(None),
        }))
        .invoke_handler(tauri::generate_handler![
            greet,
            set_volume,
            get_file_paths_in_folder,
            play_song,
            pause_song,
            get_metadata,
            combine::combine_audio_files,
        ])
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        // .run(context::generate_context("../targets").into())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
