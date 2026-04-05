#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod enumerate;
mod leftovers;
mod logo_bitmap;
mod logo_asset;
mod models;
mod ms_store;
mod steam;
mod uninstall;
mod win_lnk;

fn window_icon() -> eframe::egui::IconData {
    logo_asset::window_icon_data()
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([600.0, 460.0])
            .with_min_inner_size([440.0, 340.0])
            .with_title("Sweep Uninstall")
            .with_icon(window_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Sweep Uninstall",
        options,
        Box::new(|cc| Ok(Box::new(app::SweepApp::new(cc)))),
    )
}
