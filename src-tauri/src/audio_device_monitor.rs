use crate::audio_device_manager::{AudioDeviceManager, DeviceChangeMessage};
use crate::state::AppState;
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct AudioDeviceMonitor {
    app_handle: AppHandle,
    app_state: Arc<AppState>,
    is_running: Arc<Mutex<bool>>,
    monitor_thread: Option<thread::JoinHandle<()>>,
}

impl AudioDeviceMonitor {
    pub fn new(app_handle: AppHandle, app_state: Arc<AppState>) -> Self {
        Self {
            app_handle,
            app_state,
            is_running: Arc::new(Mutex::new(false)),
            monitor_thread: None,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        if *self.is_running.lock().unwrap() {
            return Ok(()); // Already running
        }

        *self.is_running.lock().unwrap() = true;

        let app_handle = self.app_handle.clone();
        let app_state = self.app_state.clone();
        let is_running = self.is_running.clone();

        let handle = thread::spawn(move || {
            info!("Audio device monitor started");

            // Get device change receiver from device manager
            let device_change_receiver =
                if let Some(ref device_manager) = app_state.audio_device_manager {
                    device_manager.get_device_change_receiver()
                } else {
                    error!("Audio device manager not available");
                    return;
                };

            while *is_running.lock().unwrap() {
                // Check for device change messages
                let receiver = device_change_receiver.lock().unwrap();
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(message) => {
                        drop(receiver); // Release lock before processing
                        Self::handle_device_change_message(message, &app_handle, &app_state);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        drop(receiver);
                        // No message, continue monitoring
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        error!("Device change receiver disconnected");
                        break;
                    }
                }

                // Periodic device availability check (every 5 seconds)
                thread::sleep(Duration::from_millis(5000));
                if let Some(ref device_manager) = app_state.audio_device_manager {
                    if let Err(e) = device_manager.refresh_devices() {
                        warn!("Failed to refresh devices during monitoring: {}", e);
                    }
                }
            }

            info!("Audio device monitor stopped");
        });

        self.monitor_thread = Some(handle);
        info!("Audio device monitor thread started");
        Ok(())
    }

    pub fn stop(&mut self) {
        *self.is_running.lock().unwrap() = false;

        if let Some(handle) = self.monitor_thread.take() {
            if let Err(e) = handle.join() {
                error!("Failed to join audio device monitor thread: {:?}", e);
            } else {
                info!("Audio device monitor thread stopped successfully");
            }
        }
    }

    fn handle_device_change_message(
        message: DeviceChangeMessage,
        app_handle: &AppHandle,
        app_state: &Arc<AppState>,
    ) {
        match message {
            DeviceChangeMessage::ChangeDevice(device_name) => {
                info!("Handling device change to: {}", device_name);
                Self::handle_device_change(device_name, app_handle, app_state);
            }
            DeviceChangeMessage::RefreshDevices => {
                info!("Refreshing devices on request");
                if let Some(ref device_manager) = app_state.audio_device_manager {
                    if let Err(e) = device_manager.refresh_devices() {
                        error!("Failed to refresh devices: {}", e);
                    }
                }
            }
            DeviceChangeMessage::DeviceDisconnected(device_name) => {
                warn!("Device disconnected: {}", device_name);
                Self::handle_device_disconnection(device_name, app_handle, app_state);
            }
            DeviceChangeMessage::SetFollowSystem(follow) => {
                info!("Follow system setting changed to: {}", follow);
                if let Err(e) = app_handle.emit("follow-system-changed", follow) {
                    error!("Failed to emit follow system change: {}", e);
                }
            }
        }
    }

    fn handle_device_change(
        device_name: String,
        app_handle: &AppHandle,
        app_state: &Arc<AppState>,
    ) {
        // Check if audio is currently playing
        let is_playing = {
            let current_song = app_state.current_song.lock().unwrap();
            current_song.is_some()
        };

        if is_playing {
            info!("Audio is playing during device change, attempting seamless transition");

            // Get current playback position before stopping
            let current_progress = *app_state.current_play_progress.lock().unwrap();

            // Pause current playback
            if let Some(ref sink) = *app_state.current_song.lock().unwrap() {
                sink.pause();
            }

            // Small delay to allow current stream to clean up
            thread::sleep(Duration::from_millis(50));

            // Clear current sink
            *app_state.current_song.lock().unwrap() = None;

            // Emit device change event to frontend
            if let Err(e) = app_handle.emit("audio-device-changed-during-playback", &device_name) {
                error!("Failed to emit device change event: {}", e);
            }

            // The audio manager should pick up the new device when playback resumes
            info!("Prepared for device change to: {}", device_name);

            // Optionally, restart playback automatically with new device
            // This would require access to the AudioManager, which could be added to AppState
        } else {
            info!("No audio playing during device change to: {}", device_name);
        }

        // Emit successful device change
        if let Err(e) = app_handle.emit("audio-device-changed", &device_name) {
            error!("Failed to emit device change success: {}", e);
        }
    }

    fn handle_device_disconnection(
        device_name: String,
        app_handle: &AppHandle,
        app_state: &Arc<AppState>,
    ) {
        warn!("Handling device disconnection: {}", device_name);

        // Check if the disconnected device was the current one
        let is_current_device = if let Some(ref device_manager) = app_state.audio_device_manager {
            device_manager
                .get_current_device_name()
                .map_or(false, |current| current == device_name)
        } else {
            false
        };

        if is_current_device {
            error!("Current audio device '{}' was disconnected", device_name);

            // Stop current playback if any
            if let Some(ref sink) = *app_state.current_song.lock().unwrap() {
                sink.stop();
            }
            *app_state.current_song.lock().unwrap() = None;

            // Emit disconnection event to frontend
            if let Err(e) = app_handle.emit("current-audio-device-disconnected", &device_name) {
                error!("Failed to emit device disconnection event: {}", e);
            }

            // If following system, try to switch to default
            if let Some(ref device_manager) = app_state.audio_device_manager {
                if device_manager.is_following_system() {
                    // Device manager should automatically handle fallback to default
                    info!("Following system setting enabled, will fallback to default device");
                } else {
                    warn!("Not following system, user will need to manually select a new device");
                    // Emit event asking user to select new device
                    if let Err(e) = app_handle.emit("audio-device-selection-required", ()) {
                        error!("Failed to emit device selection required event: {}", e);
                    }
                }
            }
        }
    }
}

impl Drop for AudioDeviceMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}
