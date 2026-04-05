use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallSource {
    Registry,
    Steam,
    /// Microsoft Store / AppX / MSIX (per-user package).
    MicrosoftStore,
}

#[derive(Clone, Debug)]
pub struct InstalledEntry {
    pub id: String,
    pub display_name: String,
    pub publisher: String,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    /// HKLM or HKCU subkey name under ...\Uninstall\, if from registry
    pub registry_uninstall_subkey: Option<String>,
    pub registry_hive_is_hkcu: bool,
    pub source: InstallSource,
    pub steam_app_id: Option<u32>,
    /// `PackageFullName` for Microsoft Store / AppX removal (`Remove-AppxPackage`).
    pub package_full_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeftoverKind {
    Folder,
    File,
    RegistryKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeftoverItem {
    pub kind: LeftoverKind,
    pub path: PathBuf,
    /// Human-readable reason we flagged this
    pub reason: String,
    pub selected: bool,
}

impl LeftoverItem {
    pub fn new(kind: LeftoverKind, path: PathBuf, reason: impl Into<String>) -> Self {
        Self {
            kind,
            path,
            reason: reason.into(),
            selected: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppPhase {
    Idle,
    Uninstalling,
    ScanningLeftovers,
    ReviewingLeftovers,
}
