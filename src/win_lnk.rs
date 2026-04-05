use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::Once;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH};

static COM_INIT: Once = Once::new();

/// Returns target path for a .lnk file, if resolvable.
pub fn resolve_lnk_target(lnk_path: &Path) -> Option<String> {
    let wide_path: Vec<u16> = lnk_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    COM_INIT.call_once(|| {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    });

    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let pf: IPersistFile = link.cast().ok()?;
        pf.Load(PCWSTR(wide_path.as_ptr()), STGM_READ).ok()?;

        let mut buf = vec![0u16; MAX_PATH as usize];
        let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();
        link.GetPath(
            &mut buf,
            std::ptr::addr_of_mut!(find_data),
            SLGP_RAWPATH.0 as u32,
        )
        .ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
        if len == 0 {
            return None;
        }
        let s = String::from_utf16_lossy(&buf[..len]);
        let s = s.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

/// Best-effort COM init for the process (called from main).
pub fn init_com() {
    COM_INIT.call_once(|| {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    });
}

#[allow(dead_code)]
pub fn uninit_com() {
    unsafe {
        CoUninitialize();
    }
}
