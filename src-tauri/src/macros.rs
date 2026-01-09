#[macro_export]
macro_rules! epr {
    ($($arg:tt)*) => {
        eprintln!("\x1b[1;31m{}\x1b[0m", format!($($arg)*));
    };
}

/// Macro to send events to a channel with proper error handling and logging
/// Usage: send_channel_event!(channel, event_data);
#[macro_export]
macro_rules! send_channel_event {
    ($channel:expr, $event:expr) => {
        if let Err(e) = $channel.send($event) {
            eprintln!("Failed to send channel event: {}", e);
        }
    };
}

#[macro_export]
macro_rules! emit_logged {
    ($app:expr, $event:expr, $payload:expr) => {{
        use tauri::Manager;
        use $crate::logging::{FileLocation, LogLevel, LogSystem};

        // Construct absolute file path
        let workspace_root = env!("CARGO_MANIFEST_DIR");
        let relative_path = file!();
        let abs_path = if relative_path.starts_with("src") {
            // Path is relative to workspace root
            format!("{}\\{}", workspace_root, relative_path)
        } else if std::path::Path::new(relative_path).is_absolute() {
            relative_path.to_string()
        } else {
            // Fallback: try to construct from workspace
            format!("{}\\{}", workspace_root, relative_path)
        };

        let line_num = line!();

        // Integration with backend logging system
        // Retrieve LoggingService from app state and log the event emission
        if let Some(logging_service) =
            $app.try_state::<std::sync::Arc<std::sync::Mutex<$crate::logging::LoggingService>>>()
        {
            if let Ok(logger) = logging_service.lock() {
                // Try to serialize payload for the log data field
                let data = serde_json::to_value(&$payload).ok();

                // Create FileLocation struct with file path and line number
                let file_location = FileLocation {
                    file_path: abs_path.clone(),
                    line_number: Some(line_num),
                };

                logger.log_with_location(
                    LogSystem::EventEmits,
                    LogLevel::Debug,
                    &format!("Event emitted: {}", $event),
                    Some(module_path!()),
                    data,
                    Some(file_location),
                );
            }
        }

        // Actually emit the event via Tauri
        let _ = $app.emit($event, $payload);
    }};
}
