#[macro_export]
macro_rules! epr {
    ($($arg:tt)*) => {
        eprintln!("\x1b[1;31m{}\x1b[0m", format!($($arg)*));
    };
}
