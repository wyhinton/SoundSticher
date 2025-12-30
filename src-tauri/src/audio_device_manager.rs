use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{default_host, Device, SampleFormat, StreamConfig, SupportedStreamConfig};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub sample_rates: Vec<u32>,
    pub channels: u32,
    pub sample_format: String,
}

#[derive(Debug, Clone)]
pub struct DeviceWithConfig {
    pub device: Device,
    pub config: Vec<SupportedStreamConfig>,
    pub info: AudioDeviceInfo,
}

#[derive(Debug, Clone)]
pub enum DeviceChangeMessage {
    ChangeDevice(String),
    RefreshDevices,
    DeviceDisconnected(String),
    SetFollowSystem(bool),
}

pub struct AudioDeviceManager {
    // Device caching to avoid accessing current audio device during playback
    cached_devices: Arc<Mutex<Option<Vec<DeviceWithConfig>>>>,

    // Current selected device name
    current_device_name: Arc<Mutex<Option<String>>>,
    previous_device_name: Arc<Mutex<String>>,

    // Communication channels
    device_change_sender: Sender<DeviceChangeMessage>,
    device_change_receiver: Arc<Mutex<Receiver<DeviceChangeMessage>>>,

    // Settings
    follow_system_output: Arc<Mutex<bool>>,

    // App handle for frontend communication
    app_handle: AppHandle,
}

impl AudioDeviceManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            cached_devices: Arc::new(Mutex::new(None)),
            current_device_name: Arc::new(Mutex::new(None)),
            previous_device_name: Arc::new(Mutex::new(String::new())),
            device_change_sender: sender,
            device_change_receiver: Arc::new(Mutex::new(receiver)),
            follow_system_output: Arc::new(Mutex::new(true)),
            app_handle,
        }
    }

    /// Enumerate all available audio output devices with caching
    pub fn enumerate_devices(&self, force_refresh: bool) -> Result<Vec<DeviceWithConfig>, String> {
        let mut cached = self.cached_devices.lock().unwrap();

        // Return cached devices unless forced refresh or cache is empty
        if !force_refresh && cached.is_some() {
            return Ok(cached.as_ref().unwrap().clone());
        }

        info!("Enumerating audio output devices");

        let host = default_host();
        let default_device = host.default_output_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_else(|| "Unknown Default".to_string());

        let devices: Result<Vec<DeviceWithConfig>, String> = host
            .output_devices()
            .map_err(|e| format!("Failed to enumerate devices: {}", e))?
            .filter_map(|device| {
                let device_name = device.name().ok()?;

                // Get supported output configurations
                let supported_configs: Vec<SupportedStreamConfig> =
                    device.supported_output_configs().ok()?.collect();

                if supported_configs.is_empty() {
                    warn!(
                        "Device '{}' has no supported output configurations",
                        device_name
                    );
                    return None;
                }

                // Create device info
                let sample_rates: Vec<u32> = supported_configs
                    .iter()
                    .flat_map(|config| {
                        let min_rate = config.min_sample_rate().0;
                        let max_rate = config.max_sample_rate().0;

                        // Common sample rates within the supported range
                        vec![44100, 48000, 88200, 96000, 192000]
                            .into_iter()
                            .filter(|&rate| rate >= min_rate && rate <= max_rate)
                    })
                    .collect();

                let info = AudioDeviceInfo {
                    name: device_name.clone(),
                    is_default: device_name == default_name,
                    sample_rates: sample_rates
                        .into_iter()
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect(),
                    channels: supported_configs
                        .iter()
                        .map(|c| c.channels())
                        .max()
                        .unwrap_or(2) as u32,
                    sample_format: format!("{:?}", supported_configs[0].sample_format()),
                };

                Some(DeviceWithConfig {
                    device,
                    config: supported_configs,
                    info,
                })
            })
            .collect();

        match devices {
            Ok(device_list) => {
                info!("Found {} audio output devices", device_list.len());
                *cached = Some(device_list.clone());

                // Emit updated device list to frontend
                let device_infos: Vec<AudioDeviceInfo> =
                    device_list.iter().map(|d| d.info.clone()).collect();

                if let Err(e) = self.app_handle.emit("audio-devices-updated", &device_infos) {
                    error!("Failed to emit device list update: {}", e);
                }

                Ok(device_list)
            }
            Err(e) => {
                error!("Failed to enumerate devices: {}", e);
                Err(e)
            }
        }
    }

    /// Get the currently selected audio device
    pub fn get_current_device(&self) -> Result<Option<DeviceWithConfig>, String> {
        let device_name = self.current_device_name.lock().unwrap();

        if device_name.is_none() {
            // No device selected, try to use system default
            let host = default_host();
            if let Some(default_device) = host.default_output_device() {
                if let Ok(name) = default_device.name() {
                    drop(device_name);
                    return self.get_device_by_name(&name);
                }
            }
            return Ok(None);
        }

        let name = device_name.as_ref().unwrap().clone();
        drop(device_name);

        self.get_device_by_name(&name)
    }

    /// Get device by name from cached devices
    pub fn get_device_by_name(&self, name: &str) -> Result<Option<DeviceWithConfig>, String> {
        let devices = self.enumerate_devices(false)?;

        Ok(devices.into_iter().find(|d| d.info.name == name))
    }

    /// Change the active audio device
    pub fn change_audio_device(&self, device_name: String) -> Result<(), String> {
        info!("Changing audio device to: {}", device_name);

        // Validate device exists
        if self.get_device_by_name(&device_name)?.is_none() {
            return Err(format!("Device '{}' not found", device_name));
        }

        // Store previous device name
        {
            let current = self.current_device_name.lock().unwrap();
            if let Some(ref current_name) = *current {
                *self.previous_device_name.lock().unwrap() = current_name.clone();
            }
        }

        // Update current device
        *self.current_device_name.lock().unwrap() = Some(device_name.clone());

        // Send change message
        if let Err(e) = self
            .device_change_sender
            .send(DeviceChangeMessage::ChangeDevice(device_name.clone()))
        {
            error!("Failed to send device change message: {}", e);
            return Err(format!("Failed to change device: {}", e));
        }

        // Emit to frontend
        if let Err(e) = self.app_handle.emit("audio-device-changed", &device_name) {
            error!("Failed to emit device change event: {}", e);
        }

        info!("Successfully changed audio device to: {}", device_name);
        Ok(())
    }

    /// Set whether to follow system default output device
    pub fn set_follow_system(&self, follow: bool) -> Result<(), String> {
        *self.follow_system_output.lock().unwrap() = follow;

        if let Err(e) = self
            .device_change_sender
            .send(DeviceChangeMessage::SetFollowSystem(follow))
        {
            error!("Failed to send follow system message: {}", e);
            return Err(format!("Failed to set follow system: {}", e));
        }

        // If enabling follow system, switch to default device
        if follow {
            let host = default_host();
            if let Some(default_device) = host.default_output_device() {
                if let Ok(name) = default_device.name() {
                    return self.change_audio_device(name);
                }
            }
        }

        // Emit to frontend
        if let Err(e) = self.app_handle.emit("follow-system-changed", follow) {
            error!("Failed to emit follow system event: {}", e);
        }

        Ok(())
    }

    /// Get the device change receiver for the audio playback thread
    pub fn get_device_change_receiver(&self) -> Arc<Mutex<Receiver<DeviceChangeMessage>>> {
        self.device_change_receiver.clone()
    }

    /// Refresh device list and check for changes
    pub fn refresh_devices(&self) -> Result<Vec<AudioDeviceInfo>, String> {
        let devices = self.enumerate_devices(true)?;

        // Check if current device is still available
        if let Some(current_name) = self.current_device_name.lock().unwrap().as_ref() {
            let device_exists = devices.iter().any(|d| &d.info.name == current_name);

            if !device_exists {
                warn!("Current device '{}' is no longer available", current_name);

                // Emit device disconnected event
                if let Err(e) = self
                    .app_handle
                    .emit("audio-device-disconnected", current_name)
                {
                    error!("Failed to emit device disconnected event: {}", e);
                }

                // Fall back to default device if following system
                if *self.follow_system_output.lock().unwrap() {
                    let host = default_host();
                    if let Some(default_device) = host.default_output_device() {
                        if let Ok(name) = default_device.name() {
                            if let Err(e) = self.change_audio_device(name) {
                                error!("Failed to fallback to default device: {}", e);
                            }
                        }
                    }
                } else {
                    // Clear current device if not following system
                    *self.current_device_name.lock().unwrap() = None;
                }
            }
        }

        Ok(devices.into_iter().map(|d| d.info).collect())
    }

    /// Get current device name
    pub fn get_current_device_name(&self) -> Option<String> {
        self.current_device_name.lock().unwrap().clone()
    }

    /// Check if following system output
    pub fn is_following_system(&self) -> bool {
        *self.follow_system_output.lock().unwrap()
    }

    /// Create audio output stream with specific device
    pub fn create_output_stream_with_device(
        &self,
        device_name: Option<&str>,
    ) -> Result<(cpal::Stream, Arc<Mutex<Vec<f32>>>), String> {
        let device = if let Some(name) = device_name {
            self.get_device_by_name(name)?
                .ok_or_else(|| format!("Device '{}' not found", name))?
        } else {
            self.get_current_device()?
                .ok_or_else(|| "No audio device available".to_string())?
        };

        // Find best supported configuration
        let config = device
            .config
            .iter()
            .find(|c| c.channels() >= 2 && c.sample_format() == SampleFormat::F32)
            .or_else(|| device.config.first())
            .ok_or("No supported audio configuration found")?;

        let stream_config = StreamConfig {
            channels: config.channels(),
            sample_rate: config.default_sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        let stream = device
            .device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut buf = buffer_clone.lock().unwrap();
                    if !buf.is_empty() {
                        let len = data.len().min(buf.len());
                        data[..len].copy_from_slice(&buf[..len]);
                        buf.drain(..len);
                    } else {
                        // Fill with silence if no data
                        data.fill(0.0);
                    }
                },
                |err| error!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| format!("Failed to create audio stream: {}", e))?;

        info!(
            "Created audio output stream for device: {}",
            device.info.name
        );

        Ok((stream, buffer))
    }
}
