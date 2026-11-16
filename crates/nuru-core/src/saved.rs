use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::schedule::SLOTS;

pub const STATE_FILE: &str = "state.json";
pub const PRESETS_FILE: &str = "presets.json";
pub const SCHEDULE_FILE: &str = "schedule.json";
pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TileSize {
    Compact,
    Small,
    #[default]
    Normal,
    Large,
}

impl TileSize {
    pub fn px(self) -> f32 {
        match self {
            TileSize::Compact => 148.0,
            TileSize::Small => 180.0,
            TileSize::Normal => 216.0,
            TileSize::Large => 272.0,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            TileSize::Compact => 12.0,
            TileSize::Small => 14.0,
            TileSize::Normal => 16.0,
            TileSize::Large => 20.0,
        }
    }

    pub fn gap(self) -> f32 {
        match self {
            TileSize::Compact => 10.0,
            TileSize::Small => 12.0,
            TileSize::Normal => 14.0,
            TileSize::Large => 16.0,
        }
    }

    pub fn label(self) -> f32 {
        match self {
            TileSize::Compact => 16.0,
            TileSize::Small => 19.0,
            TileSize::Normal => 22.0,
            TileSize::Large => 26.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLayer {
    pub sound_id: String,
    pub volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Saved {
    pub master_volume: f32,
    pub theme: String,
    pub layers: Vec<SavedLayer>,
    pub restore_on_launch: bool,
    pub tile_size: TileSize,
    pub sound_volumes: BTreeMap<String, f32>,
    pub scene_follows_volume: bool,
    pub schedule_enabled: bool,
    pub collapsed_groups: Vec<String>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            master_volume: 0.6,
            theme: "midnight".into(),
            layers: Vec::new(),
            restore_on_launch: true,
            tile_size: TileSize::Normal,
            sound_volumes: BTreeMap::new(),
            scene_follows_volume: true,
            schedule_enabled: false,
            collapsed_groups: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub layers: Vec<SavedLayer>,
    #[serde(default)]
    pub master_volume: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresetFile {
    #[serde(default)]
    pub items: Vec<Preset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleFile {
    #[serde(default)]
    pub slots: Vec<Option<String>>,
}

impl ScheduleFile {
    pub fn normalised(self) -> Vec<Option<String>> {
        let mut slots: Vec<Option<String>> =
            self.slots.into_iter().map(|s| s.filter(|v| !v.is_empty())).take(SLOTS).collect();
        slots.resize(SLOTS, None);
        slots
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_device: Option<String>,
    pub auto_update: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { output_device: None, auto_update: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_state_json_parses_and_drops_always_on_top() {
        let raw = r#"{"masterVolume":0.3,"theme":"ember","alwaysOnTop":true,"restoreOnLaunch":false,
            "tileSize":"small","soundVolumes":{"rain":0.21},"sceneFollowsVolume":true,
            "scheduleEnabled":true,"collapsedGroups":["Built-in"],
            "layers":[{"soundId":"rain","volume":0.21}]}"#;
        let s: Saved = serde_json::from_str(raw).unwrap();
        assert_eq!(s.master_volume, 0.3);
        assert_eq!(s.theme, "ember");
        assert_eq!(s.tile_size, TileSize::Small);
        assert!(!s.restore_on_launch);
        assert_eq!(s.layers, vec![SavedLayer { sound_id: "rain".into(), volume: 0.21 }]);
        assert!(!serde_json::to_string(&s).unwrap().contains("alwaysOnTop"));
    }

    #[test]
    fn missing_fields_take_old_defaults() {
        let s: Saved = serde_json::from_str("{}").unwrap();
        assert_eq!(s.master_volume, 0.6);
        assert_eq!(s.theme, "midnight");
        assert!(s.restore_on_launch);
        assert!(s.scene_follows_volume);
    }

    #[test]
    fn schedule_is_always_48_slots() {
        let f: ScheduleFile = serde_json::from_str(r#"{"slots":["a",null,""]}"#).unwrap();
        let slots = f.normalised();
        assert_eq!(slots.len(), SLOTS);
        assert_eq!(slots[0].as_deref(), Some("a"));
        assert_eq!(slots[2], None);
    }
}
