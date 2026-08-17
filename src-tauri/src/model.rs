//! Wire types shared with the UI. Mirrors src/lib/types.ts — keep them in step.

use serde::{Deserialize, Serialize};

/// The sentinel licence that marks development-only material.
pub const DEV_PLACEHOLDER_LICENCE: &str = "UNLICENSED-DEV-PLACEHOLDER";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFileInfo {
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub codec: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopPoints {
    pub start_sample: u64,
    pub end_sample: Option<u64>,
    pub crossfade_ms: u32,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NookRole {
    pub channel: String,
    pub state: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub origin: String,
    pub licence: String,
    pub attribution: Option<String>,
    pub shippable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub accent: String,
    pub tags: Vec<String>,
    pub cover: String,
    pub audio: AudioFileInfo,
    #[serde(rename = "loop")]
    pub loop_points: LoopPoints,
    pub nook: NookRole,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    pub schema: u32,
    pub pack: String,
    pub pack_name: String,
    pub sounds: Vec<Sound>,
}

/// A sound plus everything the UI needs to render and play it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundEntry {
    #[serde(flatten)]
    pub sound: Sound,
    pub pack: String,
    /// Absolute path to the audio file on this machine.
    pub audio_path: String,
    /// Absolute path to the cover image on this machine, if it has one.
    pub cover_path: Option<String>,
    /// False until the file has been checked against its recorded hash.
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub device: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub max_layers: usize,
    pub active: Vec<String>,
    pub underruns: u32,
}
