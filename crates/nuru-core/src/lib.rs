pub mod audio;
pub mod cloud;
pub mod core;
pub mod discord;
pub mod legacy;
pub mod library;
pub mod model;
pub mod paths;
pub mod persist;
pub mod presence;
pub mod saved;
pub mod scene;
pub mod schedule;
pub mod update;

pub fn version() -> &'static str {
    env!("NURU_VERSION")
}
