use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Represents different backend systems that can log
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LogSystem {
    Encoder,
    Combine,
    Playback,
    Sorting,
    Cook,
    Graph,
    Waveform,
    Duration,  // Add duration system
    Timeline,  // Add timeline system
    Operation, // Add operation system
    EventEmits,
}

/// A log message that will be sent to the frontend
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogMessage {
    pub timestamp: u64,
    pub level: LogLevel,
    pub system: LogSystem,
    pub category: Option<String>,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub file_location: Option<FileLocation>,
}

/// File location information for opening in editor
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileLocation {
    pub file_path: String,
    pub line_number: Option<u32>,
}

/// Configuration for which systems should log
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoggingConfig {
    pub encoder_enabled: bool,
    pub combine_enabled: bool,
    pub playback_enabled: bool,
    pub sorting_enabled: bool,
    pub waveform_enabled: bool,
    pub duration_enabled: bool,  // Add duration logging
    pub timeline_enabled: bool,  // Add timeline logging
    pub operation_enabled: bool, // Add operation logging
    pub console_output: bool,    // Whether to also print to console
    pub event_emits_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            encoder_enabled: false,
            combine_enabled: false,
            playback_enabled: false,
            sorting_enabled: false,
            waveform_enabled: false,
            duration_enabled: false,  // Default off for duration
            timeline_enabled: false,  // Default off for timeline
            operation_enabled: false, // Default off for operations
            console_output: true,
            event_emits_enabled: false,
        }
    }
}

/// Main logging service that can be shared across the app
#[derive(Debug)]
pub struct LoggingService {
    config: Arc<Mutex<LoggingConfig>>,
    app_handle: Option<AppHandle>,
}

impl LoggingService {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(LoggingConfig::default())),
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    pub fn update_config(&self, config: LoggingConfig) {
        if let Ok(mut current_config) = self.config.lock() {
            *current_config = config;
        }
    }

    pub fn get_config(&self) -> LoggingConfig {
        self.config
            .lock()
            .map(|config| config.clone())
            .unwrap_or_default()
    }

    pub fn log(
        &self,
        system: LogSystem,
        level: LogLevel,
        message: &str,
        category: Option<&str>,
        data: Option<serde_json::Value>,
    ) {
        self.log_with_location(system, level, message, category, data, None);
    }

    pub fn log_with_location(
        &self,
        system: LogSystem,
        level: LogLevel,
        message: &str,
        category: Option<&str>,
        data: Option<serde_json::Value>,
        file_location: Option<FileLocation>,
    ) {
        let config = self.get_config();

        // Check if this system should log
        let should_log = match system {
            LogSystem::Encoder => config.encoder_enabled,
            LogSystem::Combine => config.combine_enabled,
            LogSystem::Playback => config.playback_enabled,
            LogSystem::Cook => false,  // Future system, default to off
            LogSystem::Graph => false, // Future system, default to off
            LogSystem::Sorting => config.sorting_enabled,
            LogSystem::Waveform => config.waveform_enabled,
            LogSystem::Duration => config.duration_enabled, // Add duration check
            LogSystem::Timeline => config.timeline_enabled, // Add timeline check
            LogSystem::Operation => config.operation_enabled, // Add operation check
            LogSystem::EventEmits => config.event_emits_enabled,
        };

        if !should_log {
            return;
        }

        let log_msg = LogMessage {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            level: level.clone(),
            system: system.clone(),
            category: category.map(|s| s.to_string()),
            message: message.to_string(),
            data,
            file_location,
        };

        // Print to console if enabled
        if config.console_output {
            let level_emoji = match level {
                LogLevel::Debug => "🔍",
                LogLevel::Info => "ℹ️",
                LogLevel::Warning => "⚠️",
                LogLevel::Error => "❌",
            };

            let system_name = match system {
                LogSystem::Encoder => "ENCODER",
                LogSystem::Combine => "COMBINE",
                LogSystem::Playback => "PLAYBACK",
                LogSystem::Sorting => "SORTING",
                LogSystem::Cook => "COOK",
                LogSystem::Graph => "GRAPH",
                LogSystem::Waveform => "WAVEFORM",
                LogSystem::Duration => "DURATION",
                LogSystem::Timeline => "TIMELINE",
                LogSystem::Operation => "OPERATION",
                LogSystem::EventEmits => "EVENT EMITS",
            };

            let category_str = category.map(|c| format!("[{}] ", c)).unwrap_or_default();
            println!(
                "{} {} {}{}",
                level_emoji, system_name, category_str, message
            );
        }

        // Send to frontend
        if let Some(app_handle) = &self.app_handle {
            if let Err(e) = app_handle.emit("backend-log", &log_msg) {
                eprintln!("Failed to send log message to frontend: {}", e);
            }
        }
    }

    pub fn debug(&self, system: LogSystem, message: &str, category: Option<&str>) {
        self.log(system, LogLevel::Debug, message, category, None);
    }

    pub fn info(&self, system: LogSystem, message: &str, category: Option<&str>) {
        self.log(system, LogLevel::Info, message, category, None);
    }

    pub fn warning(&self, system: LogSystem, message: &str, category: Option<&str>) {
        self.log(system, LogLevel::Warning, message, category, None);
    }

    pub fn error(&self, system: LogSystem, message: &str, category: Option<&str>) {
        self.log(system, LogLevel::Error, message, category, None);
    }

    pub fn info_with_data(
        &self,
        system: LogSystem,
        message: &str,
        category: Option<&str>,
        data: serde_json::Value,
    ) {
        self.log(system, LogLevel::Info, message, category, Some(data));
    }

    pub fn debug_with_data(
        &self,
        system: LogSystem,
        message: &str,
        category: Option<&str>,
        data: serde_json::Value,
    ) {
        self.log(system, LogLevel::Debug, message, category, Some(data));
    }
}

// Convenience macros for logging
#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $system:expr, $message:expr) => {
        $logger.debug($system, $message, None);
    };
    ($logger:expr, $system:expr, $category:expr, $message:expr) => {
        $logger.debug($system, $message, Some($category));
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $system:expr, $message:expr) => {
        $logger.info($system, $message, None);
    };
    ($logger:expr, $system:expr, $category:expr, $message:expr) => {
        $logger.info($system, $message, Some($category));
    };
}

#[macro_export]
macro_rules! log_warning {
    ($logger:expr, $system:expr, $message:expr) => {
        $logger.warning($system, $message, None);
    };
    ($logger:expr, $system:expr, $category:expr, $message:expr) => {
        $logger.warning($system, $message, Some($category));
    };
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $system:expr, $message:expr) => {
        $logger.error($system, $message, None);
    };
    ($logger:expr, $system:expr, $category:expr, $message:expr) => {
        $logger.error($system, $message, Some($category));
    };
}

// Timeline-specific logging macros
#[macro_export]
macro_rules! timeline_debug {
    ($logger:expr, $message:expr) => {
        $logger.debug($crate::logging::LogSystem::Timeline, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.debug(
            crate::logging::LogSystem::Timeline,
            $message,
            Some($category),
        );
    };
}

#[macro_export]
macro_rules! timeline_info {
    ($logger:expr, $message:expr) => {
        $logger.info($crate::logging::LogSystem::Timeline, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.info(
            crate::logging::LogSystem::Timeline,
            $message,
            Some($category),
        );
    };
}

#[macro_export]
macro_rules! timeline_warning {
    ($logger:expr, $message:expr) => {
        $logger.warning($crate::logging::LogSystem::Timeline, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.warning(
            crate::logging::LogSystem::Timeline,
            $message,
            Some($category),
        );
    };
}

#[macro_export]
macro_rules! timeline_error {
    ($logger:expr, $message:expr) => {
        $logger.error($crate::logging::LogSystem::Timeline, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.error(
            crate::logging::LogSystem::Timeline,
            $message,
            Some($category),
        );
    };
}

// Operation-specific logging macros
#[macro_export]
macro_rules! operation_debug {
    ($logger:expr, $message:expr) => {
        $logger.debug($crate::logging::LogSystem::Operation, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.debug(
            crate::logging::LogSystem::Operation,
            $message,
            Some($category),
        );
    };
}

#[macro_export]
macro_rules! operation_info {
    ($logger:expr, $message:expr) => {
        $logger.info($crate::logging::LogSystem::Operation, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.info(
            crate::logging::LogSystem::Operation,
            $message,
            Some($category),
        );
    };
}

#[macro_export]
macro_rules! operation_warning {
    ($logger:expr, $message:expr) => {
        $logger.warning($crate::logging::LogSystem::Operation, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.warning(
            crate::logging::LogSystem::Operation,
            $message,
            Some($category),
        );
    };
}

#[macro_export]
macro_rules! operation_error {
    ($logger:expr, $message:expr) => {
        $logger.error($crate::logging::LogSystem::Operation, $message, None);
    };
    ($logger:expr, $category:expr, $message:expr) => {
        $logger.error(
            crate::logging::LogSystem::Operation,
            $message,
            Some($category),
        );
    };
}
