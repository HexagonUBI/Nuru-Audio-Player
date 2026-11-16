use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

pub struct Sprite {
    pub image: Image,
    pub pad: f32,
}

pub fn shadow(radius: f32, blur: f32, scale: f32) -> Sprite {
    let r = (radius * scale).ceil().max(0.0);
    let pad = pad_for(blur, scale);
    let inner = 2.0 * (r + pad) + 2.0;
    let mut image = render(inner, inner, r, blur, scale);
    let edge = (2.0 * pad + r) as u16;
    image.set_nine_slice_edges(edge, edge, edge, edge);
    Sprite { image, pad: pad / scale }
}

pub fn shadow_sized(width: f32, height: f32, radius: f32, blur: f32, scale: f32) -> Sprite {
    let r = (radius * scale).max(0.0);
    let pad = pad_for(blur, scale);
    let image = render((width * scale).round(), (height * scale).round(), r, blur, scale);
    Sprite { image, pad: pad / scale }
}

pub fn shadow_wide(height: f32, radius: f32, blur: f32, scale: f32) -> Sprite {
    let r = (radius * scale).max(0.0);
    let pad = pad_for(blur, scale);
    let width = 2.0 * (r.ceil() + pad) + 2.0;
    let mut image = render(width, (height * scale).round(), r, blur, scale);
    let edge = (2.0 * pad + r.ceil()) as u16;
    image.set_nine_slice_edges(0, edge, 0, edge);
    Sprite { image, pad: pad / scale }
}

pub fn mask(radius: f32, scale: f32) -> Image {
    let r = radius * scale;
    let edge = r.ceil() as usize + 1;
    let size = 2 * edge + 2;
    let side = size as f32;
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(size as u32, size as u32);
    for (i, px) in buffer.make_mut_slice().iter_mut().enumerate() {
        let x = (i % size) as f32 + 0.5;
        let y = (i / size) as f32 + 0.5;
        let a = rounded_coverage(x, y, side, side, r);
        *px = Rgba8Pixel { r: 255, g: 255, b: 255, a: (a * 255.0).round() as u8 };
    }
    let mut image = Image::from_rgba8(buffer);
    let e = edge as u16;
    image.set_nine_slice_edges(e, e, e, e);
    image
}

fn pad_for(blur: f32, scale: f32) -> f32 {
    (1.5 * blur * scale).max(1.5).ceil()
}

fn render(inner_w: f32, inner_h: f32, r: f32, blur: f32, scale: f32) -> Image {
    let sigma = (blur * scale / 2.0).max(0.5);
    let pad = pad_for(blur, scale);
    let w = (inner_w + 2.0 * pad) as usize;
    let h = (inner_h + 2.0 * pad) as usize;

    let mut alpha = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let px = x as f32 + 0.5 - pad;
            let py = y as f32 + 0.5 - pad;
            alpha[y * w + x] = rounded_coverage(px, py, inner_w, inner_h, r.min(inner_w / 2.0).min(inner_h / 2.0));
        }
    }
    gaussian(&mut alpha, w, h, sigma);

    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
    for (px, a) in buffer.make_mut_slice().iter_mut().zip(&alpha) {
        *px = Rgba8Pixel { r: 255, g: 255, b: 255, a: (a.clamp(0.0, 1.0) * 255.0).round() as u8 };
    }
    Image::from_rgba8(buffer)
}

pub fn rounded_coverage(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (px - w / 2.0).abs() - (w / 2.0 - r);
    let qy = (py - h / 2.0).abs() - (h / 2.0 - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    (0.5 - (outside + inside - r)).clamp(0.0, 1.0)
}

fn gaussian(data: &mut [f32], w: usize, h: usize, sigma: f32) {
    let mut scratch = vec![0f32; data.len()];
    for radius in box_radii(sigma) {
        box_pass(data, &mut scratch, w, h, radius, true);
        box_pass(&scratch, data, w, h, radius, false);
    }
}

fn box_radii(sigma: f32) -> [usize; 3] {
    let n = 3.0;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let wl_f = wl as f32;
    let m = ((12.0 * sigma * sigma - n * wl_f * wl_f - 4.0 * n * wl_f - 3.0 * n) / (-4.0 * wl_f - 4.0)).round() as i32;
    let mut out = [0usize; 3];
    for (i, slot) in out.iter_mut().enumerate() {
        let size = if (i as i32) < m { wl } else { wu };
        *slot = ((size.max(1) - 1) / 2) as usize;
    }
    out
}

fn box_pass(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize, horizontal: bool) {
    let (lines, len, stride, step) = if horizontal { (h, w, w, 1) } else { (w, h, 1, w) };
    let norm = 1.0 / (2 * r + 1) as f32;
    for line in 0..lines {
        let base = line * stride;
        let at = |i: isize| -> f32 {
            if i < 0 || i as usize >= len {
                0.0
            } else {
                src[base + i as usize * step]
            }
        };
        let mut sum = 0.0;
        for i in -(r as isize)..=(r as isize) {
            sum += at(i);
        }
        for i in 0..len {
            dst[base + i * step] = sum * norm;
            sum += at(i as isize + r as isize + 1) - at(i as isize - r as isize);
        }
    }
}
