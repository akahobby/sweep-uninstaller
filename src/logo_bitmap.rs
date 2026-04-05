//! Logo decode shared by `logo_asset` (runtime) and `build.rs` (embedded .ico).

use image::{ImageBuffer, Rgba, RgbaImage};

/// Turn near-white and very light gray (typical flat export backgrounds) fully transparent.
fn key_out_light_backdrop(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, mut a] = p.0;
        if a == 0 {
            continue;
        }
        let min_rgb = r.min(g).min(b);
        let max_rgb = r.max(g).max(b);
        let sum = r as u16 + g as u16 + b as u16;
        if min_rgb >= 248 || (max_rgb >= 245 && sum >= 740) {
            *p = Rgba([0, 0, 0, 0]);
            continue;
        }
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

fn key_out_neutral_gray_matte(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let [r, g, b, a] = p.0;
        if a == 0 {
            continue;
        }
        let min_rgb = r.min(g).min(b);
        let max_rgb = r.max(g).max(b);
        let spread = max_rgb.saturating_sub(min_rgb);
        if spread > 8 {
            continue;
        }
        let avg = (r as u16 + g as u16 + b as u16) / 3;
        if (50..=220).contains(&avg) {
            *p = Rgba([0, 0, 0, 0]);
        }
    }
}

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

/// Same pixels as the in-app / taskbar texture (keyed backdrop + crop).
pub fn decode_logo_rgba(png_bytes: &[u8]) -> RgbaImage {
    let mut img = image::load_from_memory(png_bytes)
        .expect("logo png")
        .into_rgba8();
    key_out_light_backdrop(&mut img);
    key_out_neutral_gray_matte(&mut img);
    crop_to_visible_rgba(img)
}

/// Opaque app background (matches `window_icon_data` and egui theme).
pub const ICON_BG: Rgba<u8> = Rgba([6u8, 5u8, 10u8, 255u8]);

/// Flatten transparency onto the dark chrome so shell icons never show a white plate.
/// Only called from `build.rs` (embedded .ico); unused in normal `cargo check` of the binary.
#[allow(dead_code)]
pub fn composite_on_icon_bg(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = ImageBuffer::from_fn(w, h, |_, _| ICON_BG);
    image::imageops::overlay(&mut out, img, 0, 0);
    out
}
