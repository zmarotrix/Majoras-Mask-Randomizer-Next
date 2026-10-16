fn main() {
    let mut attrs = tauri_build::Attributes::new();

    // tauri-winres escapes `'` in paths as `\'`, which rc.exe can't read, so
    // building from a folder like "Majora's Mask Randomizer" fails. Hand it a
    // copy of the icon from a path without apostrophes instead.
    #[cfg(windows)]
    {
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
        if icon.to_string_lossy().contains('\'') {
            let copy = std::env::temp_dir().join("mmr-app-icon.ico");
            std::fs::copy(&icon, &copy).expect("copy window icon");
            println!("cargo:rerun-if-changed=icons/icon.ico");
            attrs = attrs.windows_attributes(tauri_build::WindowsAttributes::new().window_icon_path(copy));
        }
    }

    tauri_build::try_build(attrs).expect("tauri build");
}
