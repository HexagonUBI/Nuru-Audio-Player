use std::path::PathBuf;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_device: Option<String>,
}

pub struct SettingsStore {
    path: RwLock<Option<PathBuf>>,
    value: RwLock<Settings>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .unwrap_or_default();
        Self { path: RwLock::new(Some(path)), value: RwLock::new(value) }
    }

    pub fn get(&self) -> Settings {
        self.value.read().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) {
        {
            let mut v = self.value.write();
            f(&mut v);
        }
        self.save();
    }

    fn save(&self) {
        let Some(path) = self.path.read().clone() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&*self.value.read()) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::warn!("could not write settings: {e}");
                }
            }
            Err(e) => log::warn!("could not serialise settings: {e}"),
        }
    }
}
