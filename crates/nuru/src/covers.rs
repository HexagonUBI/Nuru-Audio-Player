use image::imageops::{self, FilterType};
use image::RgbaImage;
use slint::{Rgba8Pixel, SharedPixelBuffer};

use crate::effects::rounded_coverage;

pub type Pixels = SharedPixelBuffer<Rgba8Pixel>;

const TINT_ALPHA: f32 = 224.0 / 255.0;
const BRIGHTNESS: f32 = 1.02;
const CONTRAST: f32 = 1.12;
const SATURATE: f32 = 1.28;
const SCRIM: [f32; 3] = [4.0 / 255.0, 5.0 / 255.0, 7.0 / 255.0];
const SCRIM_ALPHA: f32 = 0.7;
const SCRIM_REACH: f32 = 0.52;

#[derive(Clone, Copy)]
pub struct Shape {
    pub size: u32,
    pub radius: f32,
}

pub fn resting(path: &str, shape: Shape) -> Option<Pixels> {
    let mut img = square(&decode(path)?, shape.size);
    let h = shape.size as f32;
    for (_, y, px) in img.enumerate_pixels_mut() {
        let from_bottom = (h - (y as f32 + 0.5)) / h;
        if from_bottom >= SCRIM_REACH {
            continue;
        }
        let a = SCRIM_ALPHA * (1.0 - from_bottom / SCRIM_REACH);
        for i in 0..3 {
            let c = px[i] as f32 / 255.0;
            px[i] = to_u8(c * (1.0 - a) + SCRIM[i] * a);
        }
    }
    Some(rounded(img, shape))
}

pub fn active(path: &str, accent: [u8; 3], shape: Shape) -> Option<Pixels> {
    let mut img = square(&decode(path)?, shape.size);
    let a = accent.map(|c| c as f32 / 255.0);
    let s = SATURATE;
    let sat = [
        [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
    ];
    for px in img.pixels_mut() {
        let mut m = [0.0f32; 3];
        for i in 0..3 {
            let c = px[i] as f32 / 255.0;
            let screen = c + a[i] - c * a[i];
            let mixed = c * (1.0 - TINT_ALPHA) + screen * TINT_ALPHA;
            let bright = (mixed * BRIGHTNESS).min(1.0);
            m[i] = ((bright - 0.5) * CONTRAST + 0.5).clamp(0.0, 1.0);
        }
        for (i, row) in sat.iter().enumerate() {
            px[i] = to_u8(row[0] * m[0] + row[1] * m[1] + row[2] * m[2]);
        }
    }
    Some(rounded(img, shape))
}

pub fn accent_rgb(hex: &str) -> [u8; 3] {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 || !h.is_ascii() {
        return [0xff, 0xb4, 0x54];
    }
    let part = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    [part(0), part(2), part(4)]
}

fn decode(path: &str) -> Option<RgbaImage> {
    let decoded = image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(image::ImageError::IoError)
        .and_then(|reader| reader.decode());
    match decoded {
        Ok(img) => Some(img.to_rgba8()),
        Err(e) => {
            log::warn!("cover {path} did not decode: {e}");
            None
        }
    }
}

fn square(img: &RgbaImage, size: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let side = w.min(h);
    let cropped = imageops::crop_imm(img, (w - side) / 2, (h - side) / 2, side, side).to_image();
    imageops::resize(&cropped, size, size, FilterType::CatmullRom)
}

fn rounded(mut img: RgbaImage, shape: Shape) -> Pixels {
    let side = shape.size as f32;
    for (x, y, px) in img.enumerate_pixels_mut() {
        let cover = rounded_coverage(x as f32 + 0.5, y as f32 + 0.5, side, side, shape.radius);
        if cover < 1.0 {
            px[3] = to_u8(px[3] as f32 / 255.0 * cover);
        }
    }
    let (w, h) = img.dimensions();
    SharedPixelBuffer::clone_from_slice(img.as_raw(), w, h)
}

fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
