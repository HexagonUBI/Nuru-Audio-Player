// Keeps the console window from appearing behind the app on Windows release
// builds, while leaving it available for `cargo run` during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    nuru_lib::run()
}
