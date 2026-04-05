fn main() {
    println!("cargo:rerun-if-changed=assets/logo.png");
    println!("cargo:rerun-if-changed=src/logo_bitmap.rs");

    #[cfg(windows)]
    windows_icon();
}

#[cfg(windows)]
fn windows_icon() {
    #[path = "src/logo_bitmap.rs"]
    mod logo_bitmap;

    use std::fs::File;
    use std::io::BufWriter;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let logo_path = manifest_dir.join("assets/logo.png");
    let png = std::fs::read(&logo_path).unwrap_or_else(|e| panic!("read {}: {e}", logo_path.display()));

    let mut rgba = logo_bitmap::decode_logo_rgba(&png);
    logo_bitmap::purge_light_matte_for_exe_icon(&mut rgba);

    let out_ico = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app.ico");
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let flat = logo_bitmap::rasterize_logo_for_shell_ico(&rgba, size);
        let icon_img = ico::IconImage::from_rgba_data(size, size, flat.into_raw());
        // BMP DIB + mask: Explorer handles this more reliably than PNG-in-ICO for the shell.
        let entry = ico::IconDirEntry::encode_as_bmp(&icon_img)
            .unwrap_or_else(|e| panic!("ico encode {size}x{size}: {e}"));
        icon_dir.add_entry(entry);
    }

    let f = File::create(&out_ico).expect("create app.ico");
    icon_dir
        .write(BufWriter::new(f))
        .expect("write app.ico");

    let mut res = winres::WindowsResource::new();
    res.set_icon(out_ico.to_str().expect("utf8 path"));
    res.set("FileDescription", "Sweep Uninstall");
    res.set("ProductName", "Sweep Uninstall");
    res.set("OriginalFilename", "sweep-uninstall.exe");
    res.compile().unwrap_or_else(|e| panic!("winres: {e}"));
}
