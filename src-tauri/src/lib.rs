pub mod audio;
pub mod commands;
pub mod discord;
pub mod library;
pub mod model;
pub mod settings;
pub mod updater;
pub mod version;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::RwLock;
use tauri::Manager;

use crate::audio::AudioEngine;
use crate::commands::AppState;
use crate::library::Library;
use crate::settings::SettingsStore;

pub fn pack_roots(app: &tauri::AppHandle) -> Vec<(PathBuf, bool)> {
    let mut roots = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        roots.push((res.join("packs"), true));
    }
    if let Ok(data) = app.path().app_data_dir() {
        roots.push((data.join("packs"), false));
    }
    if cfg!(debug_assertions) {
        if let Some(repo) = std::env::current_dir().ok().and_then(|d| d.parent().map(Path::to_path_buf))
        {
            roots.push((repo.join("resources").join("packs"), true));
        }
    }
    roots
}

fn trim_webview() {
    const ARGS: &str = concat!(
        "--disable-features=",
        "Translate,MediaRouter,OptimizationHints,OptimizationGuideModelDownloading,",
        "InterestFeedContentSuggestions,CalculateNativeWinOcclusion,",
        "AutofillServerCommunication,CertificateTransparencyComponentUpdater",
        " --disable-background-networking",
        " --disable-component-update",
        " --disable-sync",
        " --disable-breakpad",
        " --no-pings",
        " --renderer-process-limit=1",
        " --disable-gpu-shader-disk-cache",
        " --js-flags=--max-old-space-size=96",
    );

    let mut args = String::from(ARGS);
    if let Ok(extra) = std::env::var("NURU_WEBVIEW_ARGS") {
        args.push(' ');
        args.push_str(&extra);
    }
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", args);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    trim_webview();

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
                .max_file_size(2 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .setup(|app| {
            let settings = Arc::new(SettingsStore::load(
                app.path()
                    .app_data_dir()
                    .map(|d| d.join("settings.json"))
                    .unwrap_or_else(|_| PathBuf::from("settings.json")),
            ));

            let engine = AudioEngine::open(settings.get().output_device.as_deref())?;
            log::info!(
                "audio out: {} @ {} Hz, {} ch",
                engine.device.name,
                engine.device.sample_rate,
                engine.device.channels
            );

            let library = Arc::new(Library::new());
            if let Ok(data) = app.path().app_data_dir() {
                library.use_cache(data.join("verified.json"));
            }
            let roots = pack_roots(&app.handle());

            let scope = app.asset_protocol_scope();
            for (root, _) in &roots {
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
                    "{} development placeholder sound(s) loaded, this build must not be released",
                    unshippable.len()
                );
            }

            app.manage(AppState {
                engine: RwLock::new(engine),
                library,
                settings,
                booted: AtomicBool::new(false),
                presence: crate::discord::Presence::start(),
            });

            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::boot,
            commands::list_sounds,
            commands::rescan_packs,
            commands::play_sound,
            commands::stop_sound,
            commands::set_sound_volume,
            commands::set_master_volume,
            commands::set_playing,
            commands::stop_all,
            commands::engine_status,
            commands::list_output_devices,
            commands::set_output_device,
            commands::check_update,
            commands::release_notes,
            commands::pending_changelog,
            commands::mark_changelog_seen,
            commands::get_auto_update,
            commands::set_auto_update,
            commands::install_update,
            commands::open_release_page,
            commands::set_presence,
            commands::discord_now,
            commands::unshippable_sounds,
            commands::show_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nuru");
}
