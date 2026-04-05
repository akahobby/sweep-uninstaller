use crate::models::{InstallSource, InstalledEntry};
use crate::steam;
use regex::Regex;
use std::io;
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn normalize_msi_uninstall(s: &str) -> String {
    let lower = s.to_lowercase();
    if !lower.contains("msiexec") {
        return s.to_string();
    }
    let re_i = Regex::new(r"(?i)/I(\{[0-9A-Fa-f-]{36}\})").unwrap();
    re_i.replace_all(s, "/X$1").to_string()
}

fn escape_powershell_single_quoted(s: &str) -> String {
    s.replace('\'', "''")
}

/// `Remove-AppxPackage` for the current user (Microsoft Store / AppX / MSIX).
fn run_remove_appx_package(package_full_name: &str) -> io::Result<std::process::ExitStatus> {
    let safe = escape_powershell_single_quoted(package_full_name);
    let script = format!("Remove-AppxPackage -Package '{safe}' -ErrorAction Stop");
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NoLogo",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .spawn()?
        .wait()
}

/// Runs the vendor uninstall command and waits for the top-level process to exit.
pub fn run_official_uninstall(entry: &InstalledEntry) -> io::Result<std::process::ExitStatus> {
    if entry.source == InstallSource::MicrosoftStore {
        if let Some(full) = entry.package_full_name.as_deref() {
            return run_remove_appx_package(full);
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Store package has no PackageFullName",
        ));
    }

    if let Some(appid) = entry.steam_app_id {
        if let Some(steam_exe) = steam::steam_client_exe() {
            return Command::new(&steam_exe)
                .arg(format!("steam://uninstall/{appid}"))
                .stdin(Stdio::null())
                .spawn()?
                .wait();
        }
    }

    let cmdline = entry
        .uninstall_string
        .as_deref()
        .or(entry.quiet_uninstall_string.as_deref())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No uninstall command"))?;

    let cmdline = normalize_msi_uninstall(cmdline.trim());

    #[cfg(windows)]
    {
        Command::new("cmd.exe")
            .arg("/C")
            .raw_arg(cmdline)
            .stdin(Stdio::null())
            .spawn()?
            .wait()
    }
    #[cfg(not(windows))]
    {
        Command::new("sh")
            .args(["-c", cmdline])
            .stdin(Stdio::null())
            .spawn()?
            .wait()
    }
}
