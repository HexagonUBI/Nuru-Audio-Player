//! Nuru's audio path: local file → seamless loop → mixer → device.

mod engine;
mod gain;
mod source;

#[cfg(test)]
mod tests;

pub use engine::{AudioEngine, DeviceInfo, MAX_LAYERS};
