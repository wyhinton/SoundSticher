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
