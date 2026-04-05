//! Microsoft Store / AppX / MSIX packages for the current user (WinRT `PackageManager`).

use crate::models::{InstallSource, InstalledEntry};
use windows::ApplicationModel::PackageSignatureKind;
use windows::Management::Deployment::PackageManager;

fn hstring_to_string(h: windows::core::HSTRING) -> String {
    h.to_string()
}

/// Enumerates non-framework AppX packages the current user can manage (Store, sideload, etc.).
#[cfg(windows)]
pub fn enumerate_store_packages() -> Vec<InstalledEntry> {
    let Ok(pm) = PackageManager::new() else {
        return Vec::new();
    };
    let Ok(iterable) = pm.FindPackages() else {
        return Vec::new();
    };

    let Ok(iter) = iterable.First() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for pkg in iter {
        if pkg.IsFramework().unwrap_or(true) {
            continue;
        }
        if pkg.IsResourcePackage().unwrap_or(false) {
            continue;
        }

        let Ok(sig) = pkg.SignatureKind() else {
            continue;
        };
        // Skip core OS packages signed as System; keep Store, sideload, and enterprise.
        if sig == PackageSignatureKind::System || sig == PackageSignatureKind::None {
            continue;
        }

        let Ok(id) = pkg.Id() else {
            continue;
        };
        let Ok(full) = id.FullName() else {
            continue;
        };
        let full_s = hstring_to_string(full);

        let display_name = pkg
            .DisplayName()
            .ok()
            .map(hstring_to_string)
            .filter(|s| !s.trim().is_empty())
            .or_else(|| id.Name().ok().map(hstring_to_string))
            .unwrap_or_else(|| full_s.clone());

        let publisher = pkg
            .PublisherDisplayName()
            .ok()
            .map(hstring_to_string)
            .filter(|s| !s.trim().is_empty())
            .or_else(|| id.Publisher().ok().map(hstring_to_string))
            .unwrap_or_default();

        let install_location = pkg
            .InstalledLocation()
            .ok()
            .and_then(|loc| loc.Path().ok())
            .map(hstring_to_string)
            .filter(|s| !s.is_empty());

        out.push(InstalledEntry {
            id: format!("store:{full_s}"),
            display_name,
            publisher,
            install_location,
            uninstall_string: None,
            quiet_uninstall_string: None,
            registry_uninstall_subkey: None,
            registry_hive_is_hkcu: false,
            source: InstallSource::MicrosoftStore,
            steam_app_id: None,
            package_full_name: Some(full_s),
        });
    }

    out.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    out
}

#[cfg(not(windows))]
pub fn enumerate_store_packages() -> Vec<InstalledEntry> {
    Vec::new()
}
