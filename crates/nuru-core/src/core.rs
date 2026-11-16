use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::Timelike;
use parking_lot::{Condvar, Mutex, RwLock};

use crate::audio::{AudioEngine, OnLost};
use crate::discord::{now_secs, Presence, Status};
use crate::legacy;
use crate::library::Library;
use crate::paths::Paths;
use crate::persist::{new_id, read_json, write_json};
use crate::presence::Lines;
use crate::saved::{
    Preset, PresetFile, Saved, SavedLayer, ScheduleFile, Settings, TileSize, PRESETS_FILE, SCHEDULE_FILE,
    SETTINGS_FILE, STATE_FILE,
};
use crate::scene::{self, Scene};
use crate::schedule::{self, SLOTS};

pub const DEFAULT_SOUND_VOLUME: f32 = 0.55;
const SAVE_DELAY: Duration = Duration::from_millis(400);
const PRESENCE_DELAY: Duration = Duration::from_millis(1200);
const IDLE_WAKE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Mix,
    Playing,
    Master,
    Timer,
    Schedule,
    Presets,
    Look,
    Device,
    Toast { text: String, error: bool },
}

pub type Sink = Arc<dyn Fn(Change) + Send + Sync>;

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub sound_id: String,
    pub volume: f32,
    pub loading: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Timer {
    Off,
    Running { ends_at_ms: i64, total_ms: i64 },
    Finished,
}

#[derive(Default, Clone, Copy)]
struct Dirty {
    state: bool,
    presets: bool,
    schedule: bool,
}

struct Inner {
    saved: Saved,
    presets: Vec<Preset>,
    slots: Vec<Option<String>>,
    layers: Vec<Layer>,
    playing: bool,
    timer: Timer,
    cozy: bool,
    ready: bool,
    schedule_seen: Option<usize>,
    schedule_applied: Option<Option<String>>,
    presence_started: Option<i64>,
    presence_due: Option<Instant>,
    save_due: Option<Instant>,
    dirty: Dirty,
    lines: Lines,
    woken: bool,
    stop: bool,
}

pub struct Core {
    paths: Paths,
    library: Arc<Library>,
    engine: RwLock<Arc<AudioEngine>>,
    settings: Mutex<Settings>,
    presence: Presence,
    inner: Mutex<Inner>,
    wake: Condvar,
    sink: Sink,
    ticker: Mutex<Option<JoinHandle<()>>>,
    link: Arc<OnceLock<Weak<Core>>>,
    recovering: AtomicBool,
}

fn lost_handler(link: &Arc<OnceLock<Weak<Core>>>) -> OnLost {
    let link = link.clone();
    Arc::new(move || {
        if let Some(core) = link.get().and_then(Weak::upgrade) {
            core.output_lost();
        }
    })
}

impl Core {
    pub fn open(paths: Paths, library: Arc<Library>, sink: Sink) -> Result<Arc<Core>> {
        let settings: Settings = read_json(&paths.file(SETTINGS_FILE)).unwrap_or_default();
        let (saved, presets, slots, fresh) = load_saved(&paths);

        let link: Arc<OnceLock<Weak<Core>>> = Arc::new(OnceLock::new());
        let engine = AudioEngine::open(settings.output_device.as_deref(), Some(lost_handler(&link))).context("opening the audio output")?;
        log::info!(
            "audio out: {} @ {} Hz, {} ch",
            engine.device.name,
            engine.device.sample_rate,
            engine.device.channels
        );
        engine.set_master(saved.master_volume)?;

        let core = Arc::new(Core {
            paths,
            library,
            engine: RwLock::new(Arc::new(engine)),
            settings: Mutex::new(settings),
            presence: Presence::start(),
            inner: Mutex::new(Inner {
                saved,
                presets,
                slots,
                layers: Vec::new(),
                playing: true,
                timer: Timer::Off,
                cozy: false,
                ready: false,
                schedule_seen: None,
                schedule_applied: None,
                presence_started: None,
                presence_due: None,
                save_due: None,
                dirty: Dirty { state: fresh, presets: fresh, schedule: fresh },
                lines: Lines::default(),
                woken: false,
                stop: false,
            }),
            wake: Condvar::new(),
            sink,
            ticker: Mutex::new(None),
            link: link.clone(),
            recovering: AtomicBool::new(false),
        });
        let _ = link.set(Arc::downgrade(&core));

        if fresh {
            core.flush();
        }

        let ticking = core.clone();
        let handle = std::thread::Builder::new()
            .name("nuru-ticker".into())
            .spawn(move || ticking.run_ticker())
            .context("starting the ticker")?;
        *core.ticker.lock() = Some(handle);
        Ok(core)
    }

    pub fn shutdown(&self) {
        {
            let mut inner = self.inner.lock();
            inner.stop = true;
            inner.woken = true;
        }
        self.wake.notify_all();
        if let Some(handle) = self.ticker.lock().take() {
            let _ = handle.join();
        }
        self.flush();
        let _ = self.engine().clear();
        self.presence.set(Status::default());
    }

    pub fn library(&self) -> &Arc<Library> {
        &self.library
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    fn engine(&self) -> Arc<AudioEngine> {
        self.engine.read().clone()
    }

    fn emit(&self, change: Change) {
        (self.sink)(change);
    }

    fn toast(&self, text: impl Into<String>, error: bool) {
        self.emit(Change::Toast { text: text.into(), error });
    }

    fn nudge(&self, inner: &mut Inner) {
        inner.woken = true;
        self.wake.notify_all();
    }

    fn touch_state(&self, inner: &mut Inner) {
        inner.dirty.state = true;
        inner.save_due = Some(Instant::now() + SAVE_DELAY);
        self.nudge(inner);
    }

    fn touch_presets(&self, inner: &mut Inner) {
        inner.dirty.presets = true;
        inner.save_due = Some(Instant::now() + SAVE_DELAY);
        self.nudge(inner);
    }

    fn touch_schedule(&self, inner: &mut Inner) {
        inner.dirty.schedule = true;
        inner.save_due = Some(Instant::now() + SAVE_DELAY);
        self.nudge(inner);
    }

    fn touch_presence(&self, inner: &mut Inner) {
        inner.presence_due = Some(Instant::now() + PRESENCE_DELAY);
        self.nudge(inner);
    }

    pub fn device_name(&self) -> String {
        self.engine().device.name.clone()
    }

    pub fn device_note(&self) -> String {
        let e = self.engine();
        if e.device.sample_rate > 0 {
            format!("{} - {:.1} kHz - {} ch", e.device.name, e.device.sample_rate as f32 / 1000.0, e.device.channels)
        } else {
            e.device.name.clone()
        }
    }

    pub fn layers(&self) -> Vec<Layer> {
        self.inner.lock().layers.clone()
    }

    pub fn playing(&self) -> bool {
        self.inner.lock().playing
    }

    pub fn audible(&self) -> bool {
        let inner = self.inner.lock();
        inner.playing && !inner.layers.is_empty()
    }

    pub fn saved(&self) -> Saved {
        self.inner.lock().saved.clone()
    }

    pub fn presets(&self) -> Vec<Preset> {
        self.inner.lock().presets.clone()
    }

    pub fn slots(&self) -> Vec<Option<String>> {
        self.inner.lock().slots.clone()
    }

    pub fn timer(&self) -> Timer {
        self.inner.lock().timer
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().clone()
    }

    pub fn scene(&self) -> Scene {
        let inner = self.inner.lock();
        self.scene_of(&inner)
    }

    fn scene_of(&self, inner: &Inner) -> Scene {
        let roles: Vec<_> = inner
            .layers
            .iter()
            .filter_map(|l| self.library.sound(&l.sound_id).map(|s| (s.nook, l.volume)))
            .collect();
        scene::compute(roles.iter().map(|(r, v)| (r, *v)), inner.saved.scene_follows_volume)
    }

    pub fn toggle(self: &Arc<Self>, sound_id: &str) {
        let on = self.inner.lock().layers.iter().any(|l| l.sound_id == sound_id);
        if on {
            self.remove(sound_id);
        } else {
            self.add(sound_id, None);
        }
    }

    pub fn add(self: &Arc<Self>, sound_id: &str, volume: Option<f32>) {
        let Some(sound) = self.library.sound(sound_id) else { return };
        let (volume, resumed) = {
            let mut inner = self.inner.lock();
            if inner.layers.iter().any(|l| l.sound_id == sound_id) {
                return;
            }
            let volume = volume
                .or_else(|| inner.saved.sound_volumes.get(sound_id).copied())
                .unwrap_or(DEFAULT_SOUND_VOLUME)
                .clamp(0.0, 1.0);
            inner.layers.push(Layer { sound_id: sound_id.to_string(), volume, loading: true });
            let resumed = !inner.playing;
            inner.playing = true;
            self.touch_state(&mut inner);
            self.touch_presence(&mut inner);
            (volume, resumed)
        };
        if resumed {
            let _ = self.engine().set_playing(true);
            self.emit(Change::Playing);
        }
        self.emit(Change::Mix);

        let core = self.clone();
        let id = sound_id.to_string();
        let name = sound.name.clone();
        let spawned = std::thread::Builder::new().name("nuru-start".into()).spawn(move || {
            let engine = core.engine();
            let result = core.library.ensure_local(&id).and_then(|(sound, path)| {
                engine.add_layer(
                    &id,
                    path,
                    sound.loop_points.start_sample,
                    sound.loop_points.end_sample,
                    sound.loop_points.crossfade_ms,
                    volume,
                )
            });
            core.finish_add(&id, &name, result);
        });
        if let Err(e) = spawned {
            self.finish_add(sound_id, &sound.name, Err(e.into()));
        }
    }

    fn finish_add(&self, sound_id: &str, name: &str, result: Result<()>) {
        let mut orphan = false;
        let mut failed = None;
        {
            let mut inner = self.inner.lock();
            let at = inner.layers.iter().position(|l| l.sound_id == sound_id);
            match (&result, at) {
                (Ok(()), Some(i)) => inner.layers[i].loading = false,
                (Ok(()), None) => orphan = true,
                (Err(e), Some(i)) => {
                    inner.layers.remove(i);
                    self.touch_state(&mut inner);
                    self.touch_presence(&mut inner);
                    failed = Some(format!("{e:#}"));
                }
                (Err(_), None) => {}
            }
        }
        if orphan {
            let _ = self.engine().remove_layer(sound_id);
            return;
        }
        if let Some(e) = failed {
            log::error!("could not start {sound_id}: {e}");
            self.toast(format!("{name} could not be loaded - {e}"), true);
        }
        self.emit(Change::Mix);
    }

    pub fn remove(&self, sound_id: &str) {
        let emptied = {
            let mut inner = self.inner.lock();
            let before = inner.layers.len();
            inner.layers.retain(|l| l.sound_id != sound_id);
            if inner.layers.len() == before {
                return;
            }
            let emptied = inner.layers.is_empty();
            if emptied {
                inner.playing = true;
            }
            self.touch_state(&mut inner);
            self.touch_presence(&mut inner);
            emptied
        };
        let engine = self.engine();
        let _ = engine.remove_layer(sound_id);
        if emptied {
            let _ = engine.set_playing(true);
            self.emit(Change::Playing);
        }
        self.emit(Change::Mix);
    }

    pub fn set_volume(&self, sound_id: &str, volume: f32) {
        let v = volume.clamp(0.0, 1.0);
        {
            let mut inner = self.inner.lock();
            for l in inner.layers.iter_mut().filter(|l| l.sound_id == sound_id) {
                l.volume = v;
            }
            inner.saved.sound_volumes.insert(sound_id.to_string(), v);
            self.touch_state(&mut inner);
            if inner.saved.scene_follows_volume {
                self.touch_presence(&mut inner);
            }
        }
        let _ = self.engine().set_layer_fader(sound_id, v);
        self.emit(Change::Mix);
    }

    pub fn set_master(&self, volume: f32) {
        let v = volume.clamp(0.0, 1.0);
        {
            let mut inner = self.inner.lock();
            inner.saved.master_volume = v;
            self.touch_state(&mut inner);
        }
        let _ = self.engine().set_master(v);
        self.emit(Change::Master);
    }

    pub fn toggle_playing(&self) {
        let playing = {
            let mut inner = self.inner.lock();
            if inner.layers.is_empty() {
                return;
            }
            inner.playing = !inner.playing;
            self.touch_presence(&mut inner);
            inner.playing
        };
        let _ = self.engine().set_playing(playing);
        self.emit(Change::Playing);
    }

    pub fn clear(&self) {
        {
            let mut inner = self.inner.lock();
            inner.layers.clear();
            inner.playing = true;
            self.touch_state(&mut inner);
            self.touch_presence(&mut inner);
        }
        let engine = self.engine();
        let _ = engine.clear();
        let _ = engine.set_playing(true);
        self.emit(Change::Mix);
        self.emit(Change::Playing);
    }

    pub fn restore_last_mix(self: &Arc<Self>) {
        let wanted = {
            let inner = self.inner.lock();
            if inner.saved.restore_on_launch { inner.saved.layers.clone() } else { Vec::new() }
        };
        let mut restored = false;
        for l in wanted.iter().filter(|l| self.library.sound(&l.sound_id).is_some()) {
            self.add(&l.sound_id, Some(l.volume));
            restored = true;
        }
        {
            let mut inner = self.inner.lock();
            if restored {
                inner.playing = false;
            }
            inner.ready = true;
            self.touch_presence(&mut inner);
        }
        if restored {
            let _ = self.engine().set_playing(false);
            self.emit(Change::Playing);
        }
    }

    pub fn set_cozy(&self, cozy: bool) {
        let mut inner = self.inner.lock();
        if inner.cozy != cozy {
            inner.cozy = cozy;
            self.touch_presence(&mut inner);
        }
    }

    pub fn save_preset(&self, name: &str) {
        let trimmed = name.trim();
        {
            let mut inner = self.inner.lock();
            if trimmed.is_empty() || inner.layers.is_empty() {
                return;
            }
            let preset = Preset {
                id: new_id(),
                name: trimmed.to_string(),
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                layers: inner
                    .layers
                    .iter()
                    .map(|l| SavedLayer { sound_id: l.sound_id.clone(), volume: l.volume })
                    .collect(),
                master_volume: Some(inner.saved.master_volume),
            };
            inner.presets.retain(|p| p.name != trimmed);
            inner.presets.insert(0, preset);
            self.touch_presets(&mut inner);
        }
        self.emit(Change::Presets);
        self.toast(format!("Saved \"{trimmed}\""), false);
    }

    pub fn delete_preset(&self, id: &str) {
        let cleared = {
            let mut inner = self.inner.lock();
            inner.presets.retain(|p| p.id != id);
            self.touch_presets(&mut inner);
            self.clear_slots_for(&mut inner, id)
        };
        self.emit(Change::Presets);
        if cleared {
            self.emit(Change::Schedule);
        }
    }

    pub fn load_preset(self: &Arc<Self>, id: &str, note: Option<String>) {
        let Some(preset) = self.inner.lock().presets.iter().find(|p| p.id == id).cloned() else { return };
        self.clear();
        if let Some(m) = preset.master_volume {
            self.set_master(m);
        }
        for l in &preset.layers {
            self.add(&l.sound_id, Some(l.volume));
        }
        self.toast(note.unwrap_or_else(|| format!("Loaded \"{}\"", preset.name)), false);
    }

    pub fn start_timer(&self, minutes: u32) {
        {
            let mut inner = self.inner.lock();
            let total_ms = i64::from(minutes) * 60_000;
            inner.timer = Timer::Running { ends_at_ms: now_ms() + total_ms, total_ms };
            self.nudge(&mut inner);
        }
        self.emit(Change::Timer);
    }

    pub fn stop_timer(&self) {
        {
            let mut inner = self.inner.lock();
            if matches!(inner.timer, Timer::Running { .. }) {
                inner.timer = Timer::Off;
            }
        }
        self.emit(Change::Timer);
    }

    pub fn set_schedule_enabled(&self, on: bool) {
        {
            let mut inner = self.inner.lock();
            inner.saved.schedule_enabled = on;
            inner.schedule_seen = None;
            inner.schedule_applied = None;
            self.touch_state(&mut inner);
        }
        self.emit(Change::Schedule);
    }

    pub fn set_slot(&self, slot: usize, preset: Option<String>) {
        if slot >= SLOTS {
            return;
        }
        {
            let mut inner = self.inner.lock();
            inner.slots[slot] = preset;
            inner.schedule_seen = None;
            self.touch_schedule(&mut inner);
        }
        self.emit(Change::Schedule);
    }

    pub fn clear_schedule(&self) {
        {
            let mut inner = self.inner.lock();
            inner.slots = vec![None; SLOTS];
            inner.schedule_seen = None;
            inner.schedule_applied = None;
            self.touch_schedule(&mut inner);
        }
        self.emit(Change::Schedule);
    }

    fn clear_slots_for(&self, inner: &mut Inner, id: &str) -> bool {
        if !inner.slots.iter().any(|s| s.as_deref() == Some(id)) {
            return false;
        }
        for s in inner.slots.iter_mut().filter(|s| s.as_deref() == Some(id)) {
            *s = None;
        }
        self.touch_schedule(inner);
        true
    }

    pub fn set_theme(&self, theme: &str) {
        self.look(|s| s.theme = theme.to_string());
    }

    pub fn set_tile_size(&self, size: TileSize) {
        self.look(|s| s.tile_size = size);
    }

    pub fn set_restore_on_launch(&self, on: bool) {
        self.look(|s| s.restore_on_launch = on);
    }

    pub fn set_scene_follows_volume(&self, on: bool) {
        self.look(|s| s.scene_follows_volume = on);
        let mut inner = self.inner.lock();
        self.touch_presence(&mut inner);
    }

    pub fn toggle_group(&self, name: &str) {
        self.look(|s| match s.collapsed_groups.iter().position(|g| g == name) {
            Some(i) => {
                s.collapsed_groups.remove(i);
            }
            None => s.collapsed_groups.push(name.to_string()),
        });
    }

    fn look(&self, f: impl FnOnce(&mut Saved)) {
        {
            let mut inner = self.inner.lock();
            f(&mut inner.saved);
            self.touch_state(&mut inner);
        }
        self.emit(Change::Look);
    }

    pub fn set_auto_update(&self, on: bool) {
        let mut s = self.settings.lock();
        s.auto_update = on;
        self.write_settings(&s);
    }

    pub fn mark_tray_notice_shown(&self) {
        let mut s = self.settings.lock();
        s.tray_notice_shown = true;
        self.write_settings(&s);
    }

    pub fn set_output_device(self: &Arc<Self>, device: Option<String>) {
        match AudioEngine::open(device.as_deref(), Some(lost_handler(&self.link))) {
            Ok(engine) => {
                {
                    let mut s = self.settings.lock();
                    s.output_device = device;
                    self.write_settings(&s);
                }
                self.adopt(engine);
            }
            Err(e) => self.toast(format!("Could not switch output: {e:#}"), true),
        }
    }

    fn output_lost(self: &Arc<Self>) {
        if self.recovering.swap(true, Ordering::SeqCst) {
            return;
        }
        let core = self.clone();
        let started = std::thread::Builder::new().name("nuru-device".into()).spawn(move || {
            for attempt in 0..60 {
                std::thread::sleep(Duration::from_millis(if attempt == 0 { 400 } else { 2000 }));
                let wanted = core.settings().output_device;
                match AudioEngine::open(wanted.as_deref(), Some(lost_handler(&core.link))) {
                    Ok(engine) => {
                        core.adopt(engine);
                        break;
                    }
                    Err(e) => {
                        if attempt == 0 {
                            log::warn!("the audio output went away and none is available yet: {e:#}");
                        }
                    }
                }
            }
            core.recovering.store(false, Ordering::SeqCst);
        });
        if started.is_err() {
            self.recovering.store(false, Ordering::SeqCst);
        }
    }

    fn adopt(self: &Arc<Self>, engine: AudioEngine) {
        let name = engine.device.name.clone();
        let (layers, playing, master) = {
            let inner = self.inner.lock();
            (inner.layers.clone(), inner.playing, inner.saved.master_volume)
        };
        let _ = engine.set_master(master);
        *self.engine.write() = Arc::new(engine);
        log::info!("output device switched to {name}");
        {
            let mut inner = self.inner.lock();
            inner.layers.clear();
        }
        for l in &layers {
            self.add(&l.sound_id, Some(l.volume));
        }
        if !playing && !layers.is_empty() {
            self.inner.lock().playing = false;
            let _ = self.engine().set_playing(false);
            self.emit(Change::Playing);
        }
        self.emit(Change::Device);
        self.toast(format!("Output set to {name}"), false);
    }

    fn write_settings(&self, settings: &Settings) {
        if let Err(e) = write_json(&self.paths.file(SETTINGS_FILE), settings) {
            log::warn!("could not write settings: {e:#}");
        }
    }

    pub fn flush(&self) {
        let (dirty, saved, presets, slots) = {
            let mut inner = self.inner.lock();
            let dirty = inner.dirty;
            inner.dirty = Dirty::default();
            inner.save_due = None;
            if inner.ready {
                inner.saved.layers = inner
                    .layers
                    .iter()
                    .map(|l| SavedLayer { sound_id: l.sound_id.clone(), volume: l.volume })
                    .collect();
            }
            (dirty, inner.saved.clone(), inner.presets.clone(), inner.slots.clone())
        };
        let write = |name: &str, result: Result<()>| {
            if let Err(e) = result {
                log::warn!("could not save {name}: {e:#}");
            }
        };
        if dirty.state {
            write(STATE_FILE, write_json(&self.paths.file(STATE_FILE), &saved));
        }
        if dirty.presets {
            write(PRESETS_FILE, write_json(&self.paths.file(PRESETS_FILE), &PresetFile { items: presets }));
        }
        if dirty.schedule {
            write(SCHEDULE_FILE, write_json(&self.paths.file(SCHEDULE_FILE), &ScheduleFile { slots }));
        }
    }

    fn run_ticker(self: Arc<Self>) {
        loop {
            let wait = self.tick();
            let mut inner = self.inner.lock();
            if inner.stop {
                return;
            }
            if !inner.woken {
                self.wake.wait_for(&mut inner, wait);
            }
            inner.woken = false;
            if inner.stop {
                return;
            }
        }
    }

    fn tick(self: &Arc<Self>) -> Duration {
        let mut wait = IDLE_WAKE;
        wait = wait.min(self.tick_timer());
        wait = wait.min(self.tick_schedule());
        wait = wait.min(self.tick_presence());
        wait.min(self.tick_save())
    }

    fn tick_timer(&self) -> Duration {
        let finished = {
            let mut inner = self.inner.lock();
            let Timer::Running { ends_at_ms, .. } = inner.timer else { return IDLE_WAKE };
            let left = ends_at_ms - now_ms();
            if left > 0 {
                return Duration::from_millis(left as u64);
            }
            inner.timer = Timer::Finished;
            inner.playing = false;
            self.touch_presence(&mut inner);
            true
        };
        if finished {
            let _ = self.engine().set_playing(false);
            self.emit(Change::Timer);
            self.emit(Change::Playing);
            self.toast("Timer finished - playback paused", false);
        }
        IDLE_WAKE
    }

    fn tick_schedule(self: &Arc<Self>) -> Duration {
        let now = chrono::Local::now();
        let wait = Duration::from_secs(u64::from(schedule::seconds_until_next_slot(now.minute(), now.second())) + 1);
        let slot = schedule::slot_at(now.hour(), now.minute());

        enum Act {
            Pause(String),
            Load(String, String),
            Forget(String),
        }

        let act = {
            let mut inner = self.inner.lock();
            if !inner.ready {
                return wait;
            }
            if !inner.saved.schedule_enabled {
                inner.schedule_seen = None;
                inner.schedule_applied = None;
                return IDLE_WAKE;
            }
            if inner.schedule_seen == Some(slot) {
                return wait;
            }
            inner.schedule_seen = Some(slot);
            let wanted = inner.slots[slot].clone();
            if inner.schedule_applied.as_ref() == Some(&wanted) {
                return wait;
            }
            inner.schedule_applied = Some(wanted.clone());
            match wanted {
                None => {
                    if inner.playing && !inner.layers.is_empty() {
                        Some(Act::Pause(schedule::slot_label(slot)))
                    } else {
                        None
                    }
                }
                Some(id) => match inner.presets.iter().find(|p| p.id == id) {
                    Some(p) => {
                        let ends = schedule::block(&inner.slots, slot).ends_at;
                        Some(Act::Load(id.clone(), format!("Schedule: {} until {ends}", p.name)))
                    }
                    None => Some(Act::Forget(id)),
                },
            }
        };

        match act {
            Some(Act::Pause(label)) => {
                {
                    let mut inner = self.inner.lock();
                    inner.playing = false;
                    self.touch_presence(&mut inner);
                }
                let _ = self.engine().set_playing(false);
                self.emit(Change::Playing);
                self.toast(format!("Schedule: nothing set for {label}, paused"), false);
            }
            Some(Act::Load(id, note)) => self.load_preset(&id, Some(note)),
            Some(Act::Forget(id)) => {
                let cleared = {
                    let mut inner = self.inner.lock();
                    self.clear_slots_for(&mut inner, &id)
                };
                if cleared {
                    self.emit(Change::Schedule);
                }
            }
            None => {}
        }
        wait
    }

    fn tick_presence(&self) -> Duration {
        let status = {
            let mut inner = self.inner.lock();
            let Some(due) = inner.presence_due else { return IDLE_WAKE };
            let now = Instant::now();
            if due > now {
                return due - now;
            }
            inner.presence_due = None;
            let audible = inner.playing && !inner.layers.is_empty();
            if audible && inner.presence_started.is_none() {
                inner.presence_started = Some(now_secs());
            } else if !audible {
                inner.presence_started = None;
            }
            let scene = self.scene_of(&inner);
            let sounds: Vec<_> = inner.layers.iter().filter_map(|l| self.library.sound(&l.sound_id)).collect();
            let all_noise = sounds.iter().all(|s| s.tags.iter().any(|t| t == "noise"));
            let cozy = inner.cozy;
            let started_at = inner.presence_started;
            let (details, state) = inner.lines.text(&scene, sounds.len(), all_noise, cozy);
            Status { details, state, started_at, active: audible }
        };
        self.presence.set(status);
        IDLE_WAKE
    }

    fn tick_save(&self) -> Duration {
        let due = self.inner.lock().save_due;
        match due {
            Some(due) if due > Instant::now() => due - Instant::now(),
            Some(_) => {
                self.flush();
                IDLE_WAKE
            }
            None => IDLE_WAKE,
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn load_saved(paths: &Paths) -> (Saved, Vec<Preset>, Vec<Option<String>>, bool) {
    let state_path = paths.file(STATE_FILE);
    if state_path.is_file() {
        let saved = read_json::<Saved>(&state_path).unwrap_or_default();
        let presets = read_json::<PresetFile>(&paths.file(PRESETS_FILE)).unwrap_or_default().items;
        let slots = read_json::<ScheduleFile>(&paths.file(SCHEDULE_FILE))
            .map(ScheduleFile::normalised)
            .unwrap_or_else(|| vec![None; SLOTS]);
        return (saved, presets, slots, false);
    }

    let old = legacy::read_local_storage(&paths.legacy_storage);
    let parse = |key: &str| old.get(key).map(String::as_str);
    let saved = parse("nuru.state.v1").and_then(|raw| serde_json::from_str::<Saved>(raw).ok()).unwrap_or_default();
    let presets = parse("nuru.presets.v1")
        .and_then(|raw| serde_json::from_str::<PresetFile>(raw).ok())
        .unwrap_or_default()
        .items;
    let slots = parse("nuru.schedule.v1")
        .and_then(|raw| serde_json::from_str::<ScheduleFile>(raw).ok())
        .map(ScheduleFile::normalised)
        .unwrap_or_else(|| vec![None; SLOTS]);
    if !old.is_empty() {
        log::info!(
            "imported the old settings: {} mix(es), {} scheduled slot(s), {} layer(s) to restore",
            presets.len(),
            slots.iter().filter(|s| s.is_some()).count(),
            saved.layers.len()
        );
    }
    (saved, presets, slots, true)
}
