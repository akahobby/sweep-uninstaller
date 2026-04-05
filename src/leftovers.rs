use crate::models::{InstalledEntry, LeftoverItem, LeftoverKind};
use crate::steam;
use crate::win_lnk;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use winreg::enums::*;
use winreg::RegKey;

const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "your", "app", "application", "free", "trial", "bit",
    "x64", "x86",
];

fn tokenize_name(name: &str) -> Vec<String> {
    let mut tokens: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .map(|s| s.to_lowercase())
        .filter(|s| s.len() >= 3 && !STOP_WORDS.contains(&s.as_str()))
        .collect();
    tokens.sort_by_key(|b| std::cmp::Reverse(b.len()));
    tokens.dedup();
    tokens
}

fn path_is_safe_candidate(p: &Path) -> bool {
    let s = p.to_string_lossy().to_lowercase();
    if s.contains("\\windows\\") || s.ends_with("\\windows") {
        return false;
    }
    if s.contains("\\microsoft\\windows\\") {
        return false;
    }
    if s.contains("\\program files\\windows apps\\") {
        return false;
    }
    true
}

fn folder_matches_tokens(folder_name: &str, tokens: &[String]) -> bool {
    let f = folder_name.to_lowercase();
    if tokens.is_empty() {
        return false;
    }
    // Strong match: longest token appears in folder name
    if f.contains(&tokens[0]) {
        return true;
    }
    // Secondary: two smaller tokens both appear
    if tokens.len() >= 2 {
        let a = tokens.iter().find(|t| t.len() >= 4);
        let b = tokens.iter().skip(1).find(|t| t.len() >= 4);
        if let (Some(x), Some(y)) = (a, b) {
            if x != y && f.contains(x.as_str()) && f.contains(y.as_str()) {
                return true;
            }
        }
    }
    false
}

fn enumerate_start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(pd) = env::var("ProgramData") {
        roots.push(PathBuf::from(pd).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    if let Ok(ad) = env::var("APPDATA") {
        roots.push(PathBuf::from(ad).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    roots.into_iter().filter(|p| p.is_dir()).collect()
}

/// Collect probable leftovers after the official uninstaller has run.
pub fn scan_leftovers(entry: &InstalledEntry) -> Vec<LeftoverItem> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut items = Vec::new();

    let tokens = tokenize_name(&entry.display_name);
    let pub_tokens = tokenize_name(&entry.publisher);

    // 1) Install folder still on disk (exact path only)
    if let Some(loc) = &entry.install_location {
        let p = PathBuf::from(loc);
        if p.exists() {
            let kind = if p.is_dir() {
                LeftoverKind::Folder
            } else {
                LeftoverKind::File
            };
            let pb = p.clone();
            if seen.insert(pb.clone()) {
                items.push(LeftoverItem::new(
                    kind,
                    pb,
                    "Install location still present",
                ));
            }
        }
    }

    // 2) Steam library folders (all libraries) matching game title
    if entry.steam_app_id.is_some() {
        let appid = entry.steam_app_id.unwrap();
        let slug = entry.display_name.to_lowercase();
        for common in steam::steam_common_directories() {
            for ent in WalkDir::new(&common).max_depth(1).into_iter().filter_map(Result::ok) {
                if !ent.file_type().is_dir() {
                    continue;
                }
                let name = ent.file_name().to_string_lossy().to_lowercase();
                if name.contains(&slug) || slug.contains(&name) {
                    let pb = ent.path().to_path_buf();
                    if seen.insert(pb.clone()) {
                        items.push(LeftoverItem::new(
                            LeftoverKind::Folder,
                            pb,
                            format!("Steam library folder (app {appid})"),
                        ));
                    }
                }
            }
        }
    }

    // 3) AppData / Local / Roaming / LocalLow / ProgramData — top-level folders only (fast)
    let mut scan_roots: Vec<PathBuf> = Vec::new();
    if let Ok(la) = env::var("LOCALAPPDATA") {
        scan_roots.push(PathBuf::from(la));
    }
    if let Ok(ad) = env::var("APPDATA") {
        scan_roots.push(PathBuf::from(ad));
    }
    if let Ok(la) = env::var("LOCALAPPDATA") {
        if let Some(parent) = Path::new(&la).parent() {
            scan_roots.push(parent.join("LocalLow"));
        }
    }
    if let Ok(pd) = env::var("ProgramData") {
        scan_roots.push(PathBuf::from(pd));
    }

    for root in scan_roots {
        if !root.is_dir() || !path_is_safe_candidate(&root) {
            continue;
        }
        let Ok(read) = fs::read_dir(&root) else {
            continue;
        };
        for ent in read.filter_map(Result::ok) {
            let Ok(ft) = ent.file_type() else {
                continue;
            };
            if !ft.is_dir() {
                continue;
            }
            let name = ent.file_name().to_string_lossy().to_string();
            let path = ent.path();
            if !path_is_safe_candidate(&path) {
                continue;
            }
            let by_app = folder_matches_tokens(&name, &tokens);
            let by_pub = !pub_tokens.is_empty()
                && entry.publisher.len() > 2
                && folder_matches_tokens(&name, &pub_tokens);
            if by_app || by_pub {
                if seen.insert(path.clone()) {
                    items.push(LeftoverItem::new(
                        LeftoverKind::Folder,
                        path,
                        if by_app {
                            "Name matched profile / program data folder"
                        } else {
                            "Publisher matched data folder"
                        },
                    ));
                }
            }
        }
    }

    // 4) Start Menu shortcuts
    for root in enumerate_start_menu_roots() {
        for w in WalkDir::new(&root).max_depth(4).into_iter().filter_map(Result::ok) {
            let p = w.path();
            if !p.is_file() {
                continue;
            }
            if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("lnk")) != Some(true)
            {
                continue;
            }
            let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            let mut hit = tokens.iter().any(|t| stem.contains(t.as_str()));
            if !hit {
                hit = stem.contains(&entry.display_name.to_lowercase());
            }
            let target_hit = win_lnk::resolve_lnk_target(p).map(|t| {
                let tl = t.to_lowercase();
                entry
                    .install_location
                    .as_ref()
                    .map(|loc| tl.starts_with(&loc.to_lowercase()))
                    .unwrap_or(false)
            });
            if hit || target_hit == Some(true) {
                if seen.insert(p.to_path_buf()) {
                    items.push(LeftoverItem::new(
                        LeftoverKind::File,
                        p.to_path_buf(),
                        "Start menu shortcut",
                    ));
                }
            }
        }
    }

    // 5) Uninstall registry key if it still exists
    if let Some(sub) = &entry.registry_uninstall_subkey {
        if entry.registry_hive_is_hkcu {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let path = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
            if let Ok(k) = hkcu.open_subkey(path) {
                if k.open_subkey(sub).is_ok() {
                    let disp = format!("HKCU\\{path}\\{sub}");
                    let pb = PathBuf::from(&disp);
                    if seen.insert(pb.clone()) {
                        items.push(LeftoverItem::new(
                            LeftoverKind::RegistryKey,
                            pb,
                            "Leftover uninstall registry entry",
                        ));
                    }
                }
            }
        } else {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            for path in [
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            ] {
                if let Ok(k) = hklm.open_subkey(path) {
                    if k.open_subkey(sub).is_ok() {
                        let disp = format!("HKLM\\{path}\\{sub}");
                        let pb = PathBuf::from(&disp);
                        if seen.insert(pb.clone()) {
                            items.push(LeftoverItem::new(
                                LeftoverKind::RegistryKey,
                                pb,
                                "Leftover uninstall registry entry",
                            ));
                        }
                    }
                }
            }
        }
    }

    // Sort: folders, files, registry; then path
    items.sort_by(|a, b| {
        let ord = |k: &LeftoverKind| match k {
            LeftoverKind::Folder => 0,
            LeftoverKind::File => 1,
            LeftoverKind::RegistryKey => 2,
        };
        ord(&a.kind).cmp(&ord(&b.kind)).then_with(|| {
            a.path
                .to_string_lossy()
                .to_lowercase()
                .cmp(&b.path.to_string_lossy().to_lowercase())
        })
    });

    items
}

pub fn delete_leftover(item: &LeftoverItem) -> Result<(), String> {
    match item.kind {
        LeftoverKind::Folder => {
            fs::remove_dir_all(&item.path).map_err(|e| e.to_string())?;
        }
        LeftoverKind::File => {
            fs::remove_file(&item.path).map_err(|e| e.to_string())?;
        }
        LeftoverKind::RegistryKey => {
            delete_registry_display_path(&item.path.to_string_lossy())?;
        }
    }
    Ok(())
}

fn delete_registry_display_path(s: &str) -> Result<(), String> {
    let (hive, rest) = if let Some(r) = s.strip_prefix("HKCU\\") {
        (HKEY_CURRENT_USER, r)
    } else if let Some(r) = s.strip_prefix("HKLM\\") {
        (HKEY_LOCAL_MACHINE, r)
    } else {
        return Err("Invalid registry path".into());
    };
    let parts: Vec<&str> = rest.split('\\').collect();
    if parts.len() < 2 {
        return Err("Registry path too short".into());
    }
    let subkey_name = parts.last().unwrap();
    let parent = parts[..parts.len() - 1].join("\\");
    let parent_key = RegKey::predef(hive)
        .open_subkey_with_flags(&parent, KEY_ALL_ACCESS)
        .map_err(|e| e.to_string())?;
    parent_key
        .delete_subkey_all(subkey_name)
        .map_err(|e| e.to_string())?;
    Ok(())
}
