use log::{Log, Metadata, Record, SetLoggerError, LevelFilter};

use crate::println;
use lazy_static::lazy_static;

struct SimpleLogger;

impl SimpleLogger {
    pub fn new() -> Self {
        SimpleLogger
    }
}

#[allow(unused)]
impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "[{}] {}::{} {}",
                record.level(),
                record.file().unwrap_or(""),
                record.line().unwrap_or(0),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

lazy_static! {
    static ref OSLOGGER: SimpleLogger = SimpleLogger::new();
}

pub fn init() -> Result<(), SetLoggerError> {
    println!("init logger {:x}", &raw const OSLOGGER as usize);
    log::set_logger(&(*OSLOGGER)).map(|()| log::set_max_level(LevelFilter::Info))
}
