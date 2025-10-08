use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fs, thread};
use tauri::{AppHandle, Emitter, State};

use crate::metadata;
use crate::state::AppState;

#[tauri::command]
pub fn pause_sample_preview(state: State<'_, Arc<AppState>>) {
    let mut current_song = state.current_song.lock().unwrap();
    if let Some(ref sink) = *current_song {
        println!("PAUSING!!!!");
        sink.pause();
        state.cancel_playback.store(true, Ordering::Relaxed);
        // *current_song = None;
    } else {
        println!("FAILED!!")
    }
}

#[tauri::command]
pub fn play_sample_preview(title: String, state: State<'_, Arc<AppState>>, app: AppHandle) {
    let path = title.clone();
    let state = state.inner().clone();
    log::info!("Got request to play_sample_preview {}", title);
    state.cancel_playback.store(false, Ordering::Relaxed);

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

    {
        let mut current_song = state.current_song.lock().unwrap();
        if let Some(ref current) = *current_song {
            current.stop(); // ❗ Ensure previous song is stopped
        }
        *current_song = None; // Clear it before continuing
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

        // Save the new sink
        {
            let mut current_song = state.current_song.lock().unwrap();
            *current_song = Some(Arc::clone(&sink));
        }

        let duration = metadata::get_duration(&path);
        let start = Instant::now();

        let sink_clone = Arc::clone(&sink);
        let app_clone = app.clone();

        thread::spawn(move || {
            let mut done_emitted = false;
            // let cancel_flag = state.cancel_playback.load(Ordering::Relaxed); // pass into the thread
            // println!("{}", cancel_flag);
            while !sink_clone.empty() && !done_emitted && !sink_clone.is_paused() {
                // println!("{}", cancel_flag);
                let elapsed = start.elapsed();
                let elapsed_secs = elapsed.as_secs_f32();

                if let Some(duration) = duration {
                    let progress = (elapsed_secs / duration).min(1.0);
                    let _ = app_clone.emit("song-progress", progress);

                    if progress >= 1.0 {
                        done_emitted = true;
                        break;
                    }
                }

                std::thread::sleep(Duration::from_millis(200));
            }

            let _ = app_clone.emit("song-progress", 1.0);
            sink_clone.clear();
        });

        sink.set_volume(1.0);
        sink.sleep_until_end();
    });
}
