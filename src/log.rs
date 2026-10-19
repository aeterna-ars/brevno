use core::{
    ptr,
    fmt::{self, Write},
};

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

struct LogWriter<'a, const N: usize> {
    log: &'a mut Log<N>,
}

impl<'a, const N: usize> fmt::Write for LogWriter<'a, N> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let remaining = N - self.log.len;
        
        let to_copy = bytes.len().min(remaining);
        if to_copy == 0 {
            return Ok(());
        }

        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.log.msg.as_mut_ptr().add(self.log.len),
                to_copy,
            );
        }

        self.log.len += to_copy;
        Ok(())
    }
}

pub trait LogBuffer<T> {
    fn push(&self, item: T) -> Result<(), T>;
    fn pop(&self) -> Option<T>;
    fn is_full(&self) -> bool;
}

impl<const NL: usize> LogBuffer<Log<NL>> for buffers::SpscRingBuf<Log<NL>, 1024> {
    fn push(&self, item: Log<NL>) -> Result<(), NL> {
        self.push(item).map_err(|_| "Full")
    }
    fn pop(&self) -> Option<Log<NL>> { self.pop() }
    fn is_full(&self) -> bool { self.is_full() }
}

impl<const NL: usize> LogBuffer<Log<NL>> for buffers::MpscRingBuf<Log<NL>, 1024> {
    fn push(&self, item: Log<NL>) -> Result<(), T> {
        self.push(item).map_err(|_| "Full")
    }
    fn pop(&self) -> Option<Log<NL>> { self.pop() }
    fn is_full(&self) -> bool { self.is_full() }
}

impl<const NL: usize> LogBuffer<Log<NL>> for buffers::MpmcRingBuf<Log<NL>, 1024> {
    fn push(&self, item: Log<NL>) -> Result<(), &'static str> {
        self.push(item).map_err(|_| "Full")
    }
    fn pop(&self) -> Option<Log<NL>> { self.pop() }
    fn is_full(&self) -> bool { self.is_full() }
}

pub struct Logger<B, const NL: usize> {
    buffer: B,
    level: LogLevel,
}

impl<const NB: usize, const NL: usize> Logger<NB, NL> {
    pub const fn init(level: LogLevel) -> Self {
        Self {
            buffer: buffers::SpscRingBuf::new(),
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
    pub fn push_raw(&self, log: Log<NL>) -> Result<(), &'static str> {
        if self.buffer.is_full() {
            return Err("Buffer is full");
        }
        let _ = self.buffer.push(log);
        Ok(())
    }

    #[inline(always)]
    pub fn write_log(&self, level: LogLevel, args: fmt::Arguments) -> Result<(), &'static str> {
        if self.buffer.is_full() {
            return Err("Buffer is full");
        }

        let mut log = Log::new();

        let mut writer = LogWriter { log: &mut log };
        let _ = writer.write_str(level.as_str());
        let _ = fmt::write(&mut writer, args);

        let _ = self.buffer.push(log);
        Ok(())
    }

    #[inline(always)]
    pub fn read_log(&self) -> Option<Log<NL>> { 
        if self.buffer.is_empty() { 
            return None; 
        } 
        self.buffer.pop() 
    }
}

#[cfg(test)]
mod log_test {
    use crate::init_global_logger;
    use super::*;

    #[test]
    fn main_thread_formatting() {
        crate::init_global_logger!(1024, 32, LogLevel::Info);

        let _ = crate::info!("Ferris");
        let _ = crate::warn!("the");
        let _ = crate::error!("crab");

        println!("\n");
        println!("{}", GLOBAL_LOGGER.read_log().unwrap().decode().unwrap());
        println!("{}", GLOBAL_LOGGER.read_log().unwrap().decode().unwrap());
        println!("{}", GLOBAL_LOGGER.read_log().unwrap().decode().unwrap());
        println!("\n");
    }

    use std::sync::Arc;

    #[test]
    fn threads_formatting() {
        let logger: Arc<Logger<32, 32>> = Arc::new(Logger::init(LogLevel::Info));

        let logger_1 = Arc::clone(&logger);
        let thread_1 = std::thread::spawn(move || {
            logger.write_log(LogLevel::Info, format_args!("1"))
        });

        let logger_2 = Arc::clone(&logger);
        let thread_2 = std::thread::spawn(move || {
            logger.write_log(LogLevel::Info, format_args!("1"))
        });

        let logger_3 = Arc::clone(&logger);
        let thread_3 = std::thread::spawn(move || {
            logger.write_log(LogLevel::Info, format_args!("1"))
        });

        println!("\n");
        println!("{}", logger.read_log().unwrap().decode().unwrap());
        println!("{}", logger.read_log().unwrap().decode().unwrap());
        println!("{}", logger.read_log().unwrap().decode().unwrap());
        println!("\n");
    }

    #[test]
    fn function_formatting() {
        let logger: Logger<1024, 32> = Logger::init(LogLevel::Info);

        let _ = logger.write_log(LogLevel::Info, format_args!("Ferris"));
        let _ = logger.write_log(LogLevel::Info, format_args!("the"));
        let _ = logger.write_log(LogLevel::Info, format_args!("crab"));

        println!("\n");
        println!("{}", logger.read_log().unwrap().decode().unwrap());
        println!("{}", logger.read_log().unwrap().decode().unwrap());
        println!("{}", logger.read_log().unwrap().decode().unwrap());
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
        crate::init_global_logger!(1024, 32, LogLevel::Info);

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