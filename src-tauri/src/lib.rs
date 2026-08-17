pub mod audio;
pub mod commands;
pub mod library;
pub mod model;

use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::audio::AudioEngine;
use crate::commands::AppState;
use crate::library::Library;

/// Where packs are looked for, lowest priority first. A user pack sharing an id
/// with a bundled sound wins, which is how the sound database adds to the
/// built-in set rather than sitting beside it.
pub fn pack_roots(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        roots.push(res.join("packs"));
    }
    if let Ok(data) = app.path().app_data_dir() {
        roots.push(data.join("packs"));
    }
    // In a dev build, also read packs straight out of the repo, so regenerating
    // one and restarting is enough to see the change — no reinstall, no waiting
    // for resources to be copied. Last root wins, so this shadows the bundled
    // copy during development only.
    if cfg!(debug_assertions) {
        // cargo runs us from src-tauri/, so the repo root is its parent.
        // Built with `parent()` rather than `join("..")` on purpose: the asset
        // protocol rejects any path containing `..` outright, so a literal one
        // here means every cover image silently 403s.
        if let Some(repo) = std::env::current_dir().ok().and_then(|d| d.parent().map(Path::to_path_buf))
        {
            roots.push(repo.join("resources").join("packs"));
        }
    }
    roots
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                // The plugin already writes to stdout and to the app log
                // directory (%LOCALAPPDATA%\app.nuru.player\logs on Windows).
                // Adding those targets again duplicates every line, so only the
                // size cap is set here — without it the file grows forever.
                .max_file_size(2 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .setup(|app| {
            let engine = AudioEngine::new()?;
            log::info!(
                "audio out: {} @ {} Hz, {} ch",
                engine.device.name,
                engine.device.sample_rate,
                engine.device.channels
            );

            let library = std::sync::Arc::new(Library::new());
            if let Ok(data) = app.path().app_data_dir() {
                library.use_cache(data.join("verified.json"));
            }
            let roots = pack_roots(&app.handle());

            // Cover images are loaded over the asset protocol, which only serves
            // paths inside its scope. The static scope in tauri.conf.json covers
            // the two installed locations; a development pack living in the repo
            // has to be added here, at runtime, because the config has no way to
            // name a path relative to the project.
            let scope = app.asset_protocol_scope();
            for root in &roots {
                if let Err(e) = scope.allow_directory(root, true) {
                    log::warn!("could not add {} to the asset scope: {e}", root.display());
                }
            }

            match library.scan(&roots) {
                Ok(n) => log::info!("loaded {n} sound pack(s)"),
                Err(e) => log::error!("scanning packs failed: {e:#}"),
            }

            let unshippable = library.unshippable();
            if !unshippable.is_empty() {
                log::warn!(
                    "{} development placeholder sound(s) loaded — this build must not be released",
                    unshippable.len()
                );
            }

            // Hash the whole library now rather than in front of the first play
            // of each sound. On a background thread so it costs the UI nothing;
            // by the time anyone clicks a tile the result is already cached and
            // playback starts immediately.
            {
                let library = library.clone();
                std::thread::Builder::new()
                    .name("nuru-verify".into())
                    .spawn(move || {
                        let started = std::time::Instant::now();
                        let (ok, bad) = library.verify_all();
                        log::info!(
                            "verified {ok} sound(s) in {} ms{}",
                            started.elapsed().as_millis(),
                            if bad > 0 { format!(", {bad} unplayable") } else { String::new() }
                        );
                    })
                    .ok();
            }

            app.manage(AppState { engine, library });

            // The window is created hidden so the first frame the user sees is
            // the finished layout, not a white flash while the WebView boots.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_sounds,
            commands::rescan_packs,
            commands::play_sound,
            commands::stop_sound,
            commands::set_sound_volume,
            commands::set_master_volume,
            commands::set_playing,
            commands::stop_all,
            commands::engine_status,
            commands::unshippable_sounds,
            commands::show_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nuru");
}
