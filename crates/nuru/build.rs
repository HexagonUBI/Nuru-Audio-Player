use std::path::Path;

fn main() {
    let version_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../version.json");
    println!("cargo:rerun-if-changed={}", version_file.display());
    let raw = std::fs::read_to_string(&version_file).unwrap_or_default();
    let version = raw
        .split("\"version\"")
        .nth(1)
        .and_then(|rest| rest.split('"').nth(1))
        .unwrap_or("0.0.0.0")
        .to_string();
    println!("cargo:rustc-env=NURU_VERSION={version}");
    let channel = raw
        .split("\"channel\"")
        .nth(1)
        .and_then(|rest| rest.split('"').nth(1))
        .unwrap_or("dev")
        .to_string();
    println!("cargo:rustc-env=NURU_CHANNEL={channel}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let parts: Vec<u64> = version.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        let packed = parts.iter().chain(std::iter::repeat(&0)).take(4).fold(0u64, |acc, p| (acc << 16) | (p & 0xffff));
        let mut res = winresource::WindowsResource::new();
        res.set_icon("ui/assets/icon.ico")
            .set("ProductName", "Nuru")
            .set("FileDescription", "Nuru")
            .set("CompanyName", "SimpleFox")
            .set("LegalCopyright", "Copyright (c) 2026 SimpleFox")
            .set("FileVersion", &version)
            .set("ProductVersion", &version)
            .set_version_info(winresource::VersionInfo::FILEVERSION, packed)
            .set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
        res.compile().expect("compiling the Windows resources");
    }

    slint_build::compile("ui/app.slint").expect("compiling the Slint UI");
}
