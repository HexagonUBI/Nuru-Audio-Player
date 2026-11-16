use std::path::{Path, PathBuf};

const APP_ID: &str = "app.nuru.player";

#[derive(Debug, Clone)]
pub struct Paths {
    pub data: PathBuf,
    pub local: PathBuf,
    pub legacy_storage: PathBuf,
}

impl Paths {
    pub fn resolve() -> Self {
        let legacy_override = std::env::var_os("NURU_LEGACY_STORAGE").map(PathBuf::from);

        if let Some(root) = std::env::var_os("NURU_DATA_DIR").map(PathBuf::from) {
            return Self {
                data: root.join("roaming"),
                local: root.join("local"),
                legacy_storage: legacy_override.unwrap_or_else(|| root.join("no-legacy-storage")),
            };
        }

        let roaming = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let local_root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| roaming.clone());
        let local = local_root.join(APP_ID);
        let legacy = local.join("EBWebView").join("Default").join("Local Storage").join("leveldb");
        Self { data: roaming.join(APP_ID), local, legacy_storage: legacy_override.unwrap_or(legacy) }
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.data.join(name)
    }

    pub fn logs(&self) -> PathBuf {
        self.local.join("logs")
    }

    pub fn user_packs(&self) -> PathBuf {
        self.data.join("packs")
    }

    pub fn exe_dir() -> Option<PathBuf> {
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf))
    }
}
