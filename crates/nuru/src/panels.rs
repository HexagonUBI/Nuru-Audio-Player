use std::collections::BTreeMap;
use std::rc::Rc;

use nuru_core::core::Timer;
use nuru_core::schedule::{self, SLOTS};
use slint::{Color, ModelRc, SharedString, VecModel};

use crate::{color, App, AppWindow, CreditData, EntryData, PresetData, SlotData};

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn local_slot() -> usize {
    use chrono::Timelike;
    let now = chrono::Local::now();
    schedule::slot_at(now.hour(), now.minute())
}

fn accent_of(app: &App, preset_id: &str) -> Color {
    app.core
        .presets()
        .iter()
        .find(|p| p.id == preset_id)
        .and_then(|p| p.layers.first())
        .and_then(|l| app.entries.iter().find(|e| e.sound.id == l.sound_id))
        .map(|e| color(&e.sound.accent))
        .unwrap_or_else(|| Color::from_rgb_u8(0xff, 0xb4, 0x54))
}

fn name_of(app: &App, preset_id: Option<&str>) -> String {
    match preset_id {
        None => "Nothing".into(),
        Some(id) => app.core.presets().iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_else(|| "Missing mix".into()),
    }
}

pub fn refresh_mixes(app: &App, ui: &AppWindow) {
    let rows: Vec<PresetData> = app
        .core
        .presets()
        .iter()
        .map(|p| PresetData {
            id: p.id.as_str().into(),
            name: p.name.as_str().into(),
            count: p.layers.len() as i32,
            dots: ModelRc::from(Rc::new(VecModel::from(
                p.layers
                    .iter()
                    .take(4)
                    .filter_map(|l| app.entries.iter().find(|e| e.sound.id == l.sound_id))
                    .map(|e| color(&e.sound.accent))
                    .collect::<Vec<_>>(),
            ))),
            accent: accent_of(app, &p.id),
        })
        .collect();
    ui.set_presets(ModelRc::from(Rc::new(VecModel::from(rows))));
    ui.set_playing_count(app.core.layers().len() as i32);
}

pub fn timer_label(timer: Timer) -> Option<(String, f32)> {
    let Timer::Running { ends_at_ms, total_ms } = timer else { return None };
    let left = (ends_at_ms - now_ms()).max(0);
    let s = (left + 999) / 1000;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    let text = if h > 0 { format!("{h}:{m:02}:{sec:02}") } else { format!("{m}:{sec:02}") };
    let progress = if total_ms > 0 { 1.0 - left as f32 / total_ms as f32 } else { 0.0 };
    Some((text, progress))
}

pub fn refresh_timer(app: &App, ui: &AppWindow) {
    match timer_label(app.core.timer()) {
        Some((text, progress)) => {
            ui.set_timer_running(true);
            ui.set_timer_text(text.into());
            ui.set_timer_progress(progress);
        }
        None => {
            ui.set_timer_running(false);
            ui.set_timer_text(SharedString::default());
            ui.set_timer_progress(0.0);
        }
    }
}

pub fn refresh_schedule(app: &mut App, ui: &AppWindow) {
    let presets = app.core.presets();
    if app.brush.as_ref().is_none_or(|b| !presets.iter().any(|p| &p.id == b)) {
        app.brush = presets.first().map(|p| p.id.clone());
    }
    let slots = app.core.slots();
    let saved = app.core.saved();
    let live = local_slot();

    let rows: Vec<SlotData> = (0..SLOTS)
        .map(|i| {
            let id = slots[i].as_deref();
            SlotData {
                filled: id.is_some(),
                color: id.map(|id| accent_of(app, id)).unwrap_or(Color::from_argb_u8(0, 0, 0, 0)),
                label: schedule::slot_label(i).into(),
                name: name_of(app, id).into(),
            }
        })
        .collect();
    ui.set_slots(ModelRc::from(Rc::new(VecModel::from(rows))));
    ui.set_live_slot(live as i32);
    ui.set_schedule_on(saved.schedule_enabled);
    ui.set_brush_id(app.brush.clone().unwrap_or_default().into());
    ui.set_brush_name(name_of(app, app.brush.as_deref()).into());

    let block = schedule::block(&slots, live);
    match block.id.as_deref() {
        Some(id) => {
            ui.set_now_name(name_of(app, Some(id)).into());
            ui.set_now_color(accent_of(app, id));
            ui.set_now_until(block.ends_at.into());
        }
        None => {
            ui.set_now_name(SharedString::default());
            ui.set_now_until(SharedString::default());
        }
    }
    let next = schedule::next_change(&slots, live).map(|n| {
        let what = match n.id.as_deref() {
            None => "stops".to_string(),
            Some(id) => presets.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_else(|| "a deleted mix".into()),
        };
        format!("Next at {}: {what}", n.at)
    });
    ui.set_next_line(next.unwrap_or_default().into());

    let filled = slots.iter().filter(|s| s.is_some()).count();
    let hours = filled as f32 * 0.5;
    ui.set_hours_filled(hours);
    ui.set_filled_label(if filled == 0 { "Nothing placed".into() } else { format!("{hours:.1} hours filled").into() });
}

pub fn refresh_settings(app: &App, ui: &AppWindow) {
    let saved = app.core.saved();
    let settings = app.core.settings();
    ui.set_theme_id(saved.theme.as_str().into());
    ui.set_tile_size_id(tile_id(saved.tile_size).into());
    ui.set_restore(saved.restore_on_launch);
    ui.set_follows(saved.scene_follows_volume);
    ui.set_device(settings.output_device.clone().unwrap_or_default().into());
    ui.set_playing_through(app.core.device_note().into());
    ui.set_auto_update(settings.auto_update);
    ui.set_placeholders(app.core.library().unshippable().len() as i32);
}

pub fn refresh_devices(ui: &AppWindow) {
    let names: Vec<SharedString> = nuru_core::audio::output_devices().into_iter().map(Into::into).collect();
    ui.set_devices(ModelRc::from(Rc::new(VecModel::from(names))));
}

pub fn tile_id(size: nuru_core::saved::TileSize) -> &'static str {
    use nuru_core::saved::TileSize;
    match size {
        TileSize::Compact => "compact",
        TileSize::Small => "small",
        TileSize::Normal => "normal",
        TileSize::Large => "large",
    }
}

pub fn tile_from(id: &str) -> nuru_core::saved::TileSize {
    use nuru_core::saved::TileSize;
    match id {
        "compact" => TileSize::Compact,
        "small" => TileSize::Small,
        "large" => TileSize::Large,
        _ => TileSize::Normal,
    }
}

pub fn entries(release: &nuru_core::update::Release) -> ModelRc<EntryData> {
    ModelRc::from(Rc::new(VecModel::from(
        release
            .entries
            .iter()
            .map(|e| EntryData { title: e.title.as_str().into(), text: e.text.as_str().into() })
            .collect::<Vec<_>>(),
    )))
}

pub fn refresh_credits(app: &App, ui: &AppWindow) {
    let mut groups: BTreeMap<(bool, String, String, String), Vec<String>> = BTreeMap::new();
    let mut packs = std::collections::BTreeSet::new();
    for e in &app.entries {
        let p = &e.sound.provenance;
        packs.insert(e.pack_name.clone());
        groups
            .entry((!p.shippable, e.pack_name.clone(), p.licence.clone(), p.attribution.clone().unwrap_or_default()))
            .or_default()
            .push(e.sound.name.clone());
    }
    let rows: Vec<CreditData> = groups
        .into_iter()
        .map(|((dev, pack, licence, attribution), names)| CreditData {
            pack: pack.into(),
            licence: licence.into(),
            attribution: attribution.into(),
            names: names.join(", ").into(),
            dev,
        })
        .collect();
    ui.set_credits(ModelRc::from(Rc::new(VecModel::from(rows))));
    ui.set_sound_count(app.entries.len() as i32);
    ui.set_pack_count(packs.len() as i32);
}
