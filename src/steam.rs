use crate::models::{InstallSource, InstalledEntry};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

/// `steam.exe` from registry (for launching `steam://` URLs without going through `cmd`).
pub fn steam_client_exe() -> Option<PathBuf> {
    let root = steam_path_from_registry()?;
    let exe = root.join("steam.exe");
    exe.exists().then_some(exe)
}

fn steam_path_from_registry() -> Option<PathBuf> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(steam) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(path) = steam.get_value::<String, _>("SteamPath") {
            let p = PathBuf::from(path.replace('/', "\\"));
            if p.exists() {
                return Some(p);
            }
        }
    }
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [
        r"SOFTWARE\WOW6432Node\Valve\Steam",
        r"SOFTWARE\Valve\Steam",
    ] {
        if let Ok(steam) = hklm.open_subkey(subkey) {
            if let Ok(path) = steam.get_value::<String, _>("InstallPath") {
                let p = PathBuf::from(path.replace('/', "\\"));
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn extract_quoted_paths(text: &str) -> Vec<String> {
    let re = Regex::new(r#""path"\s+"([^"]+)""#).unwrap();
    re.captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str().replace("\\\\", "\\")))
        .collect()
}

pub fn steam_common_directories() -> Vec<PathBuf> {
    let Some(steam_root) = steam_path_from_registry() else {
        return Vec::new();
    };
    library_paths(&steam_root)
        .into_iter()
        .map(|p| p.join("steamapps").join("common"))
        .filter(|p| p.is_dir())
        .collect()
}

fn library_paths(steam_root: &Path) -> Vec<PathBuf> {
    let mut set: HashSet<PathBuf> = HashSet::new();
    set.insert(steam_root.to_path_buf());

    let vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
    if let Ok(s) = fs::read_to_string(&vdf) {
        for p in extract_quoted_paths(&s) {
            let pb = PathBuf::from(p);
            if pb.exists() {
                set.insert(pb);
            }
        }
    }
    set.into_iter().collect()
}

fn parse_acf(content: &str) -> Option<(u32, String)> {
    let mut appid: Option<u32> = None;
    let mut name: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("\"appid\"") {
            let rest = rest.trim_start();
            if let Some(q) = rest.split('"').nth(1) {
                appid = q.parse().ok();
            }
        } else if let Some(rest) = line.strip_prefix("\"name\"") {
            let rest = rest.trim_start();
            if let Some(q) = rest.split('"').nth(1) {
                name = Some(q.to_string());
            }
        }
    }
    match (appid, name) {
        (Some(id), Some(n)) if !n.is_empty() => Some((id, n)),
        _ => None,
    }
}

fn steam_exe(steam_root: &Path) -> PathBuf {
    steam_root.join("steam.exe")
}

pub fn enumerate_steam_games() -> Vec<InstalledEntry> {
    let Some(steam_root) = steam_path_from_registry() else {
        return Vec::new();
    };
    let steam_exe_path = steam_exe(&steam_root);
    if !steam_exe_path.exists() {
        return Vec::new();
    }

    let mut games = Vec::new();
    for lib in library_paths(&steam_root) {
        let apps = lib.join("steamapps");
        if !apps.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&apps) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for ent in entries.filter_map(Result::ok) {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                continue;
            }
            let path = ent.path();
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Some((appid, title)) = parse_acf(&content) else {
                continue;
            };
            let install_dir_line = content
                .lines()
                .find(|l| l.trim_start().starts_with("\"installdir\""));
            let installdir = install_dir_line.and_then(|l| {
                l.split('"')
                    .nth(3)
                    .map(|s| apps.join("common").join(s))
            });
            let install_location = installdir
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().to_string());

            let uninstall_string = Some(format!(
                "\"{}\" steam://uninstall/{}",
                steam_exe_path.display(),
                appid
            ));

            games.push(InstalledEntry {
                id: format!("steam:{appid}"),
                display_name: title,
                publisher: "Steam / Valve".to_string(),
                install_location,
                uninstall_string,
                quiet_uninstall_string: None,
                registry_uninstall_subkey: None,
                registry_hive_is_hkcu: false,
                source: InstallSource::Steam,
                steam_app_id: Some(appid),
                package_full_name: None,
            });
        }
    }

    games.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    games
}
