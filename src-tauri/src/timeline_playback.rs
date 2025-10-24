use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

use crate::epr;
use crate::looping_samples_buffer::LoopingSamplesBuffer;
use crate::state::AppState;

// Custom macro for bold red error messages

#[tauri::command]
pub fn play_timeline_audio(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    start_seconds: Option<f32>,
) {
    let state = state.inner().clone();
    println!("starting play thread");

    match start_seconds {
        Some(val) => {
            println!("got start time {}", val);
        }
        None => {
            println!("got  no time time");
        }
    }

    let state_clone = state.clone();
    let progress = state_clone.current_play_progress.lock().unwrap();
    println!("CUR PROGRESS: {}", *progress);
    thread::spawn(move || {
        // Fetch audio
        let combined_samples = {
            let guard = state.combined_audio.lock().unwrap();
            guard.clone()
        };

        let Some(samples) = combined_samples else {
            eprintln!("No combined audio available.");
            return;
        };

        if samples.is_empty() {
            eprintln!("Combined audio is empty.");
            return;
        }

        let sample_rate = 44100;
        let channels = 2;
        let total_samples = samples.len();

        // Calculate start position in seconds (we'll seek to this after creating the buffer)
        let start_position_seconds = if let Some(start) = start_seconds {
            start
        } else {
            let progress = state.current_play_progress.lock().unwrap();
            let total_duration = total_samples as f32 / (sample_rate as f32 * channels as f32);
            *progress * total_duration
        };

        // Always use full samples - no trimming!
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Error creating audio output stream: {}", e);
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

        let duration = samples.len() as f32 / (channels as f32 * sample_rate as f32);
        let start = Instant::now();
        println!(
            "🎵 Total samples: {}, Using FULL audio (no trimming)",
            samples.len()
        );

        // Create buffer with FULL samples
        let mut source =
            LoopingSamplesBuffer::new(channels as u16, sample_rate, samples.to_vec(), true);

        // Seek to the desired start position
        if start_position_seconds > 0.0 {
            source.seek_to_time(start_position_seconds);
            println!("🎯 Seeking to position: {:.2}s", start_position_seconds);
        }
        sink.append(source);
        sink.set_volume(1.0);
        sink.play();

        // Store the sink and immediately release the lock
        {
            let mut current_song = state.current_song.lock().unwrap();
            *current_song = Some(Arc::clone(&sink));
        } // Lock is released here

        // Set the seek start time based on where playback begins
        let seek_start_position = start_position_seconds;

        {
            let mut seek_start = state.seek_start_time.lock().unwrap();
            *seek_start = seek_start_position;
        }

        // Progress tracking in a separate thread
        let sink_clone = Arc::clone(&sink);
        let app_clone = app.clone();
        let state_clone = state.clone();
        thread::spawn(move || {
            let mut last_seek_position = seek_start_position;
            let mut tracking_start = start;

            // For looping audio, we need to continue tracking even after the sink empties and refills
            loop {
                // Check if sink is completely stopped (not just empty due to looping)
                if sink_clone.is_paused() && sink_clone.empty() {
                    break;
                }

                // Check if seeking has occurred by comparing current seek position
                let current_seek_position = {
                    let seek_start = state_clone.seek_start_time.lock().unwrap();
                    *seek_start
                };

                // If seek position changed, reset our tracking start time
                if (current_seek_position - last_seek_position).abs() > 0.001 {
                    tracking_start = Instant::now();
                    last_seek_position = current_seek_position;
                    println!(
                        "Seek detected! Reset tracking start time. New position: {}",
                        current_seek_position
                    );
                }

                // Calculate elapsed time from the last seek/start
                let elapsed = tracking_start.elapsed().as_secs_f32();

                // Calculate total audio duration for progress calculation
                let total_duration = total_samples as f32 / (sample_rate as f32 * channels as f32);

                // Current position = seek position + elapsed time since last seek/start
                let current_position = current_seek_position + elapsed;

                // For looping audio, calculate progress that wraps around
                let progress = if total_duration > 0.0 {
                    let wrapped_position = current_position % total_duration;
                    wrapped_position / total_duration
                } else {
                    0.0
                };

                // Update progress in app state
                {
                    let mut current_progress = state_clone.current_play_progress.lock().unwrap();
                    *current_progress = progress;
                }

                println!(
                    "EMITTED PROGRESS (LOOPING): {} (position: {:.2}s)",
                    progress, current_position
                );
                let _ = app_clone.emit("timeline-progress", progress);
                std::thread::sleep(Duration::from_millis(16)); // 60 FPS for smooth animation
            }

            println!("Progress tracking stopped");
        });

        // This blocks, but now it's in its own thread and doesn't hold any locks
        sink.sleep_until_end();
    });
}

#[tauri::command]
pub fn pause_timeline_audio(state: State<'_, Arc<AppState>>, app: AppHandle) {
    println!("PAUSING");
    let current_song = state.current_song.lock().unwrap();
    if let Some(sink) = &*current_song {
        // Get the current progress before stopping
        let current_progress = {
            let progress = state.current_play_progress.lock().unwrap();
            *progress
        };

        // Calculate the current position in seconds from progress
        let current_position = {
            let guard = state.combined_audio.lock().unwrap();
            if let Some(ref samples) = *guard {
                let sample_rate = 44100.0;
                let channels = 2.0;
                let total_duration = samples.len() as f32 / (sample_rate * channels);
                current_progress * total_duration
            } else {
                0.0
            }
        };

        // Calculate normalized progress (same as current_progress, but ensure consistency)
        let normalized_progress = current_progress;

        // Emit progress update to frontend
        let _ = app.emit("timeline-progress", normalized_progress);

        // Update seek_start_time to the current position so resume continues from here
        {
            let mut seek_start = state.seek_start_time.lock().unwrap();
            *seek_start = current_position;
        }

        sink.stop(); // Use stop() instead of pause() for immediate effect
        sink.clear(); // Clear any buffered audio

        println!(
            "Paused at position: {:.2}s (progress: {:.2})",
            current_position, current_progress
        );
    } else {
        println!("PAUSE FAILED");
    }
}

#[tauri::command]
pub fn get_current_play_progress(state: State<'_, Arc<AppState>>) -> f32 {
    let progress = state.current_play_progress.lock().unwrap();
    *progress
}

#[tauri::command]
pub fn set_volume(vol: f32, state: State<'_, Arc<AppState>>) {
    let mut current_song = state.current_song.lock().unwrap();
    if let Some(ref sink) = *current_song {
        sink.set_volume(vol);
    }
}

#[tauri::command]
pub fn set_timeline_play_position(position: f32, state: State<'_, Arc<AppState>>, app: AppHandle) {
    println!("SETTING PLAY POSITINO");
    let current_song = state.current_song.lock().unwrap();

    if let Some(ref sink) = *current_song {
        let position_duration = Duration::from_secs_f32(position);
        println!("POSITION_DURATION: {} ", position_duration.as_secs_f32());
        match sink.try_seek(position_duration) {
            Ok(_) => {
                println!("Successfully seeked to position: {:.2}s", position);

                // Store the seek position for accurate progress tracking
                {
                    let mut seek_start = state.seek_start_time.lock().unwrap();
                    *seek_start = position;
                }

                // Calculate normalized progress (0-1) based on combined audio duration
                let total_duration = {
                    let guard = state.combined_audio.lock().unwrap();
                    if let Some(ref samples) = *guard {
                        let sample_rate = 44100.0;
                        let channels = 2.0;
                        samples.len() as f32 / (sample_rate)
                    } else {
                        1.0 // Default fallback
                    }
                };
                println!("TOTAL DURATION: {}", total_duration);
                let normalized_progress = (position / total_duration).min(1.0).max(0.0);

                // Update the progress in app state
                {
                    let mut progress = state.current_play_progress.lock().unwrap();
                    println!("NEW PROGRESS: {}", normalized_progress);
                    *progress = normalized_progress;
                }

                let _ = app.emit("timeline-progress", normalized_progress);
            }
            Err(e) => {
                epr!("Failed to seek to position {:.2}s: {:?}", position, e);
                // Fallback to the original method if seeking fails
                drop(current_song); // Release the lock before calling the fallback
                set_timeline_play_position_fallback(position, state, app);
            }
        }
    } else {
        eprintln!("No audio currently playing to seek.");
    }
}

// Fallback method using stream recreation (original implementation)
fn set_timeline_play_position_fallback(
    position_seconds: f32,
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) {
    let state_clone = state.inner().clone();

    // Stop current playback
    {
        let mut current_song = state_clone.current_song.lock().unwrap();
        if let Some(ref sink) = *current_song {
            sink.stop();
            sink.clear();
        }
        *current_song = None;
    }

    // Recreate stream from position (original implementation)
    thread::spawn(move || {
        let combined_samples = {
            let guard = state_clone.combined_audio.lock().unwrap();
            guard.clone()
        };

        let Some(samples) = combined_samples else {
            eprintln!("No combined audio available for seeking.");
            return;
        };

        if samples.is_empty() {
            eprintln!("Combined audio is empty.");
            return;
        }

        let sample_rate = 44100;
        let channels = 2;
        let total_samples = samples.len();

        // Always use full samples - no trimming!
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(output) => output,
            Err(e) => {
                eprintln!("Error creating audio output stream: {}", e);
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

        let duration = samples.len() as f32 / (channels as f32 * sample_rate as f32);
        let start = Instant::now();

        println!("🎵 Fallback: Using FULL audio samples: {}", samples.len());

        // Create buffer with FULL samples
        let mut source =
            LoopingSamplesBuffer::new(channels as u16, sample_rate, samples.to_vec(), true);

        // Seek to the desired position
        source.seek_to_time(position_seconds);
        println!("🎯 Fallback: Seeking to position: {:.2}s", position_seconds);
        sink.append(source);
        sink.set_volume(1.0);
        sink.play();

        // Store the sink
        {
            let mut current_song = state_clone.current_song.lock().unwrap();
            *current_song = Some(Arc::clone(&sink));
        }

        // Store the seek position for accurate progress tracking
        {
            let mut seek_start = state_clone.seek_start_time.lock().unwrap();
            *seek_start = position_seconds;
        }

        // Progress tracking
        let sink_clone = Arc::clone(&sink);
        let app_clone = app.clone();
        let state_clone_for_progress = state_clone.clone();
        thread::spawn(move || {
            let mut last_seek_position = position_seconds;
            let mut tracking_start = start;

            // For looping audio, continue tracking beyond initial playthrough
            loop {
                // Check if sink is completely stopped
                if sink_clone.is_paused() && sink_clone.empty() {
                    break;
                }

                // Check for seeking
                let current_seek_position = {
                    let seek_start = state_clone_for_progress.seek_start_time.lock().unwrap();
                    *seek_start
                };

                // Reset tracking if seek occurred
                if (current_seek_position - last_seek_position).abs() > 0.001 {
                    tracking_start = Instant::now();
                    last_seek_position = current_seek_position;
                }

                // Calculate progress
                let elapsed = tracking_start.elapsed().as_secs_f32();
                let total_duration = total_samples as f32 / (sample_rate as f32 * channels as f32);
                let current_position = current_seek_position + elapsed;

                // Wrap progress for looping
                let progress = if total_duration > 0.0 {
                    let wrapped_position = current_position % total_duration;
                    wrapped_position / total_duration
                } else {
                    0.0
                };

                // Update state
                {
                    let mut current_progress = state_clone_for_progress
                        .current_play_progress
                        .lock()
                        .unwrap();
                    *current_progress = progress;
                }

                let _ = app_clone.emit("timeline-progress", progress);
                std::thread::sleep(Duration::from_millis(16));
            }
        });

        sink.sleep_until_end();
    });
}

#[tauri::command]
pub fn stop_timeline_audio(state: State<'_, Arc<AppState>>, app: AppHandle) {
    println!("STOPPING");
    let current_song = state.current_song.lock().unwrap();
    if let Some(sink) = &*current_song {
        sink.stop(); // Stop playback immediately
        sink.clear(); // Clear any buffered audio

        // Reset progress to 0
        {
            let mut current_progress = state.current_play_progress.lock().unwrap();
            *current_progress = 0.0;
        }

        // Reset seek start time to 0
        {
            let mut seek_start = state.seek_start_time.lock().unwrap();
            *seek_start = 0.0;
        }

        // Emit progress update to frontend
        let _ = app.emit("timeline-progress", 0.0);

        println!("Stopped and reset to beginning");
    } else {
        println!("STOP FAILED - No audio playing");
    }
}

#[tauri::command]
pub fn set_timeline_loop_enabled(
    loop_enabled: bool,
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) {
    println!("Setting timeline loop to: {}", loop_enabled);

    // Get current playback position if playing
    let current_position = {
        let progress = state.current_play_progress.lock().unwrap();
        let guard = state.combined_audio.lock().unwrap();
        if let Some(ref samples) = *guard {
            let sample_rate = 44100.0;
            let channels = 2.0;
            let total_duration = samples.len() as f32 / (sample_rate * channels);
            *progress * total_duration
        } else {
            0.0
        }
    };

    // Check if currently playing
    let is_playing = {
        let current_song = state.current_song.lock().unwrap();
        current_song.is_some()
    };

    if is_playing {
        println!(
            "Restarting playback with new loop setting at position: {:.2}s",
            current_position
        );

        // Stop current playback
        let current_song = state.current_song.lock().unwrap();
        if let Some(sink) = &*current_song {
            sink.stop();
            sink.clear();
        }
        drop(current_song);

        // Restart with new loop setting
        let state_clone = state.inner().clone();
        let app_clone = app.clone();

        thread::spawn(move || {
            // Small delay to ensure cleanup
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Get audio samples
            let combined_samples = {
                let guard = state_clone.combined_audio.lock().unwrap();
                guard.clone()
            };

            let Some(samples) = combined_samples else {
                return;
            };

            if samples.is_empty() {
                return;
            }

            let sample_rate = 44100;
            let channels = 2;

            // Create new audio stream and sink
            let (_stream, stream_handle) = match OutputStream::try_default() {
                Ok(output) => output,
                Err(_) => return,
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(sink) => Arc::new(sink),
                Err(_) => return,
            };

            // Create buffer with new loop setting
            let mut source = LoopingSamplesBuffer::new(
                channels as u16,
                sample_rate,
                samples.to_vec(),
                loop_enabled,
            );

            // Seek to previous position
            if current_position > 0.0 {
                source.seek_to_time(current_position);
            }

            sink.append(source);
            sink.set_volume(1.0);
            sink.play();

            // Store the new sink
            {
                let mut current_song = state_clone.current_song.lock().unwrap();
                *current_song = Some(sink);
            }

            // Update seek start time
            {
                let mut seek_start = state_clone.seek_start_time.lock().unwrap();
                *seek_start = current_position;
            }

            println!("Playback restarted with loop_enabled: {}", loop_enabled);
        });
    } else {
        println!("Not currently playing - loop setting will apply to next playback");
    }
}
