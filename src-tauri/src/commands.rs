//! The surface the UI talks to. Thin on purpose — these translate, they don't
//! decide.

use tauri::{Manager, State};

use crate::audio::{AudioEngine, MAX_LAYERS};
use crate::library::Library;
use crate::model::{EngineStatus, SoundEntry};

pub struct AppState {
    pub engine: AudioEngine,
    /// Shared so the startup verification pass can hold it on its own thread.
    pub library: std::sync::Arc<Library>,
}

/// Anyhow errors don't cross the IPC boundary; flatten them to a string that is
/// safe to show a user, keeping the full chain.
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

/// Load a sound into the mixer. Fails if its file is missing or fails its hash —
/// nothing plays that has not been verified on this machine first.
#[tauri::command]
pub fn play_sound(
    state: State<'_, AppState>,
    sound_id: String,
    volume: f32,
) -> Result<(), String> {
    let (sound, path) = state.library.ensure_local(&sound_id).map_err(err)?;
    state
        .engine
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
    state.engine.remove_layer(&sound_id).map_err(err)
}

#[tauri::command]
pub fn set_sound_volume(
    state: State<'_, AppState>,
    sound_id: String,
    volume: f32,
) -> Result<(), String> {
    state.engine.set_layer_fader(&sound_id, volume).map_err(err)
}

#[tauri::command]
pub fn set_master_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    state.engine.set_master(volume).map_err(err)
}

#[tauri::command]
pub fn set_playing(state: State<'_, AppState>, playing: bool) -> Result<(), String> {
    state.engine.set_playing(playing).map_err(err)
}

#[tauri::command]
pub fn stop_all(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.clear().map_err(err)
}

#[tauri::command]
pub fn engine_status(state: State<'_, AppState>) -> EngineStatus {
    EngineStatus {
        device: state.engine.device.name.clone(),
        sample_rate: state.engine.device.sample_rate,
        channels: state.engine.device.channels,
        max_layers: MAX_LAYERS,
        active: state.engine.active_sounds(),
        underruns: state.engine.underruns(),
    }
}

/// Sounds currently loaded that must not ship. Surfaced in the About panel so a
/// development build is never mistaken for a releasable one.
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
