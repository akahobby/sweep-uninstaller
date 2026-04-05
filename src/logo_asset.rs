//! Shared logo decode: remove light/white backdrops so the mark sits on transparent pixels.

use eframe::egui::IconData;
use image::{ImageBuffer, Rgba, RgbaImage};

const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

/// Turn near-white and very light gray (typical flat export backgrounds) fully transparent.
fn key_out_light_backdrop(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, mut a] = p.0;
        if a == 0 {
            continue;
        }
        // Solid white / off-white blocks from generators and title-bar padding.
        let min_rgb = r.min(g).min(b);
        let max_rgb = r.max(g).max(b);
        let sum = r as u16 + g as u16 + b as u16;
        if min_rgb >= 248 || (max_rgb >= 245 && sum >= 740) {
            *p = Rgba([0, 0, 0, 0]);
            continue;
        }
        // Soft fringe: lighten anti-aliased white halos without eating purple/black.
        if sum >= 680 && min_rgb >= 200 && max_rgb >= 235 {
            let bleed = (sum.saturating_sub(650)) as f32 / 120.0;
            let factor = (1.0 - bleed.min(1.0)).max(0.0);
            a = ((a as f32) * factor).round() as u8;
            if a < 8 {
                *p = Rgba([0, 0, 0, 0]);
            } else {
                *p = Rgba([r, g, b, a]);
            }
        }
    }
}

/// Remove opaque neutral-gray “matte” blocks (common bad exports) while keeping purple (high chroma).
fn key_out_neutral_gray_matte(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        if a == 0 {
            continue;
        }
        let min_rgb = r.min(g).min(b);
        let max_rgb = r.max(g).max(b);
        let spread = max_rgb.saturating_sub(min_rgb);
        // True neutral mattes are almost achromatic; purple (even muted) keeps channel spread.
        if spread > 8 {
            continue;
        }
        let avg = (r as u16 + g as u16 + b as u16) / 3;
        if (50..=220).contains(&avg) {
            *p = Rgba([0, 0, 0, 0]);
        }
    }
}

/// Tight bounding box around visible pixels so the header doesn’t reserve a large gray letterbox.
fn crop_to_visible_rgba(img: RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 12 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x > max_x {
        return img;
    }
    let pad = 1u32;
    let min_x = min_x.saturating_sub(pad);
    let min_y = min_y.saturating_sub(pad);
    let max_x = (max_x + pad).min(w - 1);
    let max_y = (max_y + pad).min(h - 1);
    let cw = max_x - min_x + 1;
    let ch = max_y - min_y + 1;
    let mut out = RgbaImage::new(cw, ch);
    for y in 0..ch {
        for x in 0..cw {
            *out.get_pixel_mut(x, y) = *img.get_pixel(min_x + x, min_y + y);
        }
    }
    out
}

pub fn decoded_logo() -> RgbaImage {
    let mut img = image::load_from_memory(LOGO_BYTES)
        .expect("assets/logo.png must be valid")
        .into_rgba8();
    key_out_light_backdrop(&mut img);
    key_out_neutral_gray_matte(&mut img);
    crop_to_visible_rgba(img)
}

/// Square 256×256 RGBA for the native window / taskbar icon.
///
/// The cropped mark is often much smaller than 256px; we **upscale** it to ~248px so shells
/// don’t shrink a tiny glyph into a pin-sized blob. The base is opaque app chrome so
/// transparent margins don’t read as a hollow grey tile on the taskbar.
pub fn window_icon_data() -> IconData {
    const CANVAS: u32 = 256;
    const TARGET_MAX: f32 = 248.0;

    let img = decoded_logo();
    let bg = Rgba([6u8, 5u8, 10u8, 255u8]);
    let mut canvas: RgbaImage =
        ImageBuffer::from_fn(CANVAS, CANVAS, |_, _| bg);

    let (iw, ih) = (img.width(), img.height());
    let max_side = iw.max(ih).max(1) as f32;
    // No `.min(1.0)` — small cropped assets must scale up to fill the bitmap.
    let scale = TARGET_MAX / max_side;
    let nw = ((iw as f32 * scale).round() as u32).clamp(1, CANVAS);
    let nh = ((ih as f32 * scale).round() as u32).clamp(1, CANVAS);
    let resized = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Lanczos3);
    let ox = (CANVAS - nw) / 2;
    let oy = (CANVAS - nh) / 2;
    image::imageops::overlay(&mut canvas, &resized, ox as i64, oy as i64);
    IconData {
        rgba: canvas.into_raw(),
        width: CANVAS,
        height: CANVAS,
    }
}
