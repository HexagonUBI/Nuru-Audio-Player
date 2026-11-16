use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use nuru_core::model::SoundEntry;
use nuru_core::paths::Paths;

const TAG_PRIORITY: &[&str] = &[
    "fire", "transit", "weather", "water", "urban", "nature", "life", "people", "indoor", "noise",
];

const TAG_ORDER: &[&str] = &[
    "nature", "weather", "water", "fire", "life", "urban", "transit", "indoor", "people", "noise",
];

pub struct Group {
    pub name: String,
    pub sounds: Vec<SoundEntry>,
}

pub fn pack_roots(paths: &Paths) -> Vec<(PathBuf, bool)> {
    if let Some(list) = std::env::var_os("NURU_PACKS") {
        return std::env::split_paths(&list).map(|p| (p, true)).collect();
    }

    let mut roots = Vec::new();
    if let Some(dir) = Paths::exe_dir() {
        roots.push((dir.join("packs"), true));
    }
    roots.push((paths.user_packs(), false));
    if cfg!(debug_assertions) {
        if let Some(repo) = Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent) {
            roots.push((repo.join("resources").join("packs"), true));
        }
    }
    roots
}

pub fn grouped(entries: Vec<SoundEntry>) -> Vec<Group> {
    let mut groups: Vec<(String, bool, Vec<SoundEntry>)> = Vec::new();
    for entry in entries {
        let name = if entry.pack_name.is_empty() { "Other".to_string() } else { entry.pack_name.clone() };
        let builtin = entry.builtin;
        match groups.iter().position(|g| g.0 == name) {
            Some(i) => groups[i].2.push(entry),
            None => groups.push((name, builtin, vec![entry])),
        }
    }

    groups.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| compare_names(&a.0, &b.0)));
    groups
        .into_iter()
        .map(|(name, _, mut sounds)| {
            sounds.sort_by(|a, b| {
                rank(category(&a.sound.tags))
                    .cmp(&rank(category(&b.sound.tags)))
                    .then_with(|| compare_names(&a.sound.name, &b.sound.name))
            });
            Group { name, sounds }
        })
        .collect()
}

pub fn tags_by_use(entries: &[SoundEntry]) -> Vec<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for entry in entries {
        for tag in &entry.sound.tags {
            match counts.iter_mut().find(|(t, _)| t == tag) {
                Some((_, n)) => *n += 1,
                None => counts.push((tag.clone(), 1)),
            }
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts.into_iter().map(|(tag, _)| tag).collect()
}

fn category(tags: &[String]) -> &str {
    for tag in TAG_PRIORITY {
        if tags.iter().any(|t| t == tag) {
            return tag;
        }
    }
    tags.first().map(String::as_str).unwrap_or("other")
}

fn rank(tag: &str) -> usize {
    TAG_ORDER.iter().position(|t| *t == tag).unwrap_or(TAG_ORDER.len())
}

fn compare_names(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}
