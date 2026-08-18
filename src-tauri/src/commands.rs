use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::audio::{output_devices, AudioEngine, MAX_LAYERS};
use crate::library::Library;
use crate::model::{EngineStatus, SoundEntry};
use crate::settings::SettingsStore;

pub const BOOT_EVENT: &str = "nuru://boot";

pub struct AppState {
    pub engine: RwLock<AudioEngine>,
    pub library: Arc<Library>,
    pub settings: Arc<SettingsStore>,
    pub booted: AtomicBool,
    pub presence: crate::discord::Presence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub label: String,
    pub progress: f32,
    pub done: bool,
    pub error: Option<String>,
}

impl Progress {
    fn step(phase: &str, label: impl Into<String>, progress: f32) -> Self {
        Self {
            phase: phase.into(),
            label: label.into(),
            progress,
            done: false,
            error: None,
        }
    }
}

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[tauri::command]
pub fn list_sounds(state: State<'_, AppState>) -> Vec<SoundEntry> {
    state.library.entries()
}

#[tauri::command]
pub fn rescan_packs(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let roots = crate::pack_roots(&app);
    state.library.scan(&roots).map_err(err)
}

#[tauri::command]
pub fn boot(app: tauri::AppHandle, state: State<'_, AppState>) {
    if state.booted.swap(true, Ordering::SeqCst) {
        let _ = app.emit(
            BOOT_EVENT,
            Progress { phase: "ready".into(), label: String::new(), progress: 1.0, done: true, error: None },
        );
        return;
    }

    let library = state.library.clone();
    let device = state.engine.read().device.name.clone();

    std::thread::Builder::new()
        .name("nuru-boot".into())
        .spawn(move || {
            let emit = |p: Progress| {
                let _ = app.emit(BOOT_EVENT, p);
            };

            emit(Progress::step("engine", device, 0.08));
            emit(Progress::step("packs", "Reading sound packs", 0.16));

            let started = std::time::Instant::now();
            let (ok, bad) = library.verify_all(|i, total, name| {
                let frac = if total == 0 { 1.0 } else { i as f32 / total as f32 };
                emit(Progress::step(
                    "verify",
                    if name.is_empty() { String::new() } else { format!("Checking {name}") },
                    0.16 + frac * 0.78,
                ));
            });
            log::info!(
                "verified {ok} sound(s) in {} ms{}",
                started.elapsed().as_millis(),
                if bad > 0 { format!(", {bad} unplayable") } else { String::new() }
            );

            emit(Progress {
                phase: "ready".into(),
                label: String::new(),
                progress: 1.0,
                done: true,
                error: None,
            });
        })
        .ok();
}

#[tauri::command]
pub fn play_sound(state: State<'_, AppState>, sound_id: String, volume: f32) -> Result<(), String> {
    let (sound, path) = state.library.ensure_local(&sound_id).map_err(err)?;
    state
        .engine
        .read()
        .add_layer(
            &sound_id,
            path,
            sound.loop_points.start_sample,
            sound.loop_points.end_sample,
            sound.loop_points.crossfade_ms,
            volume,
        )
        .map_err(err)
}

#[tauri::command]
pub fn stop_sound(state: State<'_, AppState>, sound_id: String) -> Result<(), String> {
    state.engine.read().remove_layer(&sound_id).map_err(err)
}

#[tauri::command]
pub fn set_sound_volume(
    state: State<'_, AppState>,
    sound_id: String,
    volume: f32,
) -> Result<(), String> {
    state.engine.read().set_layer_fader(&sound_id, volume).map_err(err)
}

#[tauri::command]
pub fn set_master_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    state.engine.read().set_master(volume).map_err(err)
}

#[tauri::command]
pub fn set_playing(state: State<'_, AppState>, playing: bool) -> Result<(), String> {
    state.engine.read().set_playing(playing).map_err(err)
}

#[tauri::command]
pub fn stop_all(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.read().clear().map_err(err)
}

#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> EngineStatus {
    let engine = state.engine.read();
    EngineStatus {
        device: engine.device.name.clone(),
        sample_rate: engine.device.sample_rate,
        channels: engine.device.channels,
        max_layers: MAX_LAYERS,
        active: engine.active_sounds(),
        underruns: engine.underruns(),
    }
}

#[tauri::command]
pub fn list_output_devices(state: State<'_, AppState>) -> (Vec<String>, Option<String>, String) {
    (
        output_devices(),
        state.settings.get().output_device,
        state.engine.read().device.name.clone(),
    )
}

#[tauri::command]
pub fn set_output_device(state: State<'_, AppState>, device: Option<String>) -> Result<String, String> {
    let engine = AudioEngine::open(device.as_deref()).map_err(err)?;
    let name = engine.device.name.clone();
    *state.engine.write() = engine;
    state.settings.update(|s| s.output_device = device);
    log::info!("output device switched to {name}");
    Ok(name)
}

#[tauri::command]
pub fn set_presence(
    state: State<'_, AppState>,
    details: String,
    status: String,
    active: bool,
    started_at: Option<i64>,
) {
    state.presence.set(crate::discord::Status {
        details,
        state: status,
        started_at,
        active,
    });
}

#[tauri::command]
pub fn discord_now() -> i64 {
    crate::discord::now_secs()
}


#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> crate::updater::UpdateStatus {
    let dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let status = crate::updater::check(&dir, crate::version::FULL).await;
    match (&status.available, &status.error) {
        (Some(r), _) => log::info!(
            "update available: {} (running {}, source {})",
            r.version,
            status.current_version,
            status.channel
        ),
        (None, Some(e)) => log::info!("update check: {}", e.message()),
        (None, None) => log::info!("up to date at {}", status.current_version),
    }
    status
}

#[tauri::command]
pub fn install_update(app: tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let dir = handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        let fail = |handle: &tauri::AppHandle, e: crate::updater::UpdateError| {
            log::error!("update failed: {}", e.message());
            let _ = handle.emit(
                crate::updater::UPDATE_EVENT,
                Progress {
                    phase: "error".into(),
                    label: String::new(),
                    progress: 0.0,
                    done: true,
                    error: Some(e.message().to_string()),
                },
            );
        };

        let emit = |p: Progress| {
            let _ = handle.emit(crate::updater::UPDATE_EVENT, p);
        };

        emit(Progress::step("checking", "Looking for an update", 0.04));

        let status = crate::updater::check(&dir, crate::version::FULL).await;
        let Some(release) = status.available else {
            fail(&handle, status.error.unwrap_or(crate::updater::UpdateError::NotPublished));
            return;
        };

        emit(Progress::step("downloading", format!("Downloading {}", release.version), 0.08));

        let handle_dl = handle.clone();
        let version = release.version.clone();
        let downloaded = crate::updater::download(&release, move |got, total| {
            let frac = if total == 0 { 0.0 } else { got as f32 / total as f32 };
            let _ = handle_dl.emit(
                crate::updater::UPDATE_EVENT,
                Progress::step(
                    "downloading",
                    format!("Downloading {version}"),
                    0.08 + frac * 0.74,
                ),
            );
        })
        .await;

        let path = match downloaded {
            Ok(p) => p,
            Err(e) => return fail(&handle, e),
        };

        emit(Progress::step("installing", "Installing", 0.9));
        log::info!("running installer {}", path.display());

        if let Err(e) = crate::updater::install(&path) {
            return fail(&handle, e);
        }

        emit(Progress::step("restarting", "Restarting Nuru", 1.0));
        std::thread::sleep(crate::updater::quit_delay());
        handle.exit(0);
    });
}

#[tauri::command]
pub async fn release_notes(app: tauri::AppHandle, version: String) -> Option<crate::updater::ReleaseInfo> {
    let dir = app.path().app_data_dir().ok()?;
    crate::updater::notes_for(&dir, &version).await
}

#[tauri::command]
pub fn pending_changelog(app: tauri::AppHandle) -> Option<crate::updater::ReleaseInfo> {
    let dir = app.path().app_data_dir().ok()?;
    let found = crate::updater::pending_changelog(&dir, crate::version::FULL);
    if let Some(r) = &found {
        log::info!("showing the changelog for {}", r.version);
    }
    found
}

#[tauri::command]
pub fn mark_changelog_seen(app: tauri::AppHandle) {
    if let Ok(dir) = app.path().app_data_dir() {
        crate::updater::mark_seen(&dir, crate::version::FULL);
    }
}

#[tauri::command]
pub fn get_auto_update(state: State<'_, AppState>) -> bool {
    state.settings.get().auto_update
}

#[tauri::command]
pub fn set_auto_update(state: State<'_, AppState>, enabled: bool) {
    state.settings.update(|s| s.auto_update = enabled);
    log::info!("automatic updates {}", if enabled { "enabled" } else { "disabled" });
}

#[tauri::command]
pub fn open_release_page(app: tauri::AppHandle, url: String) {
    if url.starts_with("https://github.com/") {
        let _ = tauri_plugin_opener::open_url(url, None::<&str>);
    }
    let _ = app;
}

#[tauri::command]
pub fn unshippable_sounds(state: State<'_, AppState>) -> Vec<String> {
    state.library.unshippable()
}

#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
