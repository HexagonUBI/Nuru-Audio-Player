
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Catalogue, Sound, SoundEntry, DEV_PLACEHOLDER_LICENCE};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    sha256: String,
    size: u64,
    mtime_ms: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VerifyCache {
    entries: HashMap<String, CacheEntry>,
}

fn fingerprint(path: &Path) -> Result<(u64, u64)> {
    let meta = fs::metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Ok((meta.len(), mtime))
}

pub struct Pack {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub builtin: bool,
    pub sounds: Vec<Sound>,
}

#[derive(Default)]
pub struct Library {
    packs: RwLock<Vec<Pack>>,
    index: RwLock<HashMap<String, usize>>,
    verified: RwLock<HashMap<String, bool>>,
    cache: RwLock<VerifyCache>,
    cache_path: RwLock<Option<PathBuf>>,
}

impl Library {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn use_cache(&self, path: PathBuf) {
        if let Ok(raw) = fs::read_to_string(&path) {
            match serde_json::from_str::<VerifyCache>(&raw) {
                Ok(cache) => *self.cache.write() = cache,
                Err(e) => log::warn!("ignoring unreadable verification cache: {e}"),
            }
        }
        *self.cache_path.write() = Some(path);
    }

    fn save_cache(&self) {
        let Some(path) = self.cache_path.read().clone() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let cache = self.cache.read();
        match serde_json::to_string(&*cache) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    log::warn!("could not write the verification cache: {e}");
                }
            }
            Err(e) => log::warn!("could not serialise the verification cache: {e}"),
        }
    }

    pub fn scan(&self, roots: &[(PathBuf, bool)]) -> Result<usize> {
        let mut packs = Vec::new();

        for (root, builtin) in roots {
            if !root.is_dir() {
                continue;
            }
            let entries = fs::read_dir(root)
                .with_context(|| format!("reading pack root {}", root.display()))?;
            for entry in entries.flatten() {
                let dir = entry.path();
                let manifest = dir.join("catalogue.json");
                if !manifest.is_file() {
                    continue;
                }
                match Self::read_pack(&dir, &manifest, *builtin) {
                    Ok(pack) => packs.push(pack),
                    Err(e) => log::warn!("skipping pack at {}: {e:#}", dir.display()),
                }
            }
        }

        let mut index = HashMap::new();
        for (i, pack) in packs.iter().enumerate() {
            for sound in &pack.sounds {
                index.insert(sound.id.clone(), i);
            }
        }

        let count = packs.len();
        *self.packs.write() = packs;
        *self.index.write() = index;
        self.verified.write().clear();
        Ok(count)
    }

    fn read_pack(dir: &Path, manifest: &Path, builtin: bool) -> Result<Pack> {
        let raw = fs::read_to_string(manifest)
            .with_context(|| format!("reading {}", manifest.display()))?;
        let cat: Catalogue = serde_json::from_str(&raw)
            .with_context(|| format!("parsing {}", manifest.display()))?;
        if cat.schema != 1 {
            return Err(anyhow!("catalogue schema {} is not supported", cat.schema));
        }
        Ok(Pack {
            id: cat.pack,
            name: cat.pack_name,
            root: dir.to_path_buf(),
            builtin,
            sounds: cat.sounds,
        })
    }

    pub fn entries(&self) -> Vec<SoundEntry> {
        let packs = self.packs.read();
        let index = self.index.read();
        let verified = self.verified.read();
        let mut out = Vec::new();
        for (i, pack) in packs.iter().enumerate() {
            for sound in &pack.sounds {
                if index.get(&sound.id) != Some(&i) {
                    continue;
                }
                let audio_path = pack.root.join(&sound.audio.file);
                let cover_path = {
                    let p = pack.root.join(&sound.cover);
                    p.is_file().then(|| p.to_string_lossy().into_owned())
                };
                out.push(SoundEntry {
                    verified: verified.get(&sound.id).copied().unwrap_or(false),
                    pack: pack.id.clone(),
                    pack_name: pack.name.clone(),
                    builtin: pack.builtin,
                    audio_path: audio_path.to_string_lossy().into_owned(),
                    cover_path,
                    sound: sound.clone(),
                });
            }
        }
        out
    }

    fn with_sound<T>(&self, id: &str, f: impl FnOnce(&Pack, &Sound) -> T) -> Option<T> {
        let packs = self.packs.read();
        let index = self.index.read();
        let i = *index.get(id)?;
        let pack = packs.get(i)?;
        let sound = pack.sounds.iter().find(|s| s.id == id)?;
        Some(f(pack, sound))
    }

    pub fn sound(&self, id: &str) -> Option<Sound> {
        self.with_sound(id, |_, s| s.clone())
    }

    pub fn ensure_local(&self, id: &str) -> Result<(Sound, PathBuf)> {
        let (sound, path) = self
            .with_sound(id, |pack, sound| (sound.clone(), pack.root.join(&sound.audio.file)))
            .ok_or_else(|| anyhow!("no sound with id '{id}'"))?;

        if !path.is_file() {
            return Err(anyhow!(
                "'{id}' is listed but its audio is not on disk at {} - the pack needs downloading again",
                path.display()
            ));
        }

        if self.verified.read().get(id).copied().unwrap_or(false) {
            return Ok((sound, path));
        }

        let (size, mtime_ms) = fingerprint(&path)?;

        if let Some(hit) = self.cache.read().entries.get(id) {
            if hit.size == size
                && hit.mtime_ms == mtime_ms
                && hit.sha256.eq_ignore_ascii_case(&sound.audio.sha256)
            {
                self.verified.write().insert(id.to_string(), true);
                return Ok((sound, path));
            }
        }

        if sound.audio.sha256.is_empty() {
            if sound.audio.bytes != 0 && size != sound.audio.bytes {
                return Err(anyhow!(
                    "'{id}' is {size} bytes, catalogue says {}",
                    sound.audio.bytes
                ));
            }
        } else {
            let actual = hash_file(&path)?;
            if !actual.eq_ignore_ascii_case(&sound.audio.sha256) {
                return Err(anyhow!(
                    "'{id}' failed its integrity check - expected {}, got {actual}",
                    sound.audio.sha256
                ));
            }
            self.cache.write().entries.insert(
                id.to_string(),
                CacheEntry { sha256: actual, size, mtime_ms },
            );
        }

        self.verified.write().insert(id.to_string(), true);
        Ok((sound, path))
    }

    pub fn unshippable(&self) -> Vec<String> {
        self.entries()
            .into_iter()
            .filter(|e| {
                !e.sound.provenance.shippable
                    || e.sound.provenance.licence == DEV_PLACEHOLDER_LICENCE
            })
            .map(|e| e.sound.id)
            .collect()
    }

    pub fn verify_all<F>(&self, mut progress: F) -> (usize, usize)
    where
        F: FnMut(usize, usize, &str),
    {
        let entries = self.entries();
        let total = entries.len();
        let mut ok = 0;
        let mut bad = 0;
        for (i, entry) in entries.into_iter().enumerate() {
            let id = entry.sound.id;
            progress(i, total, &entry.sound.name);
            match self.ensure_local(&id) {
                Ok(_) => ok += 1,
                Err(e) => {
                    bad += 1;
                    log::warn!("'{id}' will not be playable: {e:#}");
                }
            }
        }
        progress(total, total, "");
        self.save_cache();
        (ok, bad)
    }

    pub fn pack_summaries(&self) -> Vec<(String, String, usize)> {
        self.packs
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.name.clone(), p.sounds.len()))
            .collect()
    }
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}
