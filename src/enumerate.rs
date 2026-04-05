use crate::models::{InstallSource, InstalledEntry};
use std::collections::HashMap;
use winreg::enums::*;
use winreg::RegKey;

fn read_string(key: &RegKey, name: &str) -> Option<String> {
    key.get_value(name).ok()
}

fn read_dword(key: &RegKey, name: &str) -> Option<u32> {
    key.get_value(name).ok()
}

fn is_system_component(key: &RegKey) -> bool {
    read_dword(key, "SystemComponent").unwrap_or(0) != 0
}

fn should_skip(key: &RegKey, display_name: Option<&String>) -> bool {
    if display_name.map(|s| s.trim().is_empty()).unwrap_or(true) {
        return true;
    }
    if is_system_component(key) {
        return true;
    }
    false
}

fn gather_from_hive(hive: &RegKey, hive_name: &str, hkcu: bool) -> Vec<InstalledEntry> {
    let uninstall = match hive.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall") {
        Ok(k) => k,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    for sub in uninstall.enum_keys().filter_map(Result::ok) {
        let key = match uninstall.open_subkey(&sub) {
            Ok(k) => k,
            Err(_) => continue,
        };
        let display_name = read_string(&key, "DisplayName");
        if should_skip(&key, display_name.as_ref()) {
            continue;
        }
        let display_name = display_name.unwrap_or_else(|| sub.clone());
        let publisher = read_string(&key, "Publisher").unwrap_or_default();
        let install_location = read_string(&key, "InstallLocation")
            .map(|s| s.trim().trim_end_matches('\\').to_string())
            .filter(|s| !s.is_empty());
        let uninstall_string = read_string(&key, "UninstallString").filter(|s| !s.trim().is_empty());
        let quiet_uninstall_string =
            read_string(&key, "QuietUninstallString").filter(|s| !s.trim().is_empty());

        let id = format!("reg:{hive_name}\\Uninstall\\{sub}");
        out.push(InstalledEntry {
            id,
            display_name,
            publisher,
            install_location,
            uninstall_string,
            quiet_uninstall_string,
            registry_uninstall_subkey: Some(sub),
            registry_hive_is_hkcu: hkcu,
            source: InstallSource::Registry,
            steam_app_id: None,
            package_full_name: None,
        });
    }
    out
}

/// 64-bit and 32-bit registry views on HKLM.
pub fn enumerate_registry_programs() -> Vec<InstalledEntry> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let mut map: HashMap<String, InstalledEntry> = HashMap::new();

    for e in gather_from_hive(&hklm, "HKLM", false) {
        if let Some(sub) = &e.registry_uninstall_subkey {
            map.insert(format!("HKLM:{sub}"), e);
        }
    }

    // 32-bit registrations on 64-bit Windows live under WOW6432Node
    if let Ok(uninstall) = hklm.open_subkey_with_flags(
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        KEY_READ,
    ) {
        for sub in uninstall.enum_keys().filter_map(Result::ok) {
            let key = match uninstall.open_subkey(&sub) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let display_name = read_string(&key, "DisplayName");
            if should_skip(&key, display_name.as_ref()) {
                continue;
            }
            let display_name = display_name.unwrap_or_else(|| sub.clone());
            let publisher = read_string(&key, "Publisher").unwrap_or_default();
            let install_location = read_string(&key, "InstallLocation")
                .map(|s| s.trim().trim_end_matches('\\').to_string())
                .filter(|s| !s.is_empty());
            let uninstall_string = read_string(&key, "UninstallString").filter(|s| !s.trim().is_empty());
            let quiet_uninstall_string =
                read_string(&key, "QuietUninstallString").filter(|s| !s.trim().is_empty());

            let id = format!("reg:HKLM\\WOW6432Node\\Uninstall\\{sub}");
            let sub_key = sub.clone();
            let e = InstalledEntry {
                id,
                display_name,
                publisher,
                install_location,
                uninstall_string,
                quiet_uninstall_string,
                registry_uninstall_subkey: Some(sub),
                registry_hive_is_hkcu: false,
                source: InstallSource::Registry,
                steam_app_id: None,
                package_full_name: None,
            };
            map.insert(format!("HKLM_WOW64:{sub_key}"), e);
        }
    }

    for e in gather_from_hive(&hkcu, "HKCU", true) {
        if let Some(sub) = &e.registry_uninstall_subkey {
            map.insert(format!("HKCU:{sub}"), e);
        }
    }

    let mut v: Vec<_> = map.into_values().collect();
    v.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    v
}
