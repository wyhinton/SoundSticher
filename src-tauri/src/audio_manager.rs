use crate::looping_samples_buffer::LoopingSamplesBuffer;
use crate::state::AppState;
use rodio::{OutputStream, Sink};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub struct AudioManager {
    state: Arc<AppState>,
    app: AppHandle,
    progress_thread_handle: Option<thread::JoinHandle<()>>,
    should_stop_progress: Arc<Mutex<bool>>,
}

impl AudioManager {
    pub fn new(state: Arc<AppState>, app: AppHandle) -> Self {
        Self {
            state,
            app,
            progress_thread_handle: None,
            should_stop_progress: Arc::new(Mutex::new(false)),
        }
    }

    pub fn play_from_position(&mut self, start_seconds: Option<f32>) -> Result<(), String> {
        println!("starting play thread");

        match start_seconds {
            Some(val) => println!("got start time {}", val),
            None => println!("got no time time"),
        }

        // Stop any existing playback first
        self.stop_progress_tracking();

        let combined_samples = {
            let guard = self.state.combined_audio.lock().unwrap();
            guard.clone()
        };

        let Some(samples) = combined_samples else {
            return Err("No combined audio available".to_string());
        };

        if samples.is_empty() {
            return Err("Combined audio is empty".to_string());
        }

        let sample_rate = 44100;
        let channels = 2;
        let total_samples = samples.len();

        // Calculate start position
        let play_progress = if let Some(start) = start_seconds {
            start / (total_samples as f32 / (sample_rate as f32 * channels as f32))
        } else {
            let progress = self.state.current_play_progress.lock().unwrap();
            *progress
        };

        println!("CUR PROGRESS: {}", play_progress);

        let start_sample_index = (play_progress * total_samples as f32).round() as usize;
        if start_sample_index >= total_samples {
            return Err("Start time exceeds audio length".to_string());
        }

        // Create audio stream and sink
        let trimmed_samples = &samples[start_sample_index..];
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Error creating audio output stream: {}", e))?;

        let sink = Arc::new(
            Sink::try_new(&stream_handle).map_err(|e| format!("Error creating sink: {}", e))?,
        );

        // // Convert f32 to i16 if needed (assuming your samples are f32)
        // let i16_samples: Vec<i16> = trimmed_samples
        //     .iter()
        //     .map(|&sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        //     .collect();

        let source =
            LoopingSamplesBuffer::new(channels as u16, sample_rate, trimmed_samples.to_vec(), true);
        sink.append(source);
        sink.set_volume(1.0);
        sink.play();

        // Store the sink
        {
            let mut current_song = self.state.current_song.lock().unwrap();
            *current_song = Some(Arc::clone(&sink));
        }

        // Calculate and store seek start position
        let seek_start_position = if let Some(start) = start_seconds {
            start
        } else {
            let progress = self.state.current_play_progress.lock().unwrap();
            let total_duration = total_samples as f32 / (sample_rate as f32 * channels as f32);
            *progress * total_duration
        };

        {
            let mut seek_start = self.state.seek_start_time.lock().unwrap();
            *seek_start = seek_start_position;
        }

        // Start consolidated progress tracking
        self.start_progress_tracking(
            sink,
            seek_start_position,
            total_samples,
            sample_rate,
            channels,
        );

        Ok(())
    }

    fn start_progress_tracking(
        &mut self,
        sink: Arc<Sink>,
        seek_start_position: f32,
        total_samples: usize,
        sample_rate: u32,
        channels: u32,
    ) {
        // Reset stop signal
        {
            let mut should_stop = self.should_stop_progress.lock().unwrap();
            *should_stop = false;
        }

        let state_clone = self.state.clone();
        let app_clone = self.app.clone();
        let should_stop_clone = Arc::clone(&self.should_stop_progress);
        let tracking_start = Instant::now();

        self.progress_thread_handle = Some(thread::spawn(move || {
            let mut last_seek_position = seek_start_position;
            let mut tracking_start = tracking_start;

            loop {
                // Check if we should stop
                {
                    let should_stop = should_stop_clone.lock().unwrap();
                    if *should_stop {
                        break;
                    }
                }

                // Check if sink is empty (but continue for looping audio)
                if sink.empty() {
                    // For looping audio, we might want to continue, but let's break for now
                    // In the future, we could check if the LoopingSamplesBuffer is still active
                    break;
                }

                // Check for seeking
                let current_seek_position = {
                    let seek_start = state_clone.seek_start_time.lock().unwrap();
                    *seek_start
                };

                // Reset tracking if seek occurred
                if (current_seek_position - last_seek_position).abs() > 0.001 {
                    tracking_start = Instant::now();
                    last_seek_position = current_seek_position;
                    println!(
                        "Seek detected! Reset tracking start time. New position: {}",
                        current_seek_position
                    );
                }

                // Calculate progress
                let elapsed = tracking_start.elapsed().as_secs_f32();
                let total_duration = total_samples as f32 / (sample_rate as f32 * channels as f32);
                let current_position = current_seek_position + elapsed;

                // For looping audio, wrap progress around
                let progress = if total_duration > 0.0 {
                    let wrapped_position = current_position % total_duration;
                    wrapped_position / total_duration
                } else {
                    0.0
                };

                // Update state and emit progress
                {
                    let mut current_progress = state_clone.current_play_progress.lock().unwrap();
                    *current_progress = progress;
                }

                println!("EMITTED PROGRESS : {}", progress);
                let _ = app_clone.emit("timeline-progress", progress);
                std::thread::sleep(Duration::from_millis(16)); // 60 FPS
            }

            // Emit end signal if not manually stopped
            let should_stop = should_stop_clone.lock().unwrap();
            if !*should_stop {
                let _ = app_clone.emit("audio-playback-ended", ());
            }
        }));
    }

    fn stop_progress_tracking(&mut self) {
        // Signal progress thread to stop
        {
            let mut should_stop = self.should_stop_progress.lock().unwrap();
            *should_stop = true;
        }

        // Wait for progress thread to finish
        if let Some(handle) = self.progress_thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn pause(&mut self) {
        println!("PAUSING");

        self.stop_progress_tracking();

        let current_song = self.state.current_song.lock().unwrap();
        if let Some(sink) = &*current_song {
            let current_progress = {
                let progress = self.state.current_play_progress.lock().unwrap();
                *progress
            };

            let current_position = {
                let guard = self.state.combined_audio.lock().unwrap();
                if let Some(ref samples) = *guard {
                    let sample_rate = 44100.0;
                    let channels = 2.0;
                    let total_duration = samples.len() as f32 / (sample_rate * channels);
                    current_progress * total_duration
                } else {
                    0.0
                }
            };

            {
                let mut seek_start = self.state.seek_start_time.lock().unwrap();
                *seek_start = current_position;
            }

            sink.stop();
            sink.clear();

            let _ = self.app.emit("timeline-progress", current_progress);
            println!(
                "Paused at position: {:.2}s (progress: {:.2})",
                current_position, current_progress
            );
        } else {
            println!("PAUSE FAILED");
        }
    }

    pub fn stop(&mut self) {
        println!("STOPPING");

        self.stop_progress_tracking();

        // Stop audio
        let current_song = self.state.current_song.lock().unwrap();
        if let Some(sink) = &*current_song {
            sink.stop();
            sink.clear();
        }

        // Reset state
        {
            let mut current_progress = self.state.current_play_progress.lock().unwrap();
            *current_progress = 0.0;
        }
        {
            let mut seek_start = self.state.seek_start_time.lock().unwrap();
            *seek_start = 0.0;
        }

        let _ = self.app.emit("timeline-progress", 0.0);
        println!("Stopped and reset to beginning");
    }

    pub fn seek_to_position(&mut self, position_seconds: f32) -> Result<(), String> {
        println!("SETTING PLAY POSITION");

        let current_song = self.state.current_song.lock().unwrap();

        if let Some(ref sink) = *current_song {
            let position_duration = Duration::from_secs_f32(position_seconds);
            println!("POSITION_DURATION: {} ", position_duration.as_secs_f32());

            match sink.try_seek(position_duration) {
                Ok(_) => {
                    println!("Successfully seeked to position: {:.2}s", position_seconds);

                    // Store the seek position for accurate progress tracking
                    {
                        let mut seek_start = self.state.seek_start_time.lock().unwrap();
                        *seek_start = position_seconds;
                    }

                    // Calculate normalized progress (0-1) based on combined audio duration
                    let total_duration = {
                        let guard = self.state.combined_audio.lock().unwrap();
                        if let Some(ref samples) = *guard {
                            let sample_rate = 44100.0;
                            let channels = 2.0;
                            samples.len() as f32 / (sample_rate * channels)
                        } else {
                            1.0 // Default fallback
                        }
                    };

                    println!("TOTAL DURATION: {}", total_duration);
                    let normalized_progress = (position_seconds / total_duration).min(1.0).max(0.0);

                    // Update the progress in app state
                    {
                        let mut progress = self.state.current_play_progress.lock().unwrap();
                        println!("NEW PROGRESS: {}", normalized_progress);
                        *progress = normalized_progress;
                    }

                    let _ = self.app.emit("timeline-progress", normalized_progress);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!(
                        "Failed to seek to position {:.2}s: {:?}",
                        position_seconds, e
                    );
                    // Fallback to stream recreation
                    drop(current_song); // Release the lock
                    return self.seek_to_position_fallback(position_seconds);
                }
            }
        } else {
            eprintln!("No audio currently playing to seek.");
            // Drop the lock before calling play_from_position
            drop(current_song);
            // If no audio is playing, start playback from the position
            return self.play_from_position(Some(position_seconds));
        }
        Ok(())
    }

    fn seek_to_position_fallback(&mut self, position_seconds: f32) -> Result<(), String> {
        // Stop current playback
        {
            let mut current_song = self.state.current_song.lock().unwrap();
            if let Some(ref sink) = *current_song {
                sink.stop();
                sink.clear();
            }
            *current_song = None;
        }

        // Use the main play method with the specified position
        self.play_from_position(Some(position_seconds))
    }

    pub fn set_volume(&self, volume: f32) {
        let current_song = self.state.current_song.lock().unwrap();
        if let Some(sink) = &*current_song {
            sink.set_volume(volume);
        }
    }

    pub fn get_current_progress(&self) -> f32 {
        let progress = self.state.current_play_progress.lock().unwrap();
        *progress
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop_progress_tracking();
    }
}
