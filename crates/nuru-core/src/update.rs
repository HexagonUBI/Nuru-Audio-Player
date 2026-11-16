use std::cmp::Ordering;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::paths::Paths;
use crate::persist::{read_json, write_json};

pub const FEED_URL: &str = "https://simplefox.studio/apps/nuru/changelog.json";
pub const DOWNLOAD_URL: &str = "https://simplefox.studio/apps/nuru/download.php?file=";
const STATE_FILE: &str = "update-state.json";
const OLD_STATE_FILE: &str = "updates.json";
const HANDOFF_FILE: &str = "updating.json";
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);
const MAX_REMEMBERED: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    NotPublished,
    Offline,
    NoAsset,
    DownloadFailed,
    VerifyFailed,
    LaunchFailed,
}

impl UpdateError {
    pub fn message(self) -> &'static str {
        match self {
            Self::NotPublished => "No release has been published yet",
            Self::Offline => "Could not reach simplefox.studio",
            Self::NoAsset => "That release has no installer attached",
            Self::DownloadFailed => "The download did not finish",
            Self::VerifyFailed => "The downloaded file did not match the release",
            Self::LaunchFailed => "The installer could not be started",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Build {
    pub version: String,
    #[serde(default)]
    pub date: String,
    pub download: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Builds {
    pub windows: Option<Build>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Feed {
    #[serde(default)]
    pub releases: Vec<Release>,
    #[serde(default)]
    pub builds: Builds,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub build: Build,
    pub release: Option<Release>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct State {
    seen_version: Option<String>,
    last_check: Option<u64>,
    releases: Vec<Release>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OldState {
    seen_version: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Handoff {
    pub from: String,
    pub to: String,
    pub started: u64,
}

pub fn compare_versions(left: &str, right: &str) -> Ordering {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches(['v', 'V']).split('.').map(|p| p.trim().parse().unwrap_or(0)).collect()
    };
    let (a, b) = (parse(left), parse(right));
    for i in 0..a.len().max(b.len()) {
        let ord = a.get(i).copied().unwrap_or(0).cmp(&b.get(i).copied().unwrap_or(0));
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

fn feed_source() -> String {
    std::env::var("NURU_UPDATE_FEED").ok().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| FEED_URL.into())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(CHECK_TIMEOUT))
        .user_agent(format!("Nuru/{}", crate::version()))
        .build()
        .into()
}

fn fetch_feed() -> Result<Feed, UpdateError> {
    let source = feed_source();
    let raw = if source.starts_with("https://") || source.starts_with("http://") {
        let mut res = agent().get(&source).call().map_err(|e| match e {
            ureq::Error::StatusCode(404) => UpdateError::NotPublished,
            _ => UpdateError::Offline,
        })?;
        res.body_mut().read_to_string().map_err(|_| UpdateError::Offline)?
    } else {
        std::fs::read_to_string(&source).map_err(|_| UpdateError::NotPublished)?
    };
    serde_json::from_str::<Feed>(raw.trim_start_matches('\u{feff}')).map_err(|e| {
        log::warn!("the update feed at {source} is not valid: {e}");
        UpdateError::NotPublished
    })
}

fn read_state(paths: &Paths) -> State {
    let mut state = read_json::<State>(&paths.file(STATE_FILE)).unwrap_or_default();
    if state.seen_version.is_none() {
        state.seen_version = read_json::<OldState>(&paths.file(OLD_STATE_FILE)).and_then(|o| o.seen_version);
    }
    state
}

fn write_state(paths: &Paths, state: &State) {
    if let Err(e) = write_json(&paths.file(STATE_FILE), state) {
        log::warn!("could not save the update state: {e:#}");
    }
}

pub fn check(paths: &Paths, current: &str) -> Result<Option<Available>, UpdateError> {
    let result = fetch_feed();
    let mut state = read_state(paths);
    state.last_check = Some(now_secs());

    let feed = match result {
        Ok(feed) => feed,
        Err(e) => {
            write_state(paths, &state);
            log::info!("update check: {}", e.message());
            return Err(e);
        }
    };

    for release in &feed.releases {
        state.releases.retain(|r| r.version != release.version);
        state.releases.push(release.clone());
    }
    state.releases.sort_by(|a, b| compare_versions(&b.version, &a.version));
    state.releases.truncate(MAX_REMEMBERED);
    write_state(paths, &state);

    let Some(build) = feed.builds.windows else {
        log::info!("update check: the feed has no Windows build");
        return Err(UpdateError::NoAsset);
    };
    if compare_versions(&build.version, current) != Ordering::Greater {
        log::info!("up to date at {current}");
        return Ok(None);
    }
    log::info!("update available: {} (running {current})", build.version);
    let release = feed.releases.iter().find(|r| r.version == build.version).cloned();
    Ok(Some(Available { build, release }))
}

pub fn pending_changelog(paths: &Paths, current: &str) -> Option<Release> {
    let mut state = read_state(paths);
    match state.seen_version.as_deref() {
        None => {
            state.seen_version = Some(current.to_string());
            write_state(paths, &state);
            None
        }
        Some(seen) if compare_versions(current, seen) == Ordering::Greater => {
            state.releases.iter().find(|r| r.version == current).cloned()
        }
        _ => None,
    }
}

pub fn mark_seen(paths: &Paths, version: &str) {
    let mut state = read_state(paths);
    state.seen_version = Some(version.to_string());
    write_state(paths, &state);
}

pub fn release_for(paths: &Paths, version: &str) -> Option<Release> {
    let releases = read_state(paths).releases;
    releases.iter().find(|r| compare_versions(&r.version, version) == Ordering::Equal).cloned().or_else(|| releases.into_iter().next())
}

pub fn newest_known(paths: &Paths) -> Option<Release> {
    read_state(paths).releases.into_iter().next()
}

pub fn download(build: &Build, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, UpdateError> {
    if Path::new(&build.download).is_absolute() {
        let path = PathBuf::from(&build.download);
        verify(&path, build)?;
        return Ok(path);
    }
    if build.download.contains(['/', '\\']) || build.download.contains("..") {
        return Err(UpdateError::NoAsset);
    }

    let dir = std::env::temp_dir().join("nuru-update");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|_| UpdateError::DownloadFailed)?;
    let target = dir.join(&build.download);

    let url = format!("{DOWNLOAD_URL}{}", build.download);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .user_agent(format!("Nuru/{}", crate::version()))
        .build()
        .into();
    let res = agent.get(&url).call().map_err(|_| UpdateError::DownloadFailed)?;
    let mut reader = res.into_body().into_reader();
    let mut file = std::fs::File::create(&target).map_err(|_| UpdateError::DownloadFailed)?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut received = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|_| UpdateError::DownloadFailed)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|_| UpdateError::DownloadFailed)?;
        received += n as u64;
        progress(received, build.size);
    }
    drop(file);
    verify(&target, build)?;
    Ok(target)
}

fn verify(path: &Path, build: &Build) -> Result<(), UpdateError> {
    let bytes = std::fs::read(path).map_err(|_| UpdateError::DownloadFailed)?;
    if bytes.len() as u64 != build.size {
        log::error!("the installer is {} bytes, the feed says {}", bytes.len(), build.size);
        return Err(UpdateError::VerifyFailed);
    }
    let actual = hex::encode(Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(&build.sha256) {
        log::error!("the installer hash is {actual}, the feed says {}", build.sha256);
        return Err(UpdateError::VerifyFailed);
    }
    Ok(())
}

pub fn write_handoff(paths: &Paths, from: &str, to: &str) {
    let handoff = Handoff { from: from.into(), to: to.into(), started: now_secs() };
    if let Err(e) = write_json(&paths.file(HANDOFF_FILE), &handoff) {
        log::warn!("could not write the update handoff: {e:#}");
    }
}

pub fn take_handoff(paths: &Paths) -> Option<Handoff> {
    let path = paths.file(HANDOFF_FILE);
    let handoff = read_json::<Handoff>(&path);
    let _ = std::fs::remove_file(&path);
    handoff
}

pub fn install(installer: &Path) -> Result<(), UpdateError> {
    let exe = std::env::current_exe().unwrap_or_default();
    if let Some(helper) = stage_helper(&exe) {
        let mut cmd = std::process::Command::new(&helper);
        cmd.arg("--installer").arg(installer).arg("--exe").arg(&exe).arg("--pid").arg(std::process::id().to_string());
        match spawn_detached(&mut cmd) {
            Ok(_) => {
                log::info!("restart window started from {}", helper.display());
                return Ok(());
            }
            Err(e) => log::warn!("restart window would not start: {e}"),
        }
    }
    log::info!("falling back to the installer handling the restart");
    let mut cmd = std::process::Command::new(installer);
    cmd.arg("/S").arg("/R").arg("/UPDATE");
    spawn_detached(&mut cmd).map(|_| ()).map_err(|e| {
        log::error!("could not start {}: {e}", installer.display());
        UpdateError::LaunchFailed
    })
}

fn stage_helper(exe: &Path) -> Option<PathBuf> {
    let source = exe.parent()?.join("nuru-restart.exe");
    if !source.is_file() {
        return None;
    }
    let dir = std::env::temp_dir().join("nuru-restart");
    std::fs::create_dir_all(&dir).ok()?;
    let target = dir.join("nuru-restart.exe");
    std::fs::copy(&source, &target).ok()?;
    Some(target)
}

fn spawn_detached(cmd: &mut std::process::Command) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_part_versions_compare() {
        assert_eq!(compare_versions("0.0.2.0", "0.0.1.17"), Ordering::Greater);
        assert_eq!(compare_versions("v0.0.1.7", "0.0.1.7"), Ordering::Equal);
        assert_eq!(compare_versions("0.0.1", "0.0.1.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.0.1.6", "0.0.1.7"), Ordering::Less);
    }

    #[test]
    fn mayu_style_feed_parses() {
        let raw = r#"{"app":"nuru","showcase":[],"releases":[{"version":"0.0.2.0","date":"2026-11-20",
            "entries":[{"title":"Native app","text":"Uses far less memory."}]}],
            "builds":{"windows":{"version":"0.0.2.0","date":"2026-11-20","download":"Nuru_0.0.2.0_x64-setup.exe",
            "size":12,"sha256":"ab"}}}"#;
        let feed: Feed = serde_json::from_str(raw).unwrap();
        assert_eq!(feed.builds.windows.unwrap().download, "Nuru_0.0.2.0_x64-setup.exe");
        assert_eq!(feed.releases[0].entries[0].title, "Native app");
    }

    #[test]
    fn download_names_cannot_leave_the_folder() {
        let build = Build { version: "1".into(), date: String::new(), download: "../x.exe".into(), size: 0, sha256: String::new() };
        assert_eq!(download(&build, |_, _| {}), Err(UpdateError::NoAsset));
    }
}
