use std::time::{Duration, Instant};

use nuru_core::scene::Scene;
use resvg::{tiny_skia, usvg};
use slint::{Image, Rgb8Pixel, Rgba8Pixel, SharedPixelBuffer};

use crate::scene_art;

const PIXEL_CAP: f64 = 2_100_000.0;
const FAST: Duration = Duration::from_millis(33);
const SLOW: Duration = Duration::from_millis(100);
const SEEN: u8 = 24;
const FLICKER: [(f32, f32); 5] = [(0.0, 0.9), (0.28, 1.2), (0.54, 0.8), (0.76, 1.1), (1.0, 0.9)];

#[derive(Clone, PartialEq, Debug)]
pub struct Look {
    pub width: u32,
    pub height: u32,
    pub px: f32,
    pub sky: String,
    pub view: String,
    pub wet: u8,
    pub fire: u8,
    pub campfire: bool,
    pub storm: bool,
    pub panes: bool,
}

impl Look {
    pub fn new(scene: &Scene, physical: (u32, u32), logical_width: f32, panes: bool) -> Option<Look> {
        let (pw, ph) = physical;
        if pw < 2 || ph < 2 || logical_width <= 0.0 {
            return None;
        }
        let area = pw as f64 * ph as f64;
        let shrink = if area > PIXEL_CAP { (PIXEL_CAP / area).sqrt() } else { 1.0 };
        let width = ((pw as f64 * shrink).round() as u32).max(2);
        let height = ((ph as f64 * shrink).round() as u32).max(2);
        Some(Look {
            width,
            height,
            px: width as f32 / logical_width,
            sky: scene.sky.clone(),
            view: scene.window.clone(),
            wet: (scene.rain.min(1.0) * 20.0).round() as u8,
            fire: (scene.hearth.min(1.0) * 20.0).round() as u8,
            campfire: scene.hearth_kind.as_deref() == Some("campfire"),
            storm: scene.storm > 0.05,
            panes,
        })
    }

    fn wet(&self) -> f64 {
        self.wet as f64 / 20.0
    }

    fn fire(&self) -> f32 {
        self.fire as f32 / 20.0
    }
}

pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    buf: SharedPixelBuffer<Rgb8Pixel>,
}

impl Tile {
    pub fn image(&self) -> Image {
        Image::from_rgb8(self.buf.clone())
    }
}

pub struct Baked {
    look: Look,
    tiles: Vec<Tile>,
    cols: usize,
    col_of: Vec<u16>,
    row_of: Vec<u16>,
    atten: Vec<u8>,
    seen: (f32, f32, f32, f32),
    band_top: u32,
    band_y: Vec<u8>,
    band_v: Vec<u8>,
    band_a: Vec<u8>,
    flash: Option<SharedPixelBuffer<Rgba8Pixel>>,
}

fn render(svg: &str, width: u32, height: u32) -> Option<tiny_skia::Pixmap> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| log::warn!("cozy scene: {e}")).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    Some(pixmap)
}

fn vignette(x: u32, y: u32, w: u32, h: u32) -> f32 {
    let (w, h) = (w as f32, h as f32);
    let dx = (x as f32 + 0.5 - 0.5 * w) / (1.2 * w);
    let dy = (y as f32 + 0.5 - 0.42 * h) / (0.9 * h);
    let d = (dx * dx + dy * dy).sqrt();
    let a = if d <= 0.4 {
        0.0
    } else if d >= 1.0 {
        0.7
    } else {
        0.7 * (d - 0.4) / 0.6
    };
    1.0 - a
}

struct Glow {
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    alpha: f32,
    stop: f32,
    color: [f32; 3],
}

impl Glow {
    fn of(campfire: bool, w: u32, h: u32) -> Glow {
        let (w, h) = (w as f32, h as f32);
        if campfire {
            Glow { cx: 0.5 * w, cy: 1.12 * h, rx: w, ry: 0.55 * h, alpha: 0.62, stop: 0.58, color: [1.0, 162.0 / 255.0, 61.0 / 255.0] }
        } else {
            Glow { cx: 0.22 * w, cy: 1.08 * h, rx: 1.2 * w, ry: 0.6 * h, alpha: 0.55, stop: 0.62, color: [1.0, 138.0 / 255.0, 61.0 / 255.0] }
        }
    }

    fn top(&self) -> f32 {
        self.cy - self.stop * self.ry
    }

    fn at(&self, x: u32, y: u32) -> f32 {
        let dx = (x as f32 + 0.5 - self.cx) / self.rx;
        let dy = (y as f32 + 0.5 - self.cy) / self.ry;
        let d = (dx * dx + dy * dy).sqrt();
        if d >= self.stop {
            0.0
        } else {
            self.alpha * (1.0 - d / self.stop)
        }
    }
}

fn cuts(mut at: Vec<u32>, end: u32) -> Vec<u32> {
    at.push(0);
    at.push(end);
    at.retain(|&v| v <= end);
    at.sort_unstable();
    at.dedup();
    at
}

fn lookup(cuts: &[u32]) -> Vec<u16> {
    let mut out = Vec::with_capacity(*cuts.last().unwrap_or(&0) as usize);
    for (i, pair) in cuts.windows(2).enumerate() {
        out.extend(std::iter::repeat_n(i as u16, (pair[1] - pair[0]) as usize));
    }
    out
}

pub fn bake(look: Look) -> Option<Baked> {
    let started = Instant::now();
    let (w, h) = (look.width, look.height);
    let n = (w * h) as usize;

    let top = render(&scene_art::top_svg(w, h, look.px as f64, look.wet()), w, h)?;
    let bottom = render(&scene_art::bottom_svg(w, h, &look.sky, &look.view, look.wet()), w, h)?;
    let (t, b) = (top.data(), bottom.data());

    let mut atten = vec![0u8; n];
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let inv = 255 - t[i * 4 + 3] as u32;
            let a = (inv as f32 * vignette(x, y, w, h) + 0.5) as u8;
            atten[i] = a;
            if a >= SEEN {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }

    let fire = look.fire();
    let glow = Glow::of(look.campfire, w, h);
    let band_top = if fire > 0.0 { (glow.top().floor().max(0.0) as u32).min(h) } else { h };
    let band_n = ((h - band_top) * w) as usize;

    let (xs, ys) = if look.panes && x1 > x0 && y1 > y0 {
        (cuts(vec![x0, x1], w), cuts(vec![y0, band_top, y1], h))
    } else {
        (cuts(Vec::new(), w), cuts(vec![band_top], h))
    };
    let cols = xs.len() - 1;
    let col_of = lookup(&xs);
    let row_of = lookup(&ys);
    let mut tiles = Vec::new();
    for r in ys.windows(2) {
        for c in xs.windows(2) {
            tiles.push(Tile { x: c[0], y: r[0], w: c[1] - c[0], h: r[1] - r[0], buf: SharedPixelBuffer::new(c[1] - c[0], r[1] - r[0]) });
        }
    }

    let mut band_y = vec![0u8; band_n * 3];
    let mut band_v = vec![0u8; band_n];
    let mut band_a = vec![0u8; band_n];
    let split = (band_top * w) as usize;

    for tile in tiles.iter_mut() {
        let out = tile.buf.make_mut_bytes();
        for ty in 0..tile.h {
            let y = tile.y + ty;
            for tx in 0..tile.w {
                let x = tile.x + tx;
                let i = (y * w + x) as usize;
                let inv = 255 - t[i * 4 + 3] as u32;
                let v = vignette(x, y, w, h);
                let k = ((ty * tile.w + tx) * 3) as usize;
                if y < band_top {
                    for c in 0..3 {
                        let yc = ((b[i * 4 + c] as u32 * inv + 127) / 255 + t[i * 4 + c] as u32).min(255);
                        out[k + c] = (yc as f32 * v + 0.5) as u8;
                    }
                } else {
                    let j = i - split;
                    for c in 0..3 {
                        band_y[j * 3 + c] = ((b[i * 4 + c] as u32 * inv + 127) / 255 + t[i * 4 + c] as u32).min(255) as u8;
                    }
                    band_v[j] = (v * 255.0 + 0.5) as u8;
                    band_a[j] = (glow.at(x, y) * fire * v * 255.0 + 0.5) as u8;
                }
            }
        }
    }

    let mut flash = if look.storm { Some(SharedPixelBuffer::<Rgba8Pixel>::new(w / 2, h / 2)) } else { None };
    if let Some(flash) = flash.as_mut() {
        let (fw, fh) = (w / 2, h / 2);
        let px = flash.make_mut_slice();
        for y in 0..fh {
            for x in 0..fw {
                let i = ((y * 2) * w + x * 2) as usize;
                let m = atten[i] as f32 / 255.0;
                let luma = (b[i * 4] as f32 * 0.3 + b[i * 4 + 1] as f32 * 0.59 + b[i * 4 + 2] as f32 * 0.11) / 255.0;
                let a = 0.9 * m * (1.0 - luma);
                px[(y * fw + x) as usize] = Rgba8Pixel {
                    r: (215.0 * a + 0.5) as u8,
                    g: (228.0 * a + 0.5) as u8,
                    b: (255.0 * a + 0.5) as u8,
                    a: (255.0 * a + 0.5) as u8,
                };
            }
        }
    }

    log::info!("cozy scene {}x{} in {} tiles baked in {} ms", w, h, tiles.len(), started.elapsed().as_millis());
    let seen = (x0 as f32, y0 as f32, x1 as f32 - 1.0, y1 as f32 - 1.0);
    Some(Baked { look, tiles, cols, col_of, row_of, atten, seen, band_top, band_y, band_v, band_a, flash })
}

#[derive(Clone, Copy)]
struct Particle {
    snow: bool,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    len: f32,
    a: f32,
}

impl Particle {
    fn bounds(&self) -> (f32, f32, f32, f32) {
        if self.snow {
            let r = self.len + 1.0;
            (self.x - r, self.y - r, self.x + r, self.y + r)
        } else {
            let xb = self.x - self.vx * 1.6;
            let yb = self.y - self.len;
            (self.x.min(xb) - 2.0, yb.min(self.y) - 1.0, self.x.max(xb) + 2.0, yb.max(self.y) + 1.0)
        }
    }
}

struct Rand(u64);

impl Rand {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Weather {
    rain: f32,
    snow: f32,
    wind: f32,
    storm: f32,
    hearth: f32,
}

pub struct Cozy {
    shown: Option<Baked>,
    weather: Weather,
    particles: Vec<Particle>,
    undo: Vec<(u16, u32, [u8; 3])>,
    touched: Vec<bool>,
    rand: Rand,
    started: Instant,
    last: Instant,
    level: f32,
    flash: f32,
    reduced: bool,
    timer: slint::Timer,
    pace: Option<Duration>,
}

impl Cozy {
    pub fn new(scene: &Scene) -> Cozy {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15)
            | 1;
        let now = Instant::now();
        Cozy {
            shown: None,
            weather: weather_of(scene),
            particles: Vec::new(),
            undo: Vec::new(),
            touched: Vec::new(),
            rand: Rand(seed),
            started: now,
            last: now,
            level: 1.0,
            flash: 0.0,
            reduced: reduced_motion(),
            timer: slint::Timer::default(),
            pace: None,
        }
    }

    pub fn timer(&self) -> &slint::Timer {
        &self.timer
    }

    pub fn look(&self) -> Option<&Look> {
        self.shown.as_ref().map(|b| &b.look)
    }

    pub fn tiles(&self) -> &[Tile] {
        self.shown.as_ref().map(|b| b.tiles.as_slice()).unwrap_or(&[])
    }

    pub fn px(&self) -> f32 {
        self.shown.as_ref().map(|b| b.look.px).unwrap_or(1.0)
    }

    pub fn set_scene(&mut self, scene: &Scene) {
        let next = weather_of(scene);
        let reseed = next.rain != self.weather.rain || next.snow != self.weather.snow || next.wind != self.weather.wind;
        self.weather = next;
        if reseed {
            self.seed();
        }
    }

    pub fn install(&mut self, baked: Baked) {
        self.undo.clear();
        self.touched = vec![false; baked.tiles.len()];
        self.shown = Some(baked);
        self.level = 0.0;
        self.seed();
        self.step(Instant::now(), &mut |_| {});
    }

    pub fn flash_image(&self) -> Image {
        self.shown
            .as_ref()
            .and_then(|b| b.flash.clone())
            .map(Image::from_rgba8_premultiplied)
            .unwrap_or_default()
    }

    pub fn flash_level(&self) -> f32 {
        self.flash
    }

    pub fn pace(&self) -> Option<Duration> {
        if self.shown.is_none() || self.reduced {
            return None;
        }
        if !self.particles.is_empty() {
            Some(FAST)
        } else if self.weather.hearth > 0.02 || self.weather.storm > 0.05 {
            Some(SLOW)
        } else {
            None
        }
    }

    pub fn paced(&mut self) -> Option<Option<Duration>> {
        let want = self.pace();
        if want == self.pace {
            None
        } else {
            self.pace = want;
            Some(want)
        }
    }

    pub fn tick(&mut self, now: Instant, release: &mut dyn FnMut(&[bool])) -> bool {
        if self.shown.is_none() {
            return false;
        }
        let before_flash = self.flash;
        self.step(now, release);
        self.flash != before_flash
    }

    fn step(&mut self, now: Instant, release: &mut dyn FnMut(&[bool])) {
        let dt = ((now - self.last).as_secs_f32() * 60.0).clamp(0.0, 4.0);
        self.last = now;
        let t = (now - self.started).as_secs_f32();

        self.flash = if self.weather.storm > 0.05 && !self.reduced {
            let rate = (12.0 - self.weather.storm * 8.0).max(3.0);
            let p = (t % rate) / rate;
            if (0.89..0.90).contains(&p) {
                0.5
            } else if (0.915..0.93).contains(&p) {
                0.32
            } else {
                0.0
            }
        } else {
            0.0
        };

        let level = if self.reduced { 1.0 } else { flicker(t) };
        let Some(b) = self.shown.as_ref() else { return };
        let hearth = b.band_top < b.look.height;
        let relight = hearth && (level - self.level).abs() > 0.004;
        if self.particles.is_empty() && self.undo.is_empty() && !relight {
            return;
        }

        if !self.particles.is_empty() {
            self.advance(dt);
        }
        self.mark(relight);
        release(&self.touched);
        self.restore();
        if relight {
            self.level = level;
            self.relight();
        }
        self.draw();
    }

    fn mark(&mut self, relight: bool) {
        let Some(b) = self.shown.as_ref() else { return };
        self.touched.iter_mut().for_each(|t| *t = false);
        for (tile, _, _) in &self.undo {
            self.touched[*tile as usize] = true;
        }
        if relight {
            for (i, tile) in b.tiles.iter().enumerate() {
                if tile.y + tile.h > b.band_top {
                    self.touched[i] = true;
                }
            }
        }
        let (sx0, sy0, sx1, sy1) = b.seen;
        for p in &self.particles {
            let (l, t, r, d) = p.bounds();
            let (l, t, r, d) = (l.max(sx0), t.max(sy0), r.min(sx1), d.min(sy1));
            if l > r || t > d {
                continue;
            }
            let (l, t, r, d) = (l as u32, t as u32, r as u32, d as u32);
            let (c0, c1) = (b.col_of[l as usize] as usize, b.col_of[r as usize] as usize);
            let (r0, r1) = (b.row_of[t as usize] as usize, b.row_of[d as usize] as usize);
            for row in r0..=r1 {
                for col in c0..=c1 {
                    self.touched[row * b.cols + col] = true;
                }
            }
        }
    }

    fn relight(&mut self) {
        let Some(b) = self.shown.as_mut() else { return };
        let glow = Glow::of(b.look.campfire, b.look.width, b.look.height);
        let tint = glow.color.map(|c| ((c * self.level).min(1.0) * 255.0 + 0.5) as u32);
        let (w, band_top) = (b.look.width, b.band_top);
        for tile in b.tiles.iter_mut() {
            if tile.y + tile.h <= band_top {
                continue;
            }
            let out = tile.buf.make_mut_bytes();
            for ty in tile.y.max(band_top) - tile.y..tile.h {
                let y = tile.y + ty;
                for tx in 0..tile.w {
                    let j = ((y - band_top) * w + tile.x + tx) as usize;
                    let (v, a) = (b.band_v[j] as u32, b.band_a[j] as u32);
                    let k = ((ty * tile.w + tx) * 3) as usize;
                    for c in 0..3 {
                        let yc = b.band_y[j * 3 + c] as u32;
                        let lit = (a * tint[c] + 127) / 255;
                        out[k + c] = ((yc * v + lit * (255 - yc) + 127) / 255).min(255) as u8;
                    }
                }
            }
        }
    }

    fn seed(&mut self) {
        self.particles.clear();
        let Some(b) = self.shown.as_ref() else { return };
        if self.reduced {
            return;
        }
        let (w, h, dpr) = (b.look.width as f32, b.look.height as f32, b.look.px);
        let rain = (self.weather.rain.min(1.6) * 260.0).round() as usize;
        let snow = (self.weather.snow.min(1.6) * 150.0).round() as usize;
        let wind = self.weather.wind;
        for i in 0..rain + snow {
            let p = spawn(&mut self.rand, i >= rain, true, w, h, wind, dpr);
            self.particles.push(p);
        }
    }

    fn advance(&mut self, dt: f32) {
        let Some(b) = self.shown.as_ref() else { return };
        let (w, h, dpr) = (b.look.width as f32, b.look.height as f32, b.look.px);
        let wind = self.weather.wind;
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.y > h + 20.0 * dpr || p.x < -60.0 * dpr || p.x > w + 60.0 * dpr {
                let snow = p.snow;
                self.particles[i] = spawn(&mut self.rand, snow, false, w, h, wind, dpr);
            }
        }
    }

    fn restore(&mut self) {
        let Some(b) = self.shown.as_mut() else {
            self.undo.clear();
            return;
        };
        if self.undo.is_empty() {
            return;
        }
        let mut slices: Vec<Option<&mut [u8]>> =
            b.tiles.iter_mut().zip(&self.touched).map(|(t, &on)| on.then(|| t.buf.make_mut_bytes())).collect();
        for (tile, k, rgb) in self.undo.drain(..).rev() {
            if let Some(out) = slices[tile as usize].as_deref_mut() {
                let k = k as usize;
                out[k..k + 3].copy_from_slice(&rgb);
            }
        }
    }

    fn draw(&mut self) {
        if self.particles.is_empty() {
            return;
        }
        let Some(b) = self.shown.as_mut() else { return };
        let (w, h) = (b.look.width as i32, b.look.height as i32);
        let dpr = b.look.px;
        let (atten, col_of, row_of, cols) = (&b.atten, &b.col_of, &b.row_of, b.cols);
        let mut slices: Vec<Option<(u32, u32, u32, &mut [u8])>> = b
            .tiles
            .iter_mut()
            .zip(&self.touched)
            .map(|(t, &on)| on.then(|| (t.x, t.y, t.w, t.buf.make_mut_bytes())))
            .collect();
        let undo = &mut self.undo;
        let mut put = |x: i32, y: i32, alpha: f32, color: [f32; 3]| {
            if x < 0 || y < 0 || x >= w || y >= h {
                return;
            }
            let i = (y * w + x) as usize;
            let m = atten[i];
            if m < SEEN {
                return;
            }
            let f = alpha * m as f32 / 255.0;
            let tile = row_of[y as usize] as usize * cols + col_of[x as usize] as usize;
            let Some((tx, ty, tw, out)) = slices[tile].as_mut() else { return };
            let k = (((y as u32 - *ty) * *tw + (x as u32 - *tx)) * 3) as usize;
            undo.push((tile as u16, k as u32, [out[k], out[k + 1], out[k + 2]]));
            for c in 0..3 {
                let cur = out[k + c] as f32;
                out[k + c] = (cur + f * (color[c] - cur) + 0.5).clamp(0.0, 255.0) as u8;
            }
        };

        const RAIN: [f32; 3] = [198.0, 220.0, 255.0];
        const SNOW: [f32; 3] = [233.0, 240.0, 255.0];
        let half = 0.55 * dpr;
        for p in &self.particles {
            if p.snow {
                let r = p.len;
                let (x0, x1) = ((p.x - r - 1.0).floor() as i32, (p.x + r + 1.0).ceil() as i32);
                let (y0, y1) = ((p.y - r - 1.0).floor() as i32, (p.y + r + 1.0).ceil() as i32);
                for py in y0..=y1 {
                    for px in x0..=x1 {
                        let dx = px as f32 + 0.5 - p.x;
                        let dy = py as f32 + 0.5 - p.y;
                        let cover = (r + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
                        if cover > 0.0 {
                            put(px, py, p.a * cover, SNOW);
                        }
                    }
                }
            } else {
                let (xa, ya) = (p.x, p.y);
                let (xb, yb) = (p.x - p.vx * 1.6, p.y - p.len);
                let (top, bot) = (yb.min(ya), yb.max(ya));
                for py in top.floor() as i32..bot.ceil() as i32 {
                    let yc = py as f32 + 0.5;
                    let span = (yc + 0.5).min(bot) - (yc - 0.5).max(top);
                    if span <= 0.0 {
                        continue;
                    }
                    let t = ((yc - yb) / (ya - yb)).clamp(0.0, 1.0);
                    let xc = xb + (xa - xb) * t;
                    let (l, r) = (xc - half, xc + half);
                    for px in l.floor() as i32..r.ceil() as i32 {
                        let cover = ((px as f32 + 1.0).min(r) - (px as f32).max(l)).clamp(0.0, 1.0) * span.min(1.0);
                        if cover > 0.0 {
                            put(px, py, p.a * cover, RAIN);
                        }
                    }
                }
            }
        }
    }
}

fn weather_of(scene: &Scene) -> Weather {
    Weather { rain: scene.rain, snow: scene.snow, wind: scene.wind, storm: scene.storm, hearth: scene.hearth }
}

fn spawn(rand: &mut Rand, snow: bool, anywhere: bool, w: f32, h: f32, wind: f32, dpr: f32) -> Particle {
    let drive = 1.0 + wind * 1.4;
    let x = rand.next() * w * 1.3 - w * 0.15;
    let y = if anywhere { rand.next() * h } else { -20.0 * dpr };
    let vy = if snow { (0.35 + rand.next() * 0.5) * (1.0 + wind * 0.5) } else { (7.0 + rand.next() * 6.0) * drive } * dpr;
    let vx = if snow { (rand.next() - 0.5) * 0.6 - wind * 2.2 } else { -0.9 - rand.next() * 0.5 - wind * 4.5 } * dpr;
    let len = if snow { 1.6 + rand.next() * 1.8 } else { (9.0 + rand.next() * 13.0) * (1.0 + wind * 0.5) } * dpr;
    let a = if snow { 0.35 + rand.next() * 0.45 } else { 0.1 + rand.next() * 0.22 };
    Particle { snow, x, y, vx, vy, len, a }
}

fn ease_in_out(x: f32) -> f32 {
    let (x1, y1, x2, y2) = (0.65f32, 0.0f32, 0.35f32, 1.0f32);
    let bez = |t: f32, a: f32, b: f32| 3.0 * (1.0 - t) * (1.0 - t) * t * a + 3.0 * (1.0 - t) * t * t * b + t * t * t;
    let mut lo = 0.0;
    let mut hi = 1.0;
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if bez(mid, x1, x2) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bez((lo + hi) / 2.0, y1, y2)
}

fn flicker(t: f32) -> f32 {
    let p = (t % 5.5) / 5.5;
    for pair in FLICKER.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if p >= a.0 && p <= b.0 {
            let f = ease_in_out((p - a.0) / (b.0 - a.0));
            return a.1 + (b.1 - a.1) * f;
        }
    }
    0.9
}

#[cfg(windows)]
fn reduced_motion() -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, value: *mut core::ffi::c_void, flags: u32) -> i32;
    }
    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    let mut on: i32 = 1;
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut on as *mut i32 as *mut _, 0) };
    ok != 0 && on == 0
}

#[cfg(not(windows))]
fn reduced_motion() -> bool {
    false
}
