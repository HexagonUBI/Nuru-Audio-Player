use std::cell::Cell;
use std::time::{Duration, Instant};

use slint::{Color, ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::AppWindow;

const MIN_BOOT: Duration = Duration::from_millis(1100);
const GUARD: Duration = Duration::from_secs(20);
const FADE: Duration = Duration::from_millis(450);
const ERROR_HOLD: Duration = Duration::from_millis(3200);

const ACCENTS: [u32; 12] = [
    0xffb454, 0x5ae3ff, 0x74dc74, 0xf7896b, 0xd5b5f6, 0x62e6a7, 0xf4bf71, 0x779cff, 0xf16b79, 0x44facc, 0xfcf44c, 0xabb5d5,
];

const LINES: [&str; 24] = [
    "Warming up the rain",
    "Tuning the crickets",
    "Stacking loops end to end",
    "Counting samples, twice",
    "Politely asking the wind to loop",
    "Folding the seams away",
    "Lighting the fire",
    "Measuring the quiet",
    "Rounding every corner",
    "Sixteen bits at a time",
    "Teaching thunder to wait its turn",
    "Sanding down the edges",
    "Convincing the cat to move",
    "Filling the kettle",
    "Finding a comfortable chair",
    "Letting the room settle",
    "Nothing streams, everything waits",
    "Dimming the lights",
    "Checking under the floorboards",
    "Arranging the weather",
    "Aligning the loop points",
    "Making the ocean agree with itself",
    "Brewing something warm",
    "Closing the curtains",
];

thread_local! {
    static WINDOW: std::cell::RefCell<Option<slint::Weak<AppWindow>>> = const { std::cell::RefCell::new(None) };
    static STARTED: Cell<Option<Instant>> = const { Cell::new(None) };
    static BOOTING: Cell<bool> = const { Cell::new(false) };
    static UPDATING: Cell<bool> = const { Cell::new(false) };
    static PROGRESS: Cell<f32> = const { Cell::new(0.0) };
}

fn ui() -> Option<AppWindow> {
    WINDOW.with(|w| w.borrow().as_ref().and_then(|w| w.upgrade()))
}

fn glow() -> Image {
    let side = 128u32;
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(side, side);
    let half = side as f32 / 2.0;
    let corner = (2.0f32).sqrt() * half;
    for (i, px) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = (i as u32 % side) as f32 + 0.5 - half;
        let y = (i as u32 / side) as f32 + 0.5 - half;
        let r = (x * x + y * y).sqrt() / corner;
        let a = if r >= 0.62 { 0.0 } else { 0.16 * (1.0 - r / 0.62) };
        let v = (a * 255.0 + 0.5) as u8;
        *px = Rgba8Pixel { r: v, g: v, b: v, a: v };
    }
    Image::from_rgba8_premultiplied(buffer)
}

fn freshen(ui: &AppWindow) {
    let pick = nuru_core::persist::random_u64();
    let rgb = ACCENTS[(pick % ACCENTS.len() as u64) as usize];
    ui.set_boot_accent(Color::from_rgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
    ui.set_boot_line(LINES[((pick >> 16) % LINES.len() as u64) as usize].into());
    ui.set_boot_glow(glow());
    ui.set_boot_error(Default::default());
    ui.set_boot_shown(true);
    ui.set_boot_visible(true);
}

fn advance(ui: &AppWindow, label: &str, at: f32) {
    let at = PROGRESS.with(|p| {
        let next = p.get().max(at);
        p.set(next);
        next
    });
    ui.set_boot_progress(at);
    ui.set_boot_label(label.into());
}

pub fn begin(ui: &AppWindow) {
    WINDOW.with(|w| *w.borrow_mut() = Some(ui.as_weak()));
    STARTED.with(|s| s.set(Some(Instant::now())));
    BOOTING.with(|b| b.set(true));
    PROGRESS.with(|p| p.set(0.0));
    freshen(ui);
    ui.set_boot_update(false);
    advance(ui, "Reading sound packs", 0.16);
    slint::Timer::single_shot(GUARD, boot_done);
}

pub fn boot_step(label: &str, at: f32) {
    if !BOOTING.with(Cell::get) || UPDATING.with(Cell::get) {
        return;
    }
    if let Some(ui) = ui() {
        advance(&ui, label, at);
    }
}

pub fn boot_done() {
    if !BOOTING.with(|b| b.replace(false)) || UPDATING.with(Cell::get) {
        return;
    }
    let Some(ui) = ui() else { return };
    advance(&ui, "", 1.0);
    let held = STARTED.with(Cell::get).map(|s| s.elapsed()).unwrap_or(MIN_BOOT);
    slint::Timer::single_shot(MIN_BOOT.saturating_sub(held), hide);
}

fn hide() {
    if UPDATING.with(Cell::get) {
        return;
    }
    let Some(window) = ui() else { return };
    window.set_boot_shown(false);
    slint::Timer::single_shot(FADE, || {
        if UPDATING.with(Cell::get) {
            return;
        }
        if let Some(ui) = ui() {
            if !ui.get_boot_shown() {
                ui.set_boot_visible(false);
                ui.set_boot_glow(Image::default());
            }
        }
    });
}

pub fn update_begin() {
    let Some(ui) = ui() else { return };
    BOOTING.with(|b| b.set(false));
    UPDATING.with(|u| u.set(true));
    PROGRESS.with(|p| p.set(0.0));
    freshen(&ui);
    ui.set_boot_update(true);
    ui.set_boot_step(0);
    advance(&ui, "Preparing", 0.0);
}

pub fn update_step(label: &str, at: f32, step: i32) {
    if !UPDATING.with(Cell::get) {
        return;
    }
    if let Some(ui) = ui() {
        ui.set_boot_step(step);
        advance(&ui, label, at);
    }
}

pub fn update_error(message: &str) {
    let Some(ui) = ui() else { return };
    ui.set_boot_error(message.into());
    slint::Timer::single_shot(ERROR_HOLD, || {
        UPDATING.with(|u| u.set(false));
        hide();
    });
}
