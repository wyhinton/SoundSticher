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
        use $crate::logging::{LogLevel, LogSystem};

        // Integration with backend logging system
        // Retrieve LoggingService from app state and log the event emission
        if let Some(logging_service) =
            $app.try_state::<std::sync::Arc<std::sync::Mutex<$crate::logging::LoggingService>>>()
        {
            if let Ok(logger) = logging_service.lock() {
                // Try to serialize payload for the log data field
                let data = serde_json::to_value(&$payload).ok();

                logger.log(
                    LogSystem::EventEmits,
                    LogLevel::Debug,
                    &format!("Event emitted: {} at {}:{}", $event, file!(), line!()),
                    Some(module_path!()),
                    data,
                );
            }
        }

        // Actually emit the event via Tauri
        let _ = $app.emit($event, $payload);
    }};
}
