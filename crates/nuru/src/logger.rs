use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_BYTES: u64 = 2 * 1024 * 1024;

struct Sink {
    path: PathBuf,
    file: File,
    written: u64,
}

struct Logger {
    sink: Mutex<Option<Sink>>,
}

static LOGGER: Logger = Logger { sink: Mutex::new(None) };

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) || !record.target().starts_with("nuru") {
            return;
        }
        let _ = writeln!(std::io::stderr(), "[{}] {}", record.level(), record.args());
        let Ok(mut guard) = self.sink.lock() else { return };
        let Some(sink) = guard.as_mut() else { return };
        let line = format!(
            "[{}][{}][{}] {}\n",
            chrono::Local::now().format("%Y-%m-%d][%H:%M:%S"),
            record.target(),
            record.level(),
            record.args()
        );
        if sink.written + line.len() as u64 > MAX_BYTES {
            match File::create(&sink.path) {
                Ok(file) => {
                    sink.file = file;
                    sink.written = 0;
                }
                Err(_) => return,
            }
        }
        if sink.file.write_all(line.as_bytes()).is_ok() {
            sink.written += line.len() as u64;
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.sink.lock() {
            if let Some(sink) = guard.as_mut() {
                let _ = sink.file.flush();
            }
        }
    }
}

pub fn init() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

pub fn to_file(dir: PathBuf) {
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("Nuru.log");
    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let opened = if size >= MAX_BYTES { File::create(&path) } else { OpenOptions::new().create(true).append(true).open(&path) };
    let Ok(file) = opened else { return };
    let written = if size >= MAX_BYTES { 0 } else { size };
    if let Ok(mut guard) = LOGGER.sink.lock() {
        *guard = Some(Sink { path, file, written });
    }
}
