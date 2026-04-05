//! Shared logo decode: remove light/white backdrops so the mark sits on transparent pixels.

use crate::logo_bitmap;
use eframe::egui::IconData;
use image::{ImageBuffer, RgbaImage};

const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

pub fn decoded_logo() -> RgbaImage {
    logo_bitmap::decode_logo_rgba(LOGO_BYTES)
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
    let bg = logo_bitmap::ICON_BG;
    let mut canvas: RgbaImage =
        ImageBuffer::from_fn(CANVAS, CANVAS, |_, _| bg);

    let (iw, ih) = (img.width(), img.height());
    let max_side = iw.max(ih).max(1) as f32;
    let scale = TARGET_MAX / max_side;
    let nw = ((iw as f32 * scale).round() as u32).clamp(1, CANVAS);
    let nh = ((ih as f32 * scale).round() as u32).clamp(1, CANVAS);
    let resized = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    let ox = (CANVAS - nw) / 2;
    let oy = (CANVAS - nh) / 2;
    image::imageops::overlay(&mut canvas, &resized, ox as i64, oy as i64);
    IconData {
        rgba: canvas.into_raw(),
        width: CANVAS,
        height: CANVAS,
    }
}
