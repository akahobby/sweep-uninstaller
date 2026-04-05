use crate::enumerate;
use crate::leftovers::{delete_leftover, scan_leftovers};
use crate::models::{AppPhase, InstallSource, InstalledEntry, LeftoverItem, LeftoverKind};
use crate::ms_store;
use crate::steam;
use crate::uninstall;
use crate::win_lnk;
use eframe::egui;
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};

// —— Premium black / violet palette ——
const C_BG: egui::Color32 = egui::Color32::from_rgb(6, 5, 10);
const C_PANEL: egui::Color32 = egui::Color32::from_rgb(12, 10, 18);
const C_SURFACE: egui::Color32 = egui::Color32::from_rgb(18, 14, 28);
const C_RAISED: egui::Color32 = egui::Color32::from_rgb(24, 18, 38);
const C_STROKE: egui::Color32 = egui::Color32::from_rgb(110, 70, 175);
const C_STROKE_SOFT: egui::Color32 = egui::Color32::from_rgb(42, 32, 68);
const C_TEXT: egui::Color32 = egui::Color32::from_rgb(248, 244, 255);
const C_MUTED: egui::Color32 = egui::Color32::from_rgb(132, 118, 168);
const C_ACCENT: egui::Color32 = egui::Color32::from_rgb(145, 82, 235);
const C_ACCENT_HOVER: egui::Color32 = egui::Color32::from_rgb(175, 120, 255);
const C_ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(48, 36, 78);
const C_DANGER: egui::Color32 = egui::Color32::from_rgb(200, 75, 130);
const C_STEAM: egui::Color32 = egui::Color32::from_rgb(100, 175, 230);
const C_STEAM_BG: egui::Color32 = egui::Color32::from_rgb(28, 42, 58);
const C_REG: egui::Color32 = egui::Color32::from_rgb(175, 145, 235);
const C_REG_BG: egui::Color32 = egui::Color32::from_rgb(38, 30, 58);
const C_STORE: egui::Color32 = egui::Color32::from_rgb(0, 160, 245);
const C_STORE_BG: egui::Color32 = egui::Color32::from_rgb(22, 48, 72);

fn entry_can_remove(e: &InstalledEntry) -> bool {
    e.package_full_name.is_some()
        || e
            .uninstall_string
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
        || e
            .quiet_uninstall_string
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
}

fn apply_purple_dark_theme(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(5.0, 3.0);
    style.spacing.window_margin = egui::Margin::same(6.0);
    style.spacing.button_padding = egui::vec2(12.0, 5.0);
    style.spacing.interact_size.y = 22.0;

    use egui::{FontFamily, FontId, TextStyle::*};
    style
        .text_styles
        .insert(Body, FontId::new(11.5, FontFamily::Proportional));
    style
        .text_styles
        .insert(Button, FontId::new(11.5, FontFamily::Proportional));
    style
        .text_styles
        .insert(Heading, FontId::new(14.0, FontFamily::Proportional));
    style
        .text_styles
        .insert(Small, FontId::new(9.5, FontFamily::Proportional));
    style
        .text_styles
        .insert(Monospace, FontId::new(10.0, FontFamily::Monospace));

    let v = &mut style.visuals;
    v.dark_mode = true;
    v.window_fill = C_BG;
    v.panel_fill = C_PANEL;
    v.extreme_bg_color = C_BG;
    v.faint_bg_color = C_SURFACE;
    v.hyperlink_color = C_ACCENT_HOVER;
    v.warn_fg_color = egui::Color32::from_rgb(255, 190, 120);
    v.error_fg_color = egui::Color32::from_rgb(255, 120, 140);

    v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(120, 70, 200, 90);
    v.selection.stroke = egui::Stroke::new(1.0, C_ACCENT);

    v.widgets.noninteractive.fg_stroke.color = C_MUTED;
    v.widgets.noninteractive.bg_fill = C_PANEL;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.expansion = 0.0;

    v.widgets.inactive.fg_stroke.color = C_TEXT;
    v.widgets.inactive.bg_fill = C_RAISED;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, C_STROKE_SOFT);
    v.widgets.inactive.expansion = 0.0;

    v.widgets.hovered.fg_stroke.color = C_TEXT;
    v.widgets.hovered.bg_fill = C_ACCENT_DIM;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, C_STROKE);
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.fg_stroke.color = C_TEXT;
    v.widgets.active.bg_fill = C_ACCENT;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, C_ACCENT_HOVER);
    v.widgets.active.expansion = 0.0;

    v.widgets.open.fg_stroke.color = C_TEXT;
    v.widgets.open.bg_fill = C_ACCENT_DIM;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, C_STROKE);

    v.window_stroke = egui::Stroke::new(1.0, C_STROKE_SOFT);
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::default();
}

fn btn_primary(ui: &mut egui::Ui, enabled: bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    let btn = egui::Button::new(text)
        .fill(if enabled { C_ACCENT } else { C_ACCENT_DIM })
        .stroke(egui::Stroke::new(
            1.0,
            if enabled {
                C_ACCENT_HOVER
            } else {
                C_STROKE_SOFT
            },
        ))
        .min_size(egui::vec2(0.0, 24.0))
        .rounding(egui::Rounding::same(5.0));
    ui.add_enabled(enabled, btn)
}

fn btn_secondary(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    ui.add(
        egui::Button::new(text)
            .fill(C_SURFACE)
            .stroke(egui::Stroke::new(1.0, C_STROKE_SOFT))
            .min_size(egui::vec2(0.0, 24.0))
            .rounding(egui::Rounding::same(5.0)),
    )
}

fn btn_danger(ui: &mut egui::Ui, enabled: bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    let btn = egui::Button::new(text)
        .fill(if enabled {
            C_DANGER
        } else {
            egui::Color32::from_rgb(45, 28, 40)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if enabled {
                egui::Color32::from_rgb(235, 130, 170)
            } else {
                C_STROKE_SOFT
            },
        ))
        .min_size(egui::vec2(0.0, 24.0))
        .rounding(egui::Rounding::same(5.0));
    ui.add_enabled(enabled, btn)
}

fn source_pill(ui: &mut egui::Ui, source: InstallSource) {
    let (label, fg, bg) = match source {
        InstallSource::Steam => ("Steam", C_STEAM, C_STEAM_BG),
        InstallSource::Registry => ("Win", C_REG, C_REG_BG),
        InstallSource::MicrosoftStore => ("Store", C_STORE, C_STORE_BG),
    };
    egui::Frame::none()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, C_STROKE_SOFT))
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).small().strong().color(fg));
        });
}

pub struct SweepApp {
    logo: Option<egui::TextureHandle>,
    /// Last `pixels_per_point` used when building `logo` (rebuild on DPI / monitor change).
    logo_ppp: f32,
    entries: Vec<InstalledEntry>,
    filter: String,
    selected_id: Option<String>,
    phase: AppPhase,
    status_message: String,
    leftover_items: Vec<LeftoverItem>,
    uninstall_log: String,
    worker_rx: Option<Receiver<WorkerMsg>>,
    _worker_tx: Option<Sender<WorkerMsg>>,
    confirm_uninstall: Option<InstalledEntry>,
}

enum WorkerMsg {
    UninstallDone(Result<std::process::ExitStatus, String>),
    LeftoversReady(Vec<LeftoverItem>),
    DeleteProgress { ok: usize, err: usize, last: String },
    DeleteDone,
}

/// Target height in UI **points**; texture is built at `points * pixels_per_point` for sharp HiDPI.
const LOGO_HEIGHT_POINTS: f32 = 28.0;

fn load_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = crate::logo_asset::decoded_logo();
    let ppp = ctx.pixels_per_point();
    let out_h = ((LOGO_HEIGHT_POINTS * ppp).round() as u32).clamp(24, 512);
    let aspect = img.width() as f32 / img.height().max(1) as f32;
    let out_w = (out_h as f32 * aspect).round().max(1.0) as u32;
    let resized = image::imageops::resize(
        &img,
        out_w,
        out_h,
        image::imageops::FilterType::Lanczos3,
    );
    let size = [resized.width() as usize, resized.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, resized.as_raw());
    Some(ctx.load_texture(
        "sweep_logo",
        color,
        egui::TextureOptions::LINEAR,
    ))
}

impl SweepApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        win_lnk::init_com();
        let mut style = (*cc.egui_ctx.style()).clone();
        apply_purple_dark_theme(&mut style);
        cc.egui_ctx.set_style(style);

        let ppp = cc.egui_ctx.pixels_per_point();
        let logo = load_logo_texture(&cc.egui_ctx);

        let mut app = Self {
            logo,
            logo_ppp: ppp,
            entries: Vec::new(),
            filter: String::new(),
            selected_id: None,
            phase: AppPhase::Idle,
            status_message: String::new(),
            leftover_items: Vec::new(),
            uninstall_log: String::new(),
            worker_rx: None,
            _worker_tx: None,
            confirm_uninstall: None,
        };
        app.refresh_list();
        app
    }

    fn refresh_list(&mut self) {
        let steam_games = steam::enumerate_steam_games();
        let steam_ids: HashSet<u32> = steam_games.iter().filter_map(|g| g.steam_app_id).collect();

        let mut list = enumerate::enumerate_registry_programs();
        list.retain(|e| {
            let Some(u) = e.uninstall_string.as_deref() else {
                return true;
            };
            let ul = u.to_lowercase();
            for id in &steam_ids {
                if ul.contains(&format!("steam://uninstall/{id}"))
                    || ul.contains(&format!("steam://uninstall/{id}/"))
                    || ul.contains(&format!("uninstall/{id}"))
                {
                    return false;
                }
            }
            true
        });

        list.extend(steam_games);
        list.extend(ms_store::enumerate_store_packages());
        list.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
        self.entries = list;
        let (nr, ns, nm) = self.entry_counts();
        self.status_message = format!(
            "{} total · {} Windows · {} Steam · {} Store",
            self.entries.len(),
            nr,
            ns,
            nm
        );
    }

    fn entry_counts(&self) -> (usize, usize, usize) {
        let n_reg = self
            .entries
            .iter()
            .filter(|e| matches!(e.source, InstallSource::Registry))
            .count();
        let n_steam = self
            .entries
            .iter()
            .filter(|e| matches!(e.source, InstallSource::Steam))
            .count();
        let n_ms = self
            .entries
            .iter()
            .filter(|e| matches!(e.source, InstallSource::MicrosoftStore))
            .count();
        (n_reg, n_steam, n_ms)
    }

    fn filtered_entries(&self) -> Vec<(usize, &InstalledEntry)> {
        let f = self.filter.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if f.is_empty() {
                    return true;
                }
                e.display_name.to_lowercase().contains(&f)
                    || e.publisher.to_lowercase().contains(&f)
                    || e
                        .install_location
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&f)
            })
            .collect()
    }

    fn selected_entry(&self) -> Option<&InstalledEntry> {
        let id = self.selected_id.as_ref()?;
        self.entries.iter().find(|e| &e.id == id)
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        let batch: Vec<WorkerMsg> = {
            let Some(rx) = self.worker_rx.as_ref() else {
                return;
            };
            let mut v = Vec::new();
            while let Ok(m) = rx.try_recv() {
                v.push(m);
            }
            v
        };
        if batch.is_empty() {
            return;
        }
        for msg in batch {
            match msg {
                WorkerMsg::UninstallDone(res) => match res {
                    Ok(st) => {
                        self.uninstall_log.push_str(&format!(
                            "Uninstaller finished (code: {:?}).\n",
                            st.code()
                        ));
                        let entry_for_scan = self
                            .entries
                            .iter()
                            .find(|e| Some(&e.id) == self.selected_id.as_ref())
                            .cloned();
                        self.refresh_list();
                        if self.selected_id.as_ref().map_or(true, |id| {
                            !self.entries.iter().any(|e| &e.id == id)
                        }) {
                            self.selected_id = None;
                        }
                        let n = self.entries.len();
                        if let Some(entry) = entry_for_scan {
                            self.phase = AppPhase::ScanningLeftovers;
                            self.status_message =
                                format!("List refreshed ({n} apps) · scanning leftovers…");
                            let tx = self._worker_tx.clone().unwrap();
                            std::thread::spawn(move || {
                                let found = scan_leftovers(&entry);
                                let _ = tx.send(WorkerMsg::LeftoversReady(found));
                            });
                        } else {
                            self.phase = AppPhase::Idle;
                            self.status_message = format!("List refreshed ({n} apps).");
                        }
                    }
                    Err(e) => {
                        self.uninstall_log.push_str(&format!("Uninstall error: {e}\n"));
                        self.phase = AppPhase::Idle;
                        self.status_message = e;
                    }
                },
                WorkerMsg::LeftoversReady(items) => {
                    self.leftover_items = items;
                    self.phase = AppPhase::ReviewingLeftovers;
                    self.status_message = format!(
                        "{} leftover items — review the window",
                        self.leftover_items.len()
                    );
                }
                WorkerMsg::DeleteProgress { ok, err, last } => {
                    self.status_message = format!("Deleted {ok} · errors {err} · {last}");
                }
                WorkerMsg::DeleteDone => {
                    self.phase = AppPhase::Idle;
                    self.leftover_items.clear();
                    self.refresh_list();
                    let n = self.entries.len();
                    self.status_message = format!("Cleanup done · list refreshed ({n} apps).");
                    self.worker_rx = None;
                    self._worker_tx = None;
                }
            }
        }
        ctx.request_repaint();
    }

    fn begin_uninstall(&mut self, ctx: &egui::Context, entry: InstalledEntry) {
        if !entry_can_remove(&entry) {
            self.status_message = "No uninstall command.".into();
            return;
        }

        let (tx, rx) = mpsc::channel();
        self._worker_tx = Some(tx.clone());
        self.worker_rx = Some(rx);
        self.phase = AppPhase::Uninstalling;
        self.uninstall_log.clear();
        if entry.source == InstallSource::MicrosoftStore {
            self.uninstall_log
                .push_str("Removing Microsoft Store package (per-user)…\n");
            self.status_message = "Removing Store package…".into();
        } else if entry.steam_app_id.is_some() {
            self.uninstall_log.push_str("Starting Steam uninstall flow…\n");
            self.status_message = "Complete the Steam dialog…".into();
        } else {
            self.uninstall_log.push_str("Starting official uninstaller…\n");
            self.status_message = "Finish the uninstaller window…".into();
        }

        std::thread::spawn(move || {
            let res = uninstall::run_official_uninstall(&entry).map_err(|e| e.to_string());
            let _ = tx.send(WorkerMsg::UninstallDone(res));
        });
        ctx.request_repaint();
    }

    fn request_uninstall_confirmation(&mut self) {
        let Some(entry) = self.selected_entry().cloned() else {
            self.status_message = "Select an app first.".into();
            return;
        };
        if !entry_can_remove(&entry) {
            self.status_message = "No uninstall command.".into();
            return;
        }
        self.confirm_uninstall = Some(entry);
    }

    fn delete_selected_leftovers(&mut self, ctx: &egui::Context) {
        let to_delete: Vec<LeftoverItem> = self
            .leftover_items
            .iter()
            .filter(|i| i.selected)
            .cloned()
            .collect();
        if to_delete.is_empty() {
            self.status_message = "No items checked.".into();
            return;
        }

        let (tx, rx) = mpsc::channel();
        self._worker_tx = Some(tx.clone());
        self.worker_rx = Some(rx);

        std::thread::spawn(move || {
            let mut ok = 0usize;
            let mut err = 0usize;
            for item in to_delete {
                let last = match delete_leftover(&item) {
                    Ok(()) => {
                        ok += 1;
                        item.path.to_string_lossy().to_string()
                    }
                    Err(e) => {
                        err += 1;
                        format!("{} — {e}", item.path.display())
                    }
                };
                let _ = tx.send(WorkerMsg::DeleteProgress { ok, err, last });
            }
            let _ = tx.send(WorkerMsg::DeleteDone);
        });
        ctx.request_repaint();
    }
}

impl eframe::App for SweepApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let ppp = ctx.pixels_per_point();
        if (self.logo_ppp - ppp).abs() > 1e-4 {
            self.logo_ppp = ppp;
            self.logo = load_logo_texture(ctx);
        }

        self.poll_worker(ctx);

        if let Some(entry) = self.confirm_uninstall.clone() {
            egui::Window::new(" ")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::window(&ctx.style())
                        .fill(C_PANEL)
                        .stroke(egui::Stroke::new(1.0, C_STROKE))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(16.0),
                )
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Uninstall")
                            .small()
                            .color(C_ACCENT_HOVER)
                            .strong(),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&entry.display_name)
                            .color(C_TEXT)
                            .strong()
                            .size(14.0),
                    );
                    ui.add_space(6.0);
                    if entry.source == InstallSource::MicrosoftStore {
                        ui.label(
                            egui::RichText::new(
                                "Removes this app for your user (Microsoft Store / AppX). Some built-in apps cannot be removed.",
                            )
                            .small()
                            .color(C_STORE),
                        );
                    } else if entry.steam_app_id.is_some() {
                        ui.label(
                            egui::RichText::new("Opens Steam’s uninstall flow for this title.")
                                .small()
                                .color(C_STEAM),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Runs the publisher’s uninstaller, then scans leftovers.")
                                .small()
                                .color(C_MUTED),
                        );
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if btn_secondary(ui, "Cancel").clicked() {
                            self.confirm_uninstall = None;
                        }
                        if btn_primary(ui, true, "Continue").clicked() {
                            self.confirm_uninstall = None;
                            self.begin_uninstall(ctx, entry);
                        }
                    });
                });
        }

        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::none()
                    .fill(C_PANEL)
                    .stroke(egui::Stroke::new(1.0, C_STROKE_SOFT))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex) = &self.logo {
                                let ppp = ui.ctx().pixels_per_point();
                                let px = tex.size_vec2();
                                // 1 texture pixel ≈ 1 screen pixel → avoids blurry downscale in the header.
                                let size_pt = egui::vec2(px.x / ppp, px.y / ppp);
                                ui.add(
                                    egui::Image::new((tex.id(), size_pt))
                                        .maintain_aspect_ratio(false)
                                        .rounding(egui::Rounding::same(6.0)),
                                );
                                ui.add_space(8.0);
                            }
                            ui.label(
                                egui::RichText::new("Sweep")
                                    .strong()
                                    .color(C_ACCENT_HOVER)
                                    .size(14.0),
                            );
                            ui.label(
                                egui::RichText::new("Uninstall")
                                    .color(C_MUTED)
                                    .size(11.5),
                            );
                            ui.add_space(6.0);
                            let (nr, ns, nm) = self.entry_counts();
                            egui::Frame::none()
                                .fill(C_SURFACE)
                                .stroke(egui::Stroke::new(1.0, C_STROKE_SOFT))
                                .rounding(egui::Rounding::same(4.0))
                                .inner_margin(egui::Margin::symmetric(7.0, 3.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{nr} Win  ·  {ns} Steam  ·  {nm} Store"
                                        ))
                                        .small()
                                        .color(C_MUTED),
                                    );
                                });
                        });
                        ui.label(
                            egui::RichText::new(&self.status_message)
                                .small()
                                .color(C_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let idle = self.phase == AppPhase::Idle;
                        let can_rm = self
                            .selected_entry()
                            .map(entry_can_remove)
                            .unwrap_or(false);
                        if btn_primary(ui, idle && can_rm, "Uninstall").clicked() {
                            self.request_uninstall_confirmation();
                        }
                        if btn_secondary(ui, "Refresh").clicked() {
                            self.refresh_list();
                        }
                    });
                });
            });

        if self.phase == AppPhase::ReviewingLeftovers {
            egui::Window::new("Leftovers")
                .default_size([400.0, 300.0])
                .collapsible(false)
                .frame(
                    egui::Frame::window(&ctx.style())
                        .fill(C_PANEL)
                        .stroke(egui::Stroke::new(1.0, C_STROKE))
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(12.0),
                )
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Review")
                            .small()
                            .color(C_ACCENT_HOVER)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Includes AppData, shortcuts, registry — Steam folders if applicable.")
                            .small()
                            .color(C_MUTED),
                    );
                    ui.add_space(6.0);

                    let any = !self.leftover_items.is_empty();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if btn_secondary(ui, "All").clicked() {
                            for item in &mut self.leftover_items {
                                item.selected = true;
                            }
                        }
                        if btn_secondary(ui, "None").clicked() {
                            for item in &mut self.leftover_items {
                                item.selected = false;
                            }
                        }
                    });

                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            for item in &mut self.leftover_items {
                                let kind = match item.kind {
                                    LeftoverKind::Folder => "DIR",
                                    LeftoverKind::File => "LNK",
                                    LeftoverKind::RegistryKey => "REG",
                                };
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut item.selected, "");
                                    ui.label(
                                        egui::RichText::new(kind)
                                            .monospace()
                                            .small()
                                            .color(C_ACCENT),
                                    );
                                    ui.label(
                                        egui::RichText::new(item.path.to_string_lossy())
                                            .small()
                                            .color(C_TEXT),
                                    );
                                });
                                ui.label(
                                    egui::RichText::new(&item.reason)
                                        .small()
                                        .color(C_MUTED),
                                );
                                ui.add_space(2.0);
                            }
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if btn_secondary(ui, "Skip").clicked() {
                            self.phase = AppPhase::Idle;
                            self.leftover_items.clear();
                            self.worker_rx = None;
                            self._worker_tx = None;
                            self.refresh_list();
                            let n = self.entries.len();
                            self.status_message =
                                format!("Skipped cleanup · list refreshed ({n} apps).");
                        }
                        if btn_danger(ui, any, "Delete checked").clicked() {
                            self.delete_selected_leftovers(ctx);
                        }
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(C_BG).inner_margin(8.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(C_SURFACE)
                    .stroke(egui::Stroke::new(1.0, C_STROKE_SOFT))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Search")
                                    .small()
                                    .color(C_MUTED),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.filter)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("filter…"),
                            );
                        });
                    });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Select a row · Uninstall in the header")
                        .small()
                        .color(egui::Color32::from_rgb(95, 80, 125)),
                );
                ui.add_space(4.0);

                egui::Frame::none()
                    .fill(C_RAISED)
                    .stroke(egui::Stroke::new(1.0, C_STROKE_SOFT))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        let rows: Vec<(usize, &InstalledEntry)> = self.filtered_entries();
                        let mut pending_select: Option<String> = None;
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("prog_grid")
                                .striped(true)
                                .num_columns(4)
                                .spacing([6.0, 1.0])
                                .min_col_width(0.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Program")
                                            .small()
                                            .strong()
                                            .color(C_MUTED),
                                    );
                                    ui.label(
                                        egui::RichText::new("Publisher")
                                            .small()
                                            .strong()
                                            .color(C_MUTED),
                                    );
                                    ui.label(
                                        egui::RichText::new("Location")
                                            .small()
                                            .strong()
                                            .color(C_MUTED),
                                    );
                                    ui.label(
                                        egui::RichText::new(" ")
                                            .small()
                                            .strong()
                                            .color(C_MUTED),
                                    );
                                    ui.end_row();
                                    for (_i, e) in &rows {
                                        let e = *e;
                                        let selected =
                                            self.selected_id.as_deref() == Some(e.id.as_str());
                                        let r = ui.selectable_label(
                                            selected,
                                            egui::RichText::new(&e.display_name).size(11.0),
                                        );
                                        if r.clicked() {
                                            pending_select = Some(e.id.clone());
                                        }
                                        ui.label(
                                            egui::RichText::new(&e.publisher)
                                                .size(10.0)
                                                .color(C_MUTED),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                e.install_location.as_deref().unwrap_or("—"),
                                            )
                                            .size(9.5)
                                            .color(C_MUTED),
                                        );
                                        ui.horizontal(|ui| {
                                            source_pill(ui, e.source);
                                        });
                                        ui.end_row();
                                    }
                                });
                        });
                        if let Some(id) = pending_select {
                            self.selected_id = Some(id);
                        }
                    });

                if let Some(e) = self.selected_entry() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("Selected · {}", e.display_name))
                            .small()
                            .color(C_ACCENT_HOVER),
                    );
                }

                if !self.uninstall_log.is_empty() {
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(
                        egui::RichText::new("Activity log").small().color(C_MUTED),
                    )
                    .show(ui, |ui| {
                        ui.monospace(
                            egui::RichText::new(&self.uninstall_log)
                                .size(9.5)
                                .color(C_MUTED),
                        );
                    });
                }
            });
    }
}
