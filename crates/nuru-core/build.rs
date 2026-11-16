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
}
