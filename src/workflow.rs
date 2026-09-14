pub fn windows_path_key(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// Parse the whole Steam app ID so app 123 does not hide app 1234.
pub fn steam_uninstall_id(command: &str) -> Option<u32> {
    let lower = command.to_ascii_lowercase();
    let rest = lower.split_once("steam://uninstall/")?.1;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    if let Some(next) = rest.as_bytes().get(digits) {
        if !next.is_ascii_whitespace() && !matches!(next, b'/' | b'"' | b'\'') {
            return None;
        }
    }
    rest[..digits].parse().ok()
}

/// Compare complete Windows path components, not string prefixes (Foo vs Foobar).
pub fn target_in_install_location(target: &str, location: &str) -> bool {
    let target = target.replace('/', "\\").to_lowercase();
    let location = location.replace('/', "\\").to_lowercase();
    let location = location.trim_end_matches('\\');
    !location.is_empty() && (target == location || target.starts_with(&format!("{location}\\")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_library_aliases_have_the_same_key() {
        assert_eq!(
            windows_path_key(r"c:\program files (x86)\steam"),
            windows_path_key(r"C:\Program Files (x86)\Steam\")
        );
        assert_eq!(
            windows_path_key("D:/SteamLibrary/"),
            windows_path_key(r"d:\steamlibrary")
        );
        assert_ne!(windows_path_key(r"C:\Steam"), windows_path_key(r"D:\Steam"));
    }

    #[test]
    fn steam_ids_are_not_prefix_matches() {
        assert_eq!(
            steam_uninstall_id(r#""C:\Steam\steam.exe" steam://uninstall/1234"#),
            Some(1234)
        );
        assert_eq!(steam_uninstall_id("STEAM://UNINSTALL/123/"), Some(123));
        assert_eq!(steam_uninstall_id("steam://uninstall/123oops"), None);
        assert_eq!(steam_uninstall_id("steam://uninstall/"), None);
        assert_eq!(steam_uninstall_id("uninstall/123"), None);
    }

    #[test]
    fn shortcuts_require_a_complete_path_component() {
        assert!(target_in_install_location(
            r"C:\Apps\Foo\run.exe",
            r"c:\apps\foo\"
        ));
        assert!(target_in_install_location(
            "C:/Apps/Foo/run.exe",
            r"C:\Apps\Foo"
        ));
        assert!(!target_in_install_location(
            r"C:\Apps\Foobar\run.exe",
            r"C:\Apps\Foo"
        ));
        assert!(!target_in_install_location(r"C:\Apps\Foo\run.exe", ""));
    }
}
