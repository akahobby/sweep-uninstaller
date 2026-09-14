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
        || e.uninstall_string
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
        || e.quiet_uninstall_string
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

fn btn_primary(
    ui: &mut egui::Ui,
    enabled: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
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

fn btn_danger(
    ui: &mut egui::Ui,
    enabled: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
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

/// One interaction rectangle covers every column and the empty space in a row.
fn program_row(
    ui: &mut egui::Ui,
    entry: &InstalledEntry,
    selected: bool,
    index: usize,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::click());
    let fill = if selected {
        C_ACCENT_DIM
    } else if response.hovered() {
        C_SURFACE
    } else if index % 2 == 0 {
        C_PANEL
    } else {
        C_RAISED
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    let width = (rect.width() - 62.0).max(0.0);
    let columns = [
        (entry.display_name.as_str(), 0.0, width * 0.34, C_TEXT, 11.0),
        (
            entry.publisher.as_str(),
            width * 0.34,
            width * 0.24,
            C_MUTED,
            10.0,
        ),
        (
            entry.install_location.as_deref().unwrap_or("—"),
            width * 0.58,
            width * 0.42,
            C_MUTED,
            9.5,
        ),
    ];
    for (text, offset, cell_width, color, size) in columns {
        let cell = egui::Rect::from_min_size(
            rect.min + egui::vec2(offset, 0.0),
            egui::vec2(cell_width, rect.height()),
        );
        ui.painter()
            .with_clip_rect(ui.clip_rect().intersect(cell.shrink2(egui::vec2(4.0, 0.0))))
            .text(
                cell.left_center() + egui::vec2(6.0, 0.0),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(size),
                color,
            );
    }
    let (label, fg, bg) = match entry.source {
        InstallSource::Steam => ("Steam", C_STEAM, C_STEAM_BG),
        InstallSource::Registry => ("Win", C_REG, C_REG_BG),
        InstallSource::MicrosoftStore => ("Store", C_STORE, C_STORE_BG),
    };
    let badge = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 57.0, rect.top() + 3.0),
        egui::vec2(52.0, 20.0),
    );
    ui.painter().rect_filled(badge, 4.0, bg);
    ui.painter().text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(9.5),
        fg,
    );
    response.on_hover_text(format!(
        "{}\n{}\n{}",
        entry.display_name,
        entry.publisher,
        entry.install_location.as_deref().unwrap_or("—")
    ))
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
    pending_cleanup: Option<InstalledEntry>,
}

enum WorkerMsg {
    UninstallDone {
        entry: InstalledEntry,
        result: Result<std::process::ExitStatus, String>,
    },
    LeftoversReady(Vec<LeftoverItem>),
    DeleteProgress {
        ok: usize,
        err: usize,
        last: String,
    },
    DeleteDone {
        deleted: Vec<LeftoverItem>,
        failed: Vec<(LeftoverItem, String)>,
    },
}

/// Target height in UI **points**; texture is built at `points * pixels_per_point` for sharp HiDPI.
const LOGO_HEIGHT_POINTS: f32 = 28.0;

fn load_logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = crate::logo_asset::decoded_logo();
    let ppp = ctx.pixels_per_point();
    let out_h = ((LOGO_HEIGHT_POINTS * ppp).round() as u32).clamp(24, 512);
    let aspect = img.width() as f32 / img.height().max(1) as f32;
    let out_w = (out_h as f32 * aspect).round().max(1.0) as u32;
    let resized =
        image::imageops::resize(&img, out_w, out_h, image::imageops::FilterType::Lanczos3);
    let size = [resized.width() as usize, resized.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, resized.as_raw());
    Some(ctx.load_texture("sweep_logo", color, egui::TextureOptions::LINEAR))
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
            pending_cleanup: None,
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
            !crate::workflow::steam_uninstall_id(u)
                .map(|id| steam_ids.contains(&id))
                .unwrap_or(false)
        });

        list.extend(steam_games);
        list.extend(ms_store::enumerate_store_packages());
        list.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        });
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
                    || e.install_location
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
                WorkerMsg::UninstallDone { entry, result } => {
                    self.worker_rx = None;
                    self._worker_tx = None;
                    match result {
                        Ok(st) if st.success() => {
                            self.uninstall_log.push_str(&format!(
                                "Uninstaller process exited for {} (code: {:?}).\n",
                                entry.display_name,
                                st.code()
                            ));
                            self.pending_cleanup = Some(entry);
                            self.check_removal(ctx);
                        }
                        Ok(st) => {
                            self.phase = AppPhase::Idle;
                            self.status_message = match st.code() {
                                Some(1602) => "Uninstall canceled. Cleanup was not started.".into(),
                                Some(1641 | 3010) => {
                                    "Restart Windows to finish uninstalling before cleaning up."
                                        .into()
                                }
                                _ => format!(
                                    "Uninstaller exited with code {:?}. Cleanup was not started.",
                                    st.code()
                                ),
                            };
                            self.uninstall_log
                                .push_str(&format!("{}\n", self.status_message));
                        }
                        Err(e) => {
                            self.uninstall_log
                                .push_str(&format!("Uninstall error: {e}\n"));
                            self.phase = AppPhase::Idle;
                            self.status_message = e;
                        }
                    }
                }
                WorkerMsg::LeftoversReady(items) => {
                    self.worker_rx = None;
                    self._worker_tx = None;
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
                WorkerMsg::DeleteDone { deleted, failed } => {
                    self.leftover_items.retain(|item| !deleted.contains(item));
                    for (item, error) in &failed {
                        self.uninstall_log.push_str(&format!(
                            "Cleanup failed: {} — {error}\n",
                            item.path.display()
                        ));
                    }
                    self.refresh_list();
                    self.phase = if self.leftover_items.is_empty() {
                        AppPhase::Idle
                    } else {
                        AppPhase::ReviewingLeftovers
                    };
                    self.status_message = format!(
                        "Cleanup: {} deleted · {} failed · {} remaining",
                        deleted.len(),
                        failed.len(),
                        self.leftover_items.len()
                    );
                    self.worker_rx = None;
                    self._worker_tx = None;
                }
            }
        }
        ctx.request_repaint();
    }

    fn check_removal(&mut self, ctx: &egui::Context) {
        let Some(entry) = self.pending_cleanup.clone() else {
            return;
        };
        self.refresh_list();
        if self.entries.iter().any(|e| e.id == entry.id) {
            self.phase = AppPhase::AwaitingRemoval;
            self.status_message = format!(
                "{} is still listed. Finish its uninstaller, then check again.",
                entry.display_name
            );
            return;
        }
        self.pending_cleanup = None;
        self.phase = AppPhase::ScanningLeftovers;
        self.status_message = format!("Scanning leftovers for {}…", entry.display_name);
        let (tx, rx) = mpsc::channel();
        self.worker_rx = Some(rx);
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let found = scan_leftovers(&entry);
            let _ = tx.send(WorkerMsg::LeftoversReady(found));
            wake.request_repaint();
        });
    }

    fn begin_uninstall(&mut self, ctx: &egui::Context, entry: InstalledEntry) {
        if self.phase != AppPhase::Idle || !entry_can_remove(&entry) {
            return;
        }
        self.leftover_items.clear();

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
            self.uninstall_log
                .push_str("Starting Steam uninstall flow…\n");
            self.status_message = "Complete the Steam dialog…".into();
        } else {
            self.uninstall_log
                .push_str("Starting official uninstaller…\n");
            self.status_message = "Finish the uninstaller window…".into();
        }

        let wake = ctx.clone();
        std::thread::spawn(move || {
            let res = uninstall::run_official_uninstall(&entry).map_err(|e| e.to_string());
            let _ = tx.send(WorkerMsg::UninstallDone { entry, result: res });
            wake.request_repaint();
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
        if self.phase != AppPhase::ReviewingLeftovers {
            return;
        }
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

        self.phase = AppPhase::DeletingLeftovers;
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let mut deleted = Vec::new();
            let mut failed = Vec::new();
            let mut ok = 0usize;
            let mut err = 0usize;
            for item in to_delete {
                let last = match delete_leftover(&item) {
                    Ok(()) => {
                        ok += 1;
                        deleted.push(item.clone());
                        item.path.to_string_lossy().to_string()
                    }
                    Err(e) => {
                        err += 1;
                        failed.push((item.clone(), e.clone()));
                        format!("{} — {e}", item.path.display())
                    }
                };
                let _ = tx.send(WorkerMsg::DeleteProgress { ok, err, last });
                wake.request_repaint();
            }
            let _ = tx.send(WorkerMsg::DeleteDone { deleted, failed });
            wake.request_repaint();
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
                            ui.label(egui::RichText::new("Uninstall").color(C_MUTED).size(11.5));
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
                        let idle = self.phase == AppPhase::Idle && self.confirm_uninstall.is_none();
                        let can_rm = self.selected_entry().map(entry_can_remove).unwrap_or(false);
                        if btn_primary(ui, idle && can_rm, "Uninstall").clicked() {
                            self.request_uninstall_confirmation();
                        }
                        if ui.add_enabled(idle, egui::Button::new("Refresh")).clicked() {
                            self.refresh_list();
                        }
                    });
                });
            });

        if self.phase == AppPhase::AwaitingRemoval {
            egui::Window::new("Finish uninstalling")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&self.status_message);
                    ui.label("If you canceled, skip cleanup to return to the app list.");
                    if btn_secondary(ui, "Check again").clicked() {
                        self.check_removal(ctx);
                    }
                    if btn_secondary(ui, "Skip cleanup").clicked() {
                        self.pending_cleanup = None;
                        self.phase = AppPhase::Idle;
                        self.refresh_list();
                    }
                });
        }

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
                        egui::RichText::new(
                            "Includes AppData, shortcuts, registry — Steam folders if applicable.",
                        )
                        .small()
                        .color(C_MUTED),
                    );
                    ui.add_space(6.0);

                    let any = self.leftover_items.iter().any(|item| item.selected);
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
                                ui.label(egui::RichText::new(&item.reason).small().color(C_MUTED));
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
                            ui.label(egui::RichText::new("Search").small().color(C_MUTED));
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
                            for (row_index, (_, entry)) in rows.iter().enumerate() {
                                let selected =
                                    self.selected_id.as_deref() == Some(entry.id.as_str());
                                if program_row(ui, entry, selected, row_index).clicked() {
                                    pending_select = Some(entry.id.clone());
                                }
                            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::ExitStatusExt;

    fn entry() -> InstalledEntry {
        InstalledEntry {
            id: "steam:123".into(),
            display_name: "Example Game".into(),
            publisher: "Steam / Valve".into(),
            install_location: Some(r"C:\Games\Example".into()),
            uninstall_string: Some("steam://uninstall/123".into()),
            quiet_uninstall_string: None,
            registry_uninstall_subkey: None,
            registry_hive_is_hkcu: false,
            source: InstallSource::Steam,
            steam_app_id: Some(123),
            package_full_name: None,
        }
    }

    fn app() -> SweepApp {
        SweepApp {
            logo: None,
            logo_ppp: 1.0,
            entries: vec![entry()],
            filter: String::new(),
            selected_id: Some("steam:999".into()),
            phase: AppPhase::Uninstalling,
            status_message: String::new(),
            leftover_items: Vec::new(),
            uninstall_log: String::new(),
            worker_rx: None,
            _worker_tx: None,
            confirm_uninstall: None,
            pending_cleanup: None,
        }
    }

    #[test]
    fn canceled_failed_and_reboot_required_uninstalls_never_scan() {
        for code in [1602, 1603, 1641, 3010, 1] {
            let mut app = app();
            let (tx, rx) = mpsc::channel();
            app.worker_rx = Some(rx);
            tx.send(WorkerMsg::UninstallDone {
                entry: entry(),
                result: Ok(std::process::ExitStatus::from_raw(code)),
            })
            .unwrap();
            app.poll_worker(&egui::Context::default());
            assert_eq!(app.phase, AppPhase::Idle);
            assert!(app.pending_cleanup.is_none());
            assert!(app.worker_rx.is_none());
            assert!(app.leftover_items.is_empty());
        }
    }

    #[test]
    fn deleting_phase_blocks_repeat_deletions() {
        let mut app = app();
        app.phase = AppPhase::DeletingLeftovers;
        app.delete_selected_leftovers(&egui::Context::default());
        assert_eq!(app.phase, AppPhase::DeletingLeftovers);
        assert!(app.worker_rx.is_none());
    }

    #[test]
    fn every_column_and_row_padding_is_clickable() {
        let ctx = egui::Context::default();
        let entry = entry();
        let mut rect = egui::Rect::NOTHING;
        let mut draw = |input| {
            let mut clicked = false;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = program_row(ui, &entry, false, 0);
                    rect = response.rect;
                    clicked = response.clicked();
                });
            });
            (clicked, rect)
        };
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 200.0),
            )),
            ..Default::default()
        };
        let (_, row) = draw(input());
        for fraction in [0.01, 0.15, 0.42, 0.75, 0.95, 0.99] {
            let pos = egui::pos2(row.left() + row.width() * fraction, row.center().y);
            let mut press = input();
            press.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ];
            draw(press);
            let mut release = input();
            release.events = vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }];
            assert!(draw(release).0, "row click at {fraction}");
        }
    }
}
