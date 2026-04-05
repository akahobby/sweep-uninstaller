# Sweep Uninstall

Windows uninstall helper: browse **Win32** (registry) programs, **Steam** games, and **Microsoft Store** apps in one list, run the official uninstaller, then review optional leftovers (folders, shortcuts, registry scraps) before deleting them.

Built with Rust and [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/main/crates/eframe). Ships as a single `sweep-uninstall.exe` when built in release mode.

## Portable download (exe only)

GitHub **Releases** attach the 64-bit Windows **`.exe`** directly—download and run; no installer, no zip.

To produce the same file locally (e.g. for a manual upload):

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1
```

Output: `dist/sweep-uninstall-v<version>-windows-x64.exe`.

Publishing a release from git: tag with `v` plus the version (same as `Cargo.toml`), push the tag, and the `release-portable` workflow will attach that executable to the GitHub Release.

## Requirements

- **Windows 10 or 11** (x64)
- [Rust](https://rustup.rs/) toolchain and **MSVC** build tools (Visual Studio Build Tools with “Desktop development with C++” is enough)

## Build

```bash
cargo build --release
```

The binary is at `target/release/sweep-uninstall.exe`.

## Usage

Run the exe, pick an entry, use **Uninstall** to launch the publisher’s uninstaller (or Store removal / Steam flow where applicable). Afterward you can scan for leftovers and delete what you’re sure is safe.

**Caution:** leftover cleanup can remove real user data if you delete the wrong paths. Read each item before confirming.

## License

MIT — see [LICENSE](LICENSE).
