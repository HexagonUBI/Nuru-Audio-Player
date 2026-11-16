use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

const APP: &str = "nuru";
const RETRY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum CloudEvent {
    Connected,
    Disconnected,
    UpdateReady { version: String },
    CloseRequest { reason: String },
}

pub struct Cloud {
    connected: Arc<AtomicBool>,
}

impl Cloud {
    pub fn start(sink: impl Fn(CloudEvent) + Send + 'static) -> Self {
        let connected = Arc::new(AtomicBool::new(false));
        if std::env::var_os("NURU_NO_CLOUD").is_some() {
            return Self { connected };
        }
        let flag = connected.clone();
        let started = std::thread::Builder::new().name("nuru-cloud".into()).spawn(move || loop {
            match session(&sink, &flag) {
                Ok(()) => {}
                Err(e) => log::debug!("hexagon cloud: {e}"),
            }
            if flag.swap(false, Ordering::AcqRel) {
                sink(CloudEvent::Disconnected);
            }
            std::thread::sleep(RETRY);
        });
        if let Err(e) = started {
            log::warn!("hexagon cloud watcher did not start: {e}");
        }
        Self { connected }
    }

    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }
}

fn pipe_name() -> String {
    if let Ok(name) = std::env::var("HEXAGON_CLOUD_PIPE") {
        if !name.trim().is_empty() {
            return name;
        }
    }
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into()).to_lowercase();
    format!(r"\\.\pipe\hexagon-cloud-{user}")
}

fn session(sink: &impl Fn(CloudEvent), flag: &AtomicBool) -> std::io::Result<()> {
    let pipe = std::fs::OpenOptions::new().read(true).write(true).open(pipe_name())?;
    let mut writer = pipe.try_clone()?;
    let mut lines = BufReader::new(pipe).lines();

    let mut ask = |req: Value| -> std::io::Result<Value> {
        writeln!(writer, "{req}")?;
        writer.flush()?;
        let line = lines.next().ok_or_else(|| std::io::Error::other("pipe closed"))??;
        serde_json::from_str(&line).map_err(std::io::Error::other)
    };

    let hello = ask(json!({"op": "hello", "app": APP, "version": crate::version(), "pid": std::process::id()}))?;
    if hello["ok"] != json!(true) {
        return Err(std::io::Error::other(format!("hello refused: {hello}")));
    }
    let watch = ask(json!({"op": "watch"}))?;
    if watch["ok"] != json!(true) {
        return Err(std::io::Error::other(format!("watch refused: {watch}")));
    }
    drop(ask);

    log::info!("connected to Hexagon Cloud {}", hello["cloud"].as_str().unwrap_or("?"));
    flag.store(true, Ordering::Release);
    sink(CloudEvent::Connected);

    for line in lines {
        let line = line?;
        let Ok(event) = serde_json::from_str::<Value>(&line) else { continue };
        if event["app"].as_str().is_some_and(|a| a != APP) {
            continue;
        }
        match event["event"].as_str() {
            Some("update-ready") => {
                sink(CloudEvent::UpdateReady { version: event["version"].as_str().unwrap_or("").to_string() })
            }
            Some("close-request") => {
                sink(CloudEvent::CloseRequest { reason: event["reason"].as_str().unwrap_or("").to_string() })
            }
            _ => {}
        }
    }
    Ok(())
}
