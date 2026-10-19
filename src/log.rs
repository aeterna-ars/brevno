use core::{
    ptr,
    fmt::{self, Write},
};

use crate::buf::LogBuffer;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "[ERROR] ",
            Self::Warn => "[WARN] ",
            Self::Info => "[INFO] ",
            Self::Debug => "[DEBUG] ",
            Self::Trace => "[TRACE] ",
        }
    }
}

pub struct Log<const N: usize> {
    pub(crate) msg: [u8; N],
    pub(crate) len: usize,
}

impl<const N: usize> Log<N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Log {
            msg: [0u8; N],
            len: 0,
        }
    }

    pub fn encode(msg: &'static str) -> Self {
        let mut msg_array = [0u8; N];

        let len = msg.len().min(N);
        unsafe {
            ptr::copy_nonoverlapping(
                msg.as_ptr(),
                msg_array.as_mut_ptr(),
                len,
            );
        }

        Log {
            msg: msg_array,
            len,
        }
    }

    pub fn decode(&self) -> Result<&str, core::str::Utf8Error> {
        let valid_bytes = &self.msg[..self.len];
        str::from_utf8(valid_bytes)
    }
}

impl<const N: usize> core::fmt::Display for Log<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Ok(s) = self.decode() {
            f.write_str(s)
        } else {
            f.write_str("[Invalid UTF-8 Log]")
        }
    }
}

struct LogWriter<'a, const N: usize> {
    log: &'a mut Log<N>,
}

impl<'a, const N: usize> fmt::Write for LogWriter<'a, N> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining = N - self.log.len;
        
        let to_copy = s.len().min(remaining);
        if to_copy == 0 {
            return Ok(());
        }

        unsafe {
            ptr::copy_nonoverlapping(
                s.as_ptr(),
                self.log.msg.as_mut_ptr().add(self.log.len),
                to_copy,
            );
        }

        self.log.len += to_copy;
        Ok(())
    }
}

pub struct Logger<B: LogBuffer> {
    buffer: B,
    level: LogLevel,
}

impl<B: LogBuffer<Item = Log<NL>>, const NL: usize> Logger<B> {
    pub const fn init(buf: B, level: LogLevel) -> Self {
        Self {
            buffer: buf,
            level,
        }
    }

    #[inline(always)]
    pub fn level(&self) -> LogLevel {
        self.level
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    #[inline(always)]
    pub fn push_raw(&self, log: B::Item) -> Result<(), B::Item> {
        self.buffer.push(log)
    }

    #[inline(always)]
    pub fn pop(&self) -> Option<B::Item> {
        self.buffer.pop()
    }

    #[inline(always)]
    pub fn write_log(&self, level: LogLevel, args: fmt::Arguments) -> bool {
        if level > self.level {
            return false;
        }
        if self.buffer.is_full() {
            return false;
        }

        let mut log: Log<NL> = Log::new();

        let mut writer = LogWriter { log: &mut log };
        let _ = writer.write_str(level.as_str());
        let _ = fmt::write(&mut writer, args);

        self.buffer.push(log).is_ok()
    }
}

#[cfg(test)]
mod log_test {
    extern crate std;

    use super::*;

    use std::*;

    #[test]
    fn main_thread_formatting() {
        pub static GLOBAL_LOGGER: Logger<crate::buf::SpscRingBuf<Log<16>, 1024>> =
            Logger::init(crate::buf::SpscRingBuf::new(), LogLevel::Info);

        let _ = crate::info!("Ferris");
        let _ = crate::warn!("the");
        let _ = crate::error!("crab");

        println!("\n");
        println!("{}", GLOBAL_LOGGER.pop().unwrap().decode().unwrap());
        println!("{}", GLOBAL_LOGGER.pop().unwrap().decode().unwrap());
        println!("{}", GLOBAL_LOGGER.pop().unwrap().decode().unwrap());
        println!("\n");
    }

    use std::sync::Arc;

    #[test]
    fn threads_formatting() {
        let logger: Arc<Logger<crate::buf::MpscRingBuf<Log<16>, 1024>>> =
            Arc::new(Logger::init(crate::buf::MpscRingBuf::new(), LogLevel::Info));

        let logger_1 = Arc::clone(&logger);
        let handle_1 = std::thread::spawn(move || {
            logger_1.write_log(LogLevel::Info, format_args!("1"))
        });

        let logger_2 = Arc::clone(&logger);
        let handle_2 = std::thread::spawn(move || {
            logger_2.write_log(LogLevel::Info, format_args!("1"))
        });

        let logger_3 = Arc::clone(&logger);
        let handle_3 = std::thread::spawn(move || {
            logger_3.write_log(LogLevel::Info, format_args!("1"))
        });

        handle_1.join().unwrap();
        handle_2.join().unwrap();
        handle_3.join().unwrap();

        println!("\n");
        println!("{}", logger.pop().unwrap().decode().unwrap());
        println!("{}", logger.pop().unwrap().decode().unwrap());
        println!("{}", logger.pop().unwrap().decode().unwrap());
        println!("\n");
    }

    #[test]
    fn function_formatting() {
        let logger: Logger<crate::buf::SpscRingBuf<Log<16>, 1024>> =
            Logger::init(crate::buf::SpscRingBuf::new(), LogLevel::Info);

        let _ = logger.write_log(LogLevel::Info, format_args!("Ferris"));
        let _ = logger.write_log(LogLevel::Info, format_args!("the"));
        let _ = logger.write_log(LogLevel::Info, format_args!("crab"));

        println!("\n");
        println!("{}", logger.pop().unwrap().decode().unwrap());
        println!("{}", logger.pop().unwrap().decode().unwrap());
        println!("{}", logger.pop().unwrap().decode().unwrap());
        println!("\n");
    }

    use log::{info, Log as TraitLog, Metadata, Record};
    use std::time::Instant;

    struct DummyLogger;
    impl TraitLog for DummyLogger {
        fn enabled(&self, _: &Metadata) -> bool { true }
        fn log(&self, record: &Record) {
            let _ = format!("{}", record.args());
        }
        fn flush(&self) {}
    }

    static LOGGER: DummyLogger = DummyLogger;

    static INIT_LOG: std::sync::Once = std::sync::Once::new();

    use std::hint::black_box;

    #[test]
    fn benchmark_vs_log() {
        pub static GLOBAL_LOGGER: Logger<crate::buf::SpscRingBuf<Log<16>, 1024>> =
            Logger::init(crate::buf::SpscRingBuf::new(), LogLevel::Info);

        INIT_LOG.call_once(|| {
            log::set_logger(&LOGGER).unwrap();
            log::set_max_level(log::LevelFilter::Info);
        });

        let iterations = 100_000;
        let test_msg = "Value: 123";

        for _ in 0..5_000 {
            let _ = black_box(crate::info!("{}", black_box(test_msg)));
            let _ = black_box(info!("{}", black_box(test_msg)));
        }

        let start_brevno = Instant::now();
        for _ in 0..iterations {
            let res = crate::info!("{}", black_box(test_msg));
            let _ = black_box(res); 
        }
        let duration_brevno = start_brevno.elapsed();

        let start_log = Instant::now();
        for _ in 0..iterations {
            let res = info!("{}", black_box(test_msg));
            black_box(res);
        }
        let duration_log = start_log.elapsed();

        let nanos_brevno = duration_brevno.as_nanos();
        let nanos_log = duration_log.as_nanos();

        println!("\n");
        println!("=== BENCHMARK ===");
        println!("BREVNO: {:?}", duration_brevno);
        println!("LOG:    {:?}", duration_log);
        println!("BREVNO IS {} TIMES FASTER!", nanos_log / nanos_brevno);
        println!("=================");
        println!("\n");
    }
}