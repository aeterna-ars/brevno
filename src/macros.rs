#[macro_export]
macro_rules! error {
    ($msg:literal) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Error as u8) {
            let _ = GLOBAL_LOGGER.push_raw($crate::log::Log::encode($msg));
        }
    };
    ($($arg:tt)+) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Error as u8) {
            let _ = GLOBAL_LOGGER.write_log($crate::log::LogLevel::Error, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! warn {
    ($msg:literal) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Warn as u8) {
            let _ = GLOBAL_LOGGER.push_raw($crate::log::Log::encode($msg));
        }
    };
    ($($arg:tt)+) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Warn as u8) {
            let _ = GLOBAL_LOGGER.write_log($crate::log::LogLevel::Warn, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! info {
    ($msg:literal) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Info as u8) {
            let _ = GLOBAL_LOGGER.push_raw($crate::log::Log::encode($msg));
        }
    };
    ($($arg:tt)+) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Info as u8) {
            let _ = GLOBAL_LOGGER.write_log($crate::log::LogLevel::Info, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($msg:literal) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Debug as u8) {
            let _ = GLOBAL_LOGGER.push_raw($crate::log::Log::encode($msg));
        }
    };
    ($($arg:tt)+) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Debug as u8) {
            let _ = GLOBAL_LOGGER.write_log($crate::log::LogLevel::Debug, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! trace {
    ($msg:literal) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Trace as u8) {
            let _ = GLOBAL_LOGGER.push_raw($crate::log::Log::encode($msg));
        }
    };
    ($($arg:tt)+) => {
        if (GLOBAL_LOGGER.level() as u8) >= ($crate::log::LogLevel::Trace as u8) {
            let _ = GLOBAL_LOGGER.write_log($crate::log::LogLevel::Trace, format_args!($($arg)+));
        }
    };
}