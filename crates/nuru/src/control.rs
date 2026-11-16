use std::io::{BufRead, Write};

pub fn enabled() -> bool {
    std::env::var_os("NURU_CONTROL").is_some()
}

pub fn spawn(weak: slint::Weak<crate::AppWindow>) {
    if !enabled() {
        return;
    }
    let started = std::thread::Builder::new().name("nuru-control".into()).spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let line = line.trim().trim_start_matches('\u{feff}').to_string();
            if line.is_empty() {
                continue;
            }
            let weak = weak.clone();
            let sent = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    crate::command(&ui, &line);
                }
            });
            if sent.is_err() {
                break;
            }
        }
    });
    if let Err(e) = started {
        log::warn!("control channel did not start: {e}");
    }
}

pub fn say(line: &str) {
    if !enabled() {
        return;
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}
