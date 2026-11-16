#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod catalog;
mod control;
mod covers;
mod cozy;
mod effects;
mod logger;
mod panels;
mod scene_art;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use nuru_core::cloud::{Cloud, CloudEvent};
use nuru_core::core::{Change, Core};
use nuru_core::update;
use nuru_core::library::Library;
use nuru_core::model::SoundEntry;
use nuru_core::paths::Paths;
use slint::winit_030::WinitWindowAccessor;
use slint::{Color, ComponentHandle, Image, Model, ModelRc, SharedString, VecModel};

slint::include_modules!();

const VISIBLE_TAGS: usize = 8;
const SPINNER_DELAY: Duration = Duration::from_millis(150);

enum Pending {
    Cloud(String),
    Own(update::Available),
}

struct App {
    ui: slint::Weak<AppWindow>,
    core: Arc<Core>,
    pending: Option<Pending>,
    entries: Vec<SoundEntry>,
    group_of: Vec<usize>,
    group_names: Vec<String>,
    shape: covers::Shape,
    cover_images: Vec<Image>,
    active_images: Vec<Option<Image>>,
    tinting: Vec<bool>,
    shown_active: Vec<bool>,
    shown_loading: Vec<bool>,
    loading_since: Vec<Option<Instant>>,
    tags: Vec<String>,
    all_tags: bool,
    filter: String,
    search: String,
    slots: Vec<Option<(usize, usize)>>,
    group_models: Vec<Rc<VecModel<TileData>>>,
    groups: Rc<VecModel<GroupData>>,
    chips: Rc<VecModel<ChipData>>,
    layers: Rc<VecModel<LayerData>>,
    toasts: Rc<VecModel<ToastData>>,
    next_toast: i32,
    brush: Option<String>,
    painting: bool,
    paint_clear: bool,
    ticker: slint::Timer,
    scale: f32,
    cozy: Option<CozyState>,
}

struct CozyState {
    engine: cozy::Cozy,
    tiles: Rc<VecModel<CozyTile>>,
    panes: bool,
    baking: bool,
    want: Option<cozy::Look>,
    settle: slint::Timer,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.borrow_mut().as_mut().map(f))
}

fn main() -> Result<()> {
    logger::init();
    select_backend()?;

    let paths = Paths::resolve();
    let library = Arc::new(Library::new());
    let found = library.scan(&catalog::pack_roots(&paths)).context("scanning sound packs")?;
    log::info!("loaded {found} sound pack(s)");

    let sink: nuru_core::core::Sink = Arc::new(|change: Change| {
        let _ = slint::invoke_from_event_loop(move || on_change(change));
    });
    let core = Core::open(paths, library.clone(), sink)?;
    let cloud = Arc::new(Cloud::start(|event| {
        let _ = slint::invoke_from_event_loop(move || on_cloud(event));
    }));
    if let Some(h) = update::take_handoff(core.paths()) {
        log::info!("updated from {} to {}", h.from, h.to);
    }

    let unsorted = library.entries();
    let tags = catalog::tags_by_use(&unsorted);
    let mut entries = Vec::new();
    let mut group_of = Vec::new();
    let mut group_names = Vec::new();
    for (gi, group) in catalog::grouped(unsorted).into_iter().enumerate() {
        group_names.push(group.name);
        for sound in group.sounds {
            entries.push(sound);
            group_of.push(gi);
        }
    }
    let count = entries.len();

    let ui = AppWindow::new()?;
    ui.set_version(env!("NURU_VERSION").into());
    ui.set_channel(env!("NURU_CHANNEL").into());
    ui.global::<Theme>().set_name(core.saved().theme.as_str().into());
    ui.set_update_status("Not checked yet".into());
    ui.set_master(core.saved().master_volume);
    ui.set_playing(core.playing());

    let scale = ui.window().scale_factor();
    let size = core.saved().tile_size;
    let shape = covers::Shape { size: (size.px() * scale).round() as u32, radius: size.radius() * scale };
    let cover_images: Vec<Image> = entries
        .iter()
        .map(|e| {
            e.cover_path
                .as_deref()
                .and_then(|p| covers::resting(p, shape))
                .map(Image::from_rgba8)
                .unwrap_or_default()
        })
        .collect();
    install_effects(&ui, size.radius(), scale);
    apply_tile_props(&ui, size);

    let groups_model = Rc::new(VecModel::<GroupData>::default());
    let chips_model = Rc::new(VecModel::<ChipData>::default());
    let layers_model = Rc::new(VecModel::<LayerData>::default());
    ui.set_groups(ModelRc::from(groups_model.clone()));
    ui.set_chips(ModelRc::from(chips_model.clone()));
    ui.set_layers(ModelRc::from(layers_model.clone()));
    let toasts_model = Rc::new(VecModel::<ToastData>::default());
    ui.set_toasts(ModelRc::from(toasts_model.clone()));

    APP.with(|cell| {
        *cell.borrow_mut() = Some(App {
            ui: ui.as_weak(),
            core: core.clone(),
            pending: None,
            entries,
            group_of,
            group_names,
            shape,
            cover_images,
            active_images: vec![None; count],
            tinting: vec![false; count],
            shown_active: vec![false; count],
            shown_loading: vec![false; count],
            loading_since: vec![None; count],
            tags,
            all_tags: false,
            filter: String::new(),
            search: String::new(),
            slots: vec![None; count],
            group_models: Vec::new(),
            groups: groups_model,
            chips: chips_model,
            layers: layers_model,
            toasts: toasts_model,
            next_toast: 0,
            brush: None,
            painting: false,
            paint_clear: false,
            ticker: slint::Timer::default(),
            scale,
            cozy: None,
        })
    });
    with_app(|app| {
        refresh_chips(app);
        refresh_grid(app);
        if let Some(ui) = app.ui.upgrade() {
            panels::refresh_mixes(app, &ui);
            panels::refresh_timer(app, &ui);
        }
    });

    {
        let core = core.clone();
        std::thread::Builder::new().name("nuru-verify".into()).spawn(move || {
            let started = Instant::now();
            let (ok, bad) = core.library().verify_all(|_, _, _| {});
            log::info!("verified {ok} sound(s) in {} ms, {bad} not playable", started.elapsed().as_millis());
            core.restore_last_mix();
            control::say("verified");
            if let Some(release) = update::pending_changelog(core.paths(), nuru_core::version()) {
                let _ = slint::invoke_from_event_loop(move || show_changelog(release));
            }
            if cloud.connected() {
                return;
            }
            let result = update::check(core.paths(), nuru_core::version());
            let _ = slint::invoke_from_event_loop(move || on_checked(result, true));
        })?;
    }

    wire(&ui);
    control::spawn(ui.as_weak());
    ui.show()?;
    slint::Timer::single_shot(Duration::ZERO, || control::say("ready"));
    slint::run_event_loop_until_quit()?;

    let _ = ui.hide();
    APP.with(|cell| cell.borrow_mut().take());
    drop(ui);
    core.shutdown();
    Ok(())
}

fn on_checked(result: Result<Option<update::Available>, update::UpdateError>, on_launch: bool) {
    with_app(|app| {
        if let Some(ui) = app.ui.upgrade() {
            ui.set_checking(false);
            ui.set_update_status(match &result {
                Ok(_) => "Up to date",
                Err(e) => e.message(),
            }.into());
        }
    });
    if let Ok(Some(available)) = result {
        on_available(available, on_launch);
    }
}

fn show_changelog(release: update::Release) {
    with_app(|app| {
        if let Some(ui) = app.ui.upgrade() {
            ui.set_changelog_version(release.version.as_str().into());
            ui.set_changelog_entries(panels::entries(&release));
            ui.set_panel("changelog".into());
        }
    });
}

fn toast(text: String, error: bool) {
    let id = with_app(|app| {
        app.next_toast += 1;
        let id = app.next_toast;
        app.toasts.push(ToastData { id, text: text.into(), error });
        id
    });
    let Some(id) = id else { return };
    let wait = Duration::from_millis(if error { 6000 } else { 3200 });
    slint::Timer::single_shot(wait, move || {
        with_app(|app| {
            if let Some(pos) = (0..app.toasts.row_count()).find(|&i| app.toasts.row_data(i).is_some_and(|t| t.id == id)) {
                app.toasts.remove(pos);
            }
        });
    });
}

fn apply_tile_props(ui: &AppWindow, size: nuru_core::saved::TileSize) {
    ui.set_tile_size(size.px());
    ui.set_tile_gap(size.gap());
    ui.set_tile_radius(size.radius());
    ui.set_tile_label(size.label());
}

fn change_tile_size(size: nuru_core::saved::TileSize) {
    let Some(core) = core() else { return };
    core.set_tile_size(size);
    with_app(|app| {
        let Some(ui) = app.ui.upgrade() else { return };
        app.shape = covers::Shape { size: (size.px() * app.scale).round() as u32, radius: size.radius() * app.scale };
        let shape = app.shape;
        app.cover_images = app
            .entries
            .iter()
            .map(|e| e.cover_path.as_deref().and_then(|p| covers::resting(p, shape)).map(Image::from_rgba8).unwrap_or_default())
            .collect();
        for img in app.active_images.iter_mut() {
            *img = None;
        }
        for flag in app.shown_active.iter_mut() {
            *flag = false;
        }
        install_effects(&ui, size.radius(), app.scale);
        apply_tile_props(&ui, size);
        refresh_grid(app);
    });
    sync_mix();
}

fn tick_timer() {
    let running = with_app(|app| {
        if let Some(ui) = app.ui.upgrade() {
            panels::refresh_timer(app, &ui);
            ui.get_timer_running()
        } else {
            false
        }
    })
    .unwrap_or(false);
    with_app(|app| {
        if running && !app.ticker.running() {
            app.ticker.start(slint::TimerMode::Repeated, Duration::from_millis(500), tick_timer);
        } else if !running {
            app.ticker.stop();
        }
    });
}

fn paint(slot: usize) {
    with_app(|app| {
        let Some(brush) = app.brush.clone() else { return };
        let current = app.core.slots().get(slot).cloned().flatten();
        let next = if app.paint_clear || current.as_deref() == Some(brush.as_str()) { None } else { Some(brush) };
        app.core.set_slot(slot, next);
    });
}

fn open_link(url: &str) {
    if !url.starts_with("https://") {
        return;
    }
    if let Err(e) = std::process::Command::new("explorer").arg(url).spawn() {
        log::warn!("could not open {url}: {e}");
    }
}

fn on_available(available: update::Available, on_launch: bool) {
    let auto = with_app(|app| {
        if let Some(ui) = app.ui.upgrade() {
            ui.set_update(available.build.version.as_str().into());
            if let Some(release) = &available.release {
                ui.set_available_notes(panels::entries(release));
            }
        }
        app.pending = Some(Pending::Own(available.clone()));
        app.core.settings().auto_update
    });
    if on_launch && auto == Some(true) {
        start_own_update(available);
    }
}

fn start_own_update(available: update::Available) {
    let spawned = std::thread::Builder::new().name("nuru-update".into()).spawn(move || {
        match update::download(&available.build, |_, _| {}) {
            Ok(installer) => {
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(core) = core() else { return };
                    update::write_handoff(core.paths(), nuru_core::version(), &available.build.version);
                    match update::install(&installer) {
                        Ok(()) => {
                            let _ = slint::quit_event_loop();
                        }
                        Err(e) => log::warn!("{}", e.message()),
                    }
                });
            }
            Err(e) => log::warn!("{}", e.message()),
        }
    });
    if let Err(e) = spawned {
        log::warn!("could not start the update: {e}");
    }
}

fn quit_for_cloud(version: &str) {
    if let Some(core) = core() {
        update::write_handoff(core.paths(), nuru_core::version(), version);
    }
    let _ = slint::quit_event_loop();
}

fn on_cloud(event: CloudEvent) {
    match event {
        CloudEvent::Connected | CloudEvent::Disconnected => {
            with_app(|app| {
                if matches!(app.pending, Some(Pending::Cloud(_))) {
                    app.pending = None;
                    if let Some(ui) = app.ui.upgrade() {
                        ui.set_update(SharedString::default());
                    }
                }
            });
        }
        CloudEvent::UpdateReady { version } => {
            with_app(|app| {
                if let Some(ui) = app.ui.upgrade() {
                    ui.set_update(version.as_str().into());
                }
                app.pending = Some(Pending::Cloud(version));
            });
        }
        CloudEvent::CloseRequest { reason } => {
            if reason == "uninstall" {
                let _ = slint::quit_event_loop();
                return;
            }
            let (audible, version) = with_app(|app| {
                let version = match &app.pending {
                    Some(Pending::Cloud(v)) => v.clone(),
                    _ => String::new(),
                };
                (app.core.audible(), version)
            })
            .unwrap_or_default();
            if audible {
                with_app(|app| {
                    if app.pending.is_none() {
                        app.pending = Some(Pending::Cloud(version.clone()));
                    }
                    if let Some(ui) = app.ui.upgrade() {
                        if ui.get_update().is_empty() {
                            ui.set_update("ready".into());
                        }
                    }
                });
            } else {
                quit_for_cloud(&version);
            }
        }
    }
}

fn wire(ui: &AppWindow) {
    ui.on_panel_opened(|name| {
        with_app(|app| {
            let Some(ui) = app.ui.upgrade() else { return };
            match name.as_str() {
                "mixes" => panels::refresh_mixes(app, &ui),
                "timers" => {
                    panels::refresh_timer(app, &ui);
                    panels::refresh_mixes(app, &ui);
                    panels::refresh_schedule(app, &ui);
                }
                "settings" => {
                    panels::refresh_settings(app, &ui);
                    panels::refresh_devices(&ui);
                }
                "credits" => panels::refresh_credits(app, &ui),
                _ => {}
            }
        });
    });
    ui.on_save_mix(|name| {
        if let Some(core) = core() {
            core.save_preset(name.as_str());
        }
    });
    ui.on_load_mix(|id| {
        if let Some(core) = core() {
            core.load_preset(id.as_str(), None);
        }
    });
    ui.on_delete_mix(|id| {
        if let Some(core) = core() {
            core.delete_preset(id.as_str());
        }
    });
    ui.on_start_timer(|minutes| {
        if let Some(core) = core() {
            core.start_timer(minutes.clamp(1, 600) as u32);
        }
    });
    ui.on_cancel_timer(|| {
        if let Some(core) = core() {
            core.stop_timer();
        }
    });
    ui.on_set_schedule(|on| {
        if let Some(core) = core() {
            core.set_schedule_enabled(on);
        }
    });
    ui.on_pick_brush(|id| {
        with_app(|app| {
            app.brush = Some(id.to_string());
            if let Some(ui) = app.ui.upgrade() {
                panels::refresh_schedule(app, &ui);
            }
        });
    });
    ui.on_paint_begin(|slot| {
        let slot = slot.max(0) as usize;
        let start = with_app(|app| {
            let Some(brush) = app.brush.clone() else { return false };
            app.painting = true;
            app.paint_clear = app.core.slots().get(slot).cloned().flatten().as_deref() == Some(brush.as_str());
            true
        });
        if start == Some(true) {
            paint(slot);
        }
    });
    ui.on_paint_extend(|slot| {
        if with_app(|app| app.painting) == Some(true) {
            paint(slot.max(0) as usize);
        }
    });
    ui.on_paint_finish(|| {
        with_app(|app| app.painting = false);
    });
    ui.on_clear_schedule(|| {
        if let Some(core) = core() {
            core.clear_schedule();
        }
    });
    ui.on_set_theme(|theme| {
        if let Some(core) = core() {
            core.set_theme(theme.as_str());
        }
    });
    ui.on_set_tile_size(|id| change_tile_size(panels::tile_from(id.as_str())));
    ui.on_set_restore(|on| {
        if let Some(core) = core() {
            core.set_restore_on_launch(on);
        }
    });
    ui.on_set_follows(|on| {
        if let Some(core) = core() {
            core.set_scene_follows_volume(on);
        }
    });
    ui.on_set_device(|name| {
        if let Some(core) = core() {
            core.set_output_device(if name.is_empty() { None } else { Some(name.to_string()) });
        }
    });
    ui.on_set_auto_update(|on| {
        if let Some(core) = core() {
            core.set_auto_update(on);
        }
        with_app(|app| {
            if let Some(ui) = app.ui.upgrade() {
                panels::refresh_settings(app, &ui);
            }
        });
    });
    ui.on_check_update({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_checking(true);
            }
            let Some(core) = core() else { return };
            let _ = std::thread::Builder::new().name("nuru-check".into()).spawn(move || {
                let result = update::check(core.paths(), nuru_core::version());
                let _ = slint::invoke_from_event_loop(move || on_checked(result, false));
            });
        }
    });
    ui.on_whats_new(|| {
        let Some(core) = core() else { return };
        let current = nuru_core::version();
        let release = update::release_for(core.paths(), current)
            .unwrap_or(update::Release { version: current.to_string(), date: String::new(), entries: Vec::new() });
        show_changelog(release);
    });
    ui.on_changelog_done(|| {
        with_app(|app| {
            update::mark_seen(app.core.paths(), nuru_core::version());
            if let Some(ui) = app.ui.upgrade() {
                ui.set_panel(SharedString::default());
            }
        });
    });
    ui.on_open_link(|url| open_link(url.as_str()));
    ui.on_cozy_toggled(cozy_toggled);
    ui.on_cozy_resized(cozy_refresh);
    ui.on_update_clicked(|| {
        let pending = with_app(|app| match &app.pending {
            Some(Pending::Cloud(v)) => Some(Err(v.clone())),
            Some(Pending::Own(a)) => Some(Ok(a.clone())),
            None => None,
        })
        .flatten();
        match pending {
            Some(Ok(available)) => start_own_update(available),
            Some(Err(version)) => quit_for_cloud(&version),
            None => {}
        }
    });
    ui.on_toggle_tile(|i| {
        if let Some(Some((core, id))) =
            with_app(|app| app.entries.get(i as usize).map(|e| (app.core.clone(), e.sound.id.clone())))
        {
            core.toggle(&id);
        }
    });
    ui.on_chip_clicked(|chip| {
        with_app(|app| {
            if chip.more {
                app.all_tags = !app.all_tags;
            } else if chip.tag.is_empty() || app.filter == chip.tag.as_str() {
                app.filter.clear();
            } else {
                app.filter = chip.tag.to_string();
            }
            refresh_chips(app);
            refresh_grid(app);
        });
    });
    ui.on_toggle_group(|name| {
        if let Some(core) = core() {
            core.toggle_group(name.as_str());
        }
    });
    ui.on_search_changed(|text| {
        with_app(|app| {
            if app.search != text.as_str() {
                app.search = text.to_string();
                refresh_grid(app);
            }
        });
    });
    ui.on_set_volume(|id, v| {
        if let Some(core) = core() {
            core.set_volume(id.as_str(), v);
        }
    });
    ui.on_remove_layer(|id| {
        if let Some(core) = core() {
            core.remove(id.as_str());
        }
    });
    ui.on_set_master(|v| {
        if let Some(core) = core() {
            core.set_master(v);
        }
    });
    ui.on_stop_all(|| {
        if let Some(core) = core() {
            core.clear();
        }
    });
    ui.on_toggle_playing(|| {
        if let Some(core) = core() {
            core.toggle_playing();
        }
    });
    ui.on_drag_window({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().with_winit_window(|w| {
                    let _ = w.drag_window();
                });
            }
        }
    });
    ui.on_minimize_window({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().set_minimized(true);
            }
        }
    });
    ui.on_toggle_maximize({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                let maximized = !ui.window().is_maximized();
                ui.window().set_maximized(maximized);
            }
        }
    });
    ui.on_close_window(|| {
        let _ = slint::quit_event_loop();
    });
    ui.window().on_winit_window_event({
        let weak = ui.as_weak();
        move |window, event| {
            if let slint::winit_030::winit::event::WindowEvent::Resized(_) = event {
                if let Some(ui) = weak.upgrade() {
                    ui.set_awake(!window.is_minimized());
                }
            }
            slint::winit_030::EventResult::Propagate
        }
    });
    ui.window().on_close_requested(|| {
        let _ = slint::quit_event_loop();
        slint::CloseRequestResponse::HideWindow
    });
}

fn core() -> Option<Arc<Core>> {
    with_app(|app| app.core.clone())
}

fn on_change(change: Change) {
    match change {
        Change::Mix => {
            sync_mix();
            cozy_refresh();
        }
        Change::Playing => {
            with_app(|app| {
                if let Some(ui) = app.ui.upgrade() {
                    ui.set_playing(app.core.playing());
                }
            });
        }
        Change::Master => {
            with_app(|app| {
                if let Some(ui) = app.ui.upgrade() {
                    ui.set_master(app.core.saved().master_volume);
                }
            });
        }
        Change::Look => {
            with_app(|app| {
                refresh_grid(app);
                if let Some(ui) = app.ui.upgrade() {
                    ui.global::<Theme>().set_name(app.core.saved().theme.as_str().into());
                    panels::refresh_settings(app, &ui);
                    panels::refresh_schedule(app, &ui);
                }
            });
            cozy_refresh();
        }
        Change::Toast { text, error } => {
            if error {
                log::warn!("{text}");
            } else {
                log::info!("{text}");
            }
            toast(text, error);
        }
        Change::Timer => tick_timer(),
        Change::Schedule | Change::Presets => {
            with_app(|app| {
                if let Some(ui) = app.ui.upgrade() {
                    panels::refresh_mixes(app, &ui);
                    panels::refresh_schedule(app, &ui);
                }
            });
        }
        Change::Device => {
            with_app(|app| {
                if let Some(ui) = app.ui.upgrade() {
                    panels::refresh_settings(app, &ui);
                    panels::refresh_devices(&ui);
                }
            });
        }
    }
}

fn cozy_toggled(on: bool) {
    let Some(ui) = with_app(|app| {
        if on && app.cozy.is_none() {
            let tiles = Rc::new(VecModel::<CozyTile>::default());
            if let Some(ui) = app.ui.upgrade() {
                ui.set_cozy_tiles(ModelRc::from(tiles.clone()));
            }
            app.cozy = Some(CozyState {
                engine: cozy::Cozy::new(&app.core.scene()),
                tiles,
                panes: true,
                baking: false,
                want: None,
                settle: slint::Timer::default(),
            });
        }
        app.core.set_cozy(on);
        let ui = app.ui.upgrade()?;
        if on {
            ui.set_panel(SharedString::default());
            ui.set_cozy_shown(true);
            let ready = app.cozy.as_ref().is_some_and(|c| c.engine.look().is_some());
            ui.set_cozy_ready(ready);
            if ready {
                slint::Timer::single_shot(Duration::from_millis(340), cozy_cover);
            }
        } else {
            ui.set_cozy_covered(false);
            ui.set_cozy_ready(false);
        }
        Some(ui)
    })
    .flatten() else {
        return;
    };
    if std::env::var_os("NURU_NO_FULLSCREEN").is_none() {
        ui.window().set_fullscreen(on);
    }
    if on {
        cozy_refresh();
    } else {
        slint::Timer::single_shot(Duration::from_millis(340), cozy_gone);
    }
}

fn cozy_cover() {
    with_app(|app| {
        if let Some(ui) = app.ui.upgrade() {
            if ui.get_cozy() && ui.get_cozy_ready() {
                ui.set_cozy_covered(true);
            }
        }
    });
}

fn cozy_gone() {
    with_app(|app| {
        let Some(ui) = app.ui.upgrade() else { return };
        if ui.get_cozy() {
            return;
        }
        app.cozy = None;
        ui.set_cozy_shown(false);
        ui.set_cozy_tiles(ModelRc::default());
        ui.set_cozy_flash(Image::default());
        ui.set_cozy_flash_level(0.0);
    });
}

fn cozy_refresh() {
    let now = with_app(|app| {
        let ui = app.ui.upgrade()?;
        let state = app.cozy.as_mut()?;
        let scene = app.core.scene();
        state.engine.set_scene(&scene);
        cozy_apply(state, &ui);
        let window = ui.window();
        let size = window.size();
        let look = cozy::Look::new(&scene, (size.width, size.height), size.width as f32 / window.scale_factor(), state.panes)?;
        if state.engine.look() == Some(&look) {
            state.want = None;
            return None;
        }
        state.want = Some(look);
        if state.baking {
            return None;
        }
        if state.engine.look().is_none() {
            return Some(true);
        }
        state.settle.start(slint::TimerMode::SingleShot, Duration::from_millis(150), cozy_bake_next);
        Some(false)
    })
    .flatten();
    if now == Some(true) {
        cozy_bake_next();
    }
}

fn cozy_bake_next() {
    with_app(|app| {
        let Some(state) = app.cozy.as_mut() else { return };
        if state.baking {
            return;
        }
        let Some(look) = state.want.take() else { return };
        if state.engine.look() == Some(&look) {
            return;
        }
        state.baking = true;
        let started = std::thread::Builder::new().name("nuru-cozy".into()).spawn(move || {
            let baked = cozy::bake(look);
            let _ = slint::invoke_from_event_loop(move || cozy_baked(baked));
        });
        if let Err(e) = started {
            log::warn!("cozy scene thread did not start: {e}");
            state.baking = false;
        }
    });
}

fn cozy_baked(baked: Option<cozy::Baked>) {
    with_app(|app| {
        let Some(ui) = app.ui.upgrade() else { return };
        let Some(state) = app.cozy.as_mut() else { return };
        state.baking = false;
        if let Some(baked) = baked {
            state.tiles.set_vec(Vec::new());
            state.engine.install(baked);
            let px = state.engine.px();
            let rows: Vec<CozyTile> = state
                .engine
                .tiles()
                .iter()
                .map(|t| CozyTile {
                    x: t.x as f32 / px,
                    y: t.y as f32 / px,
                    w: t.w as f32 / px,
                    h: t.h as f32 / px,
                    image: t.image(),
                })
                .collect();
            state.tiles.set_vec(rows);
            ui.set_cozy_flash(state.engine.flash_image());
            ui.set_cozy_flash_level(0.0);
            if ui.get_cozy() && !ui.get_cozy_ready() {
                ui.set_cozy_ready(true);
                slint::Timer::single_shot(Duration::from_millis(340), cozy_cover);
            }
        }
        cozy_apply(state, &ui);
    });
    cozy_bake_next();
}

fn cozy_apply(state: &mut CozyState, ui: &AppWindow) {
    let model = state.tiles.clone();
    let mut touched = Vec::new();
    let flash = state.engine.tick(Instant::now(), &mut |mask| {
        for (i, _) in mask.iter().enumerate().filter(|(_, on)| **on) {
            if let Some(mut row) = model.row_data(i) {
                row.image = Image::default();
                model.set_row_data(i, row);
                touched.push(i);
            }
        }
    });
    for i in touched {
        if let (Some(mut row), Some(tile)) = (model.row_data(i), state.engine.tiles().get(i)) {
            row.image = tile.image();
            model.set_row_data(i, row);
        }
    }
    if flash {
        ui.set_cozy_flash_level(state.engine.flash_level());
    }
    if let Some(pace) = state.engine.paced() {
        match pace {
            Some(every) => state.engine.timer().start(slint::TimerMode::Repeated, every, cozy_tick),
            None => state.engine.timer().stop(),
        }
    }
}

fn cozy_tick() {
    with_app(|app| {
        let Some(ui) = app.ui.upgrade() else { return };
        if !ui.get_awake() {
            return;
        }
        if let Some(state) = app.cozy.as_mut() {
            cozy_apply(state, &ui);
        }
    });
}

fn sync_mix() {
    let mut recheck = false;
    let mut tint_jobs = Vec::new();
    with_app(|app| {
        let layers = app.core.layers();
        let now = Instant::now();

        let rows: Vec<LayerData> = layers
            .iter()
            .filter_map(|l| {
                let e = app.entries.iter().find(|e| e.sound.id == l.sound_id)?;
                Some(LayerData {
                    id: l.sound_id.as_str().into(),
                    name: e.sound.name.as_str().into(),
                    accent: color(&e.sound.accent),
                    volume: l.volume,
                })
            })
            .collect();
        let same_ids = rows.len() == app.layers.row_count()
            && rows.iter().enumerate().all(|(j, r)| app.layers.row_data(j).is_some_and(|old| old.id == r.id));
        if same_ids {
            for (j, r) in rows.into_iter().enumerate() {
                if app.layers.row_data(j).is_some_and(|old| old.volume != r.volume) {
                    app.layers.set_row_data(j, r);
                }
            }
        } else {
            app.layers.set_vec(rows);
        }

        for i in 0..app.entries.len() {
            let layer = layers.iter().find(|l| l.sound_id == app.entries[i].sound.id);
            let present = layer.is_some();
            let loading = layer.is_some_and(|l| l.loading);

            if !present {
                app.active_images[i] = None;
                app.loading_since[i] = None;
            }
            if loading {
                let since = *app.loading_since[i].get_or_insert(now);
                if now.duration_since(since) < SPINNER_DELAY {
                    recheck = true;
                }
            } else {
                app.loading_since[i] = None;
            }

            let has_cover = app.entries[i].cover_path.is_some();
            if present && !loading && has_cover && app.active_images[i].is_none() && !app.tinting[i] {
                app.tinting[i] = true;
                let e = &app.entries[i];
                tint_jobs.push((i, e.cover_path.clone().unwrap_or_default(), covers::accent_rgb(&e.sound.accent), app.shape));
            }

            let show_active = present && !loading && (!has_cover || app.active_images[i].is_some());
            let show_loading = loading
                && app.loading_since[i].is_some_and(|since| now.duration_since(since) >= SPINNER_DELAY);
            if show_active != app.shown_active[i] || show_loading != app.shown_loading[i] {
                app.shown_active[i] = show_active;
                app.shown_loading[i] = show_loading;
                update_tile(app, i);
            }
        }
    });

    for (i, path, accent, shape) in tint_jobs {
        let spawned = std::thread::Builder::new().name("nuru-tint".into()).spawn(move || {
            let pixels = covers::active(&path, accent, shape);
            let _ = slint::invoke_from_event_loop(move || {
                with_app(|app| {
                    app.tinting[i] = false;
                    if let Some(p) = pixels {
                        app.active_images[i] = Some(Image::from_rgba8(p));
                    }
                });
                sync_mix();
            });
        });
        if spawned.is_err() {
            with_app(|app| app.tinting[i] = false);
        }
    }

    if recheck {
        slint::Timer::single_shot(SPINNER_DELAY, sync_mix);
    }

    with_app(|app| {
        if let Some(ui) = app.ui.upgrade() {
            ui.set_playing(app.core.playing());
            ui.set_playing_count(app.core.layers().len() as i32);
        }
    });
}

fn select_backend() -> Result<()> {
    use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attributes| attributes.with_undecorated_shadow(true))
        .select()
        .context("starting the window system")
}

fn install_effects(ui: &AppWindow, tile_radius: f32, scale: f32) {
    let fx = ui.global::<Effects>();
    fx.set_tile_mask(effects::mask(tile_radius, scale));
    let tile = |blur: f32| effects::shadow(tile_radius, blur, scale);

    let s = tile(4.0);
    fx.set_tile_tight(s.image);
    fx.set_tile_tight_pad(s.pad);
    let s = tile(20.0);
    fx.set_tile_rest(s.image);
    fx.set_tile_rest_pad(s.pad);
    let s = tile(48.0);
    fx.set_tile_lifted(s.image);
    fx.set_tile_lifted_pad(s.pad);
    let s = tile(34.0);
    fx.set_tile_glow(s.image);
    fx.set_tile_glow_pad(s.pad);

    let s = effects::shadow_sized(52.0, 52.0, 26.0, 4.0, scale);
    fx.set_round_tight(s.image);
    fx.set_round_tight_pad(s.pad);
    let s = effects::shadow_sized(52.0, 52.0, 26.0, 20.0, scale);
    fx.set_round_rest(s.image);
    fx.set_round_rest_pad(s.pad);
    let s = effects::shadow_wide(2.0, 1.0, 14.0, scale);
    fx.set_pill_glow(s.image);
    fx.set_pill_glow_pad(s.pad);
    let s = effects::shadow_sized(5.0, 5.0, 2.5, 10.0, scale);
    fx.set_dot_glow(s.image);
    fx.set_dot_glow_pad(s.pad);
    let s = effects::shadow_sized(13.0, 13.0, 6.5, 6.0, scale);
    fx.set_thumb_shadow(s.image);
    fx.set_thumb_shadow_pad(s.pad);
}

fn color(hex: &str) -> Color {
    let [r, g, b] = covers::accent_rgb(hex);
    Color::from_rgb_u8(r, g, b)
}

fn tile_data(app: &App, i: usize) -> TileData {
    let entry = &app.entries[i];
    TileData {
        index: i as i32,
        name: entry.sound.name.as_str().into(),
        accent: color(&entry.sound.accent),
        cover: app.cover_images[i].clone(),
        active_cover: app.active_images[i].clone().unwrap_or_default(),
        active: app.shown_active[i],
        loading: app.shown_loading[i],
    }
}

fn update_tile(app: &App, i: usize) {
    if let Some((g, r)) = app.slots[i] {
        if let Some(model) = app.group_models.get(g) {
            model.set_row_data(r, tile_data(app, i));
        }
    }
}

fn matches(app: &App, i: usize, query: &str) -> bool {
    let sound = &app.entries[i].sound;
    if !app.filter.is_empty() && !sound.tags.iter().any(|t| *t == app.filter) {
        return false;
    }
    if !query.is_empty()
        && !sound.name.to_lowercase().contains(query)
        && !sound.tags.iter().any(|t| t.contains(query))
    {
        return false;
    }
    true
}

fn refresh_grid(app: &mut App) {
    let collapsed = app.core.saved().collapsed_groups;
    let query = app.search.trim().to_lowercase();
    let mut per_group: Vec<Vec<usize>> = vec![Vec::new(); app.group_names.len()];
    for i in 0..app.entries.len() {
        if matches(app, i, &query) {
            per_group[app.group_of[i]].push(i);
        }
    }

    app.slots = vec![None; app.entries.len()];
    let mut models = Vec::new();
    let mut groups = Vec::new();
    for (gi, members) in per_group.iter().enumerate() {
        if members.is_empty() {
            continue;
        }
        let slot = models.len();
        for (row, &i) in members.iter().enumerate() {
            app.slots[i] = Some((slot, row));
        }
        let model = Rc::new(VecModel::from(members.iter().map(|&i| tile_data(app, i)).collect::<Vec<_>>()));
        let name = app.group_names[gi].clone();
        groups.push(GroupData {
            collapsed: collapsed.contains(&name),
            name: name.into(),
            tiles: ModelRc::from(model.clone()),
        });
        models.push(model);
    }
    let shown = groups.len();
    app.group_models = models;
    app.groups.set_vec(groups);

    let empty = if app.entries.is_empty() {
        "No sound packs found. Nuru looks in its resources folder and in your app data folder."
    } else if shown == 0 {
        "Nothing matches that."
    } else {
        ""
    };
    if let Some(ui) = app.ui.upgrade() {
        ui.set_empty_text(empty.into());
    }
}

fn refresh_chips(app: &mut App) {
    let mut shown: Vec<String> = if app.all_tags || app.tags.len() <= VISIBLE_TAGS {
        app.tags.clone()
    } else {
        app.tags[..VISIBLE_TAGS].to_vec()
    };
    if !app.all_tags && app.tags.len() > VISIBLE_TAGS && !app.filter.is_empty() && !shown.contains(&app.filter) {
        if let Some(last) = shown.last_mut() {
            *last = app.filter.clone();
        }
    }
    let hidden = app.tags.len() - shown.len();

    let mut chips = vec![ChipData {
        label: "All".into(),
        tag: SharedString::default(),
        on: app.filter.is_empty(),
        more: false,
    }];
    chips.extend(shown.iter().map(|tag| ChipData {
        label: capitalize(tag).into(),
        tag: tag.as_str().into(),
        on: app.filter == *tag,
        more: false,
    }));
    if hidden > 0 || app.all_tags {
        chips.push(ChipData {
            label: if app.all_tags { "Less".into() } else { format!("+{hidden} more").into() },
            tag: SharedString::default(),
            on: false,
            more: true,
        });
    }
    app.chips.set_vec(chips);
}

fn capitalize(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn command(ui: &AppWindow, line: &str) {
    let mut parts = line.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some("toggle"), Some(n)) => {
            let found = with_app(|app| {
                let entry = match n.parse::<usize>() {
                    Ok(i) => app.entries.get(i),
                    Err(_) => app.entries.iter().find(|e| e.sound.id.as_str() == n),
                };
                entry.map(|e| (app.core.clone(), e.sound.id.clone()))
            });
            if let Some(Some((core, id))) = found {
                core.toggle(&id);
            }
        }
        (Some("hide"), _) => {
            let _ = ui.hide();
        }
        (Some("show"), _) => {
            let _ = ui.show();
        }
        (Some("minimize"), _) => ui.window().set_minimized(true),
        (Some("restore"), _) => ui.window().set_minimized(false),
        (Some("panel"), name) => ui.set_panel(name.unwrap_or("").into()),
        (Some("cozy"), state) => ui.set_cozy(state != Some("off")),
        (Some("panes"), state) => {
            with_app(|app| {
                if let Some(c) = app.cozy.as_mut() {
                    c.panes = state != Some("off");
                }
            });
            cozy_refresh();
        }
        (Some("timer"), arg) => {
            if let Some(core) = core() {
                match arg.and_then(|m| m.parse::<u32>().ok()) {
                    Some(minutes) => core.start_timer(minutes),
                    None => core.stop_timer(),
                }
            }
        }
        (Some("state"), _) => {
            let size = ui.window().size();
            log::info!(
                "window minimized={} size={}x{} awake={} playing={} panel={} timer={}:{} cozy={}",
                ui.window().is_minimized(),
                size.width,
                size.height,
                ui.get_awake(),
                ui.get_playing(),
                ui.get_panel(),
                ui.get_timer_running(),
                ui.get_timer_text(),
                ui.get_cozy_ready()
            );
        }
        (Some("stop"), _) => {
            if let Some(core) = core() {
                core.clear();
            }
        }
        (Some("quit"), _) => {
            let _ = slint::quit_event_loop();
        }
        _ => {}
    }
    control::say(&format!("ok {line}"));
}
