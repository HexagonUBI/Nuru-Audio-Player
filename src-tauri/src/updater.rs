use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

pub const UPDATE_EVENT: &str = "nuru://update";

const DEFAULT_REPO: &str = "HexagonUBI/Nuru-Audio-Player";
const API_ROOT: &str = "https://api.github.com";
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);
const QUIT_DELAY: Duration = Duration::from_millis(900);
const MAX_NOTES_CHARS: usize = 24_000;
const MAX_REMEMBERED: usize = 8;

const ALLOWED_HOSTS: [&str; 3] = [
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UpdateError {
    NotPublished,
    Offline,
    RateLimited,
    NoAsset,
    DownloadFailed,
    VerifyFailed,
    LaunchFailed,
}

impl UpdateError {
    pub fn message(self) -> &'static str {
        match self {
            Self::NotPublished => "No release has been published yet",
            Self::Offline => "Could not reach GitHub",
            Self::RateLimited => "GitHub is rate limiting, try again later",
            Self::NoAsset => "That release has no installer attached",
            Self::DownloadFailed => "The download did not finish",
            Self::VerifyFailed => "The downloaded file did not match the release",
            Self::LaunchFailed => "The installer could not be started",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub version: String,
    pub tag_name: String,
    pub title: String,
    pub notes: String,
    pub published_iso: Option<String>,
    pub html_url: String,
    pub download_url: Option<String>,
    pub download_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub current_version: String,
    pub available: Option<ReleaseInfo>,
    pub checked: u64,
    pub error: Option<UpdateError>,
    pub error_message: Option<String>,
    pub channel: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct UpdateState {
    seen_version: Option<String>,
    last_check: Option<u64>,
    releases: Vec<ReleaseInfo>,
}

pub fn repo_slug() -> String {
    std::env::var("NURU_UPDATE_REPO")
        .ok()
        .filter(|s| s.contains('/'))
        .unwrap_or_else(|| DEFAULT_REPO.to_string())
}

pub fn channel() -> &'static str {
    if cfg!(debug_assertions) {
        "development"
    } else {
        "installer"
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_version(value: &str) -> Vec<u64> {
    let cleaned = value.trim().trim_start_matches('v').trim_start_matches('V');
    let head = cleaned.split('-').next().unwrap_or(cleaned);
    head.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let a = parse_version(left);
    let b = parse_version(right);
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

fn allowed(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(u) => {
            u.scheme() == "https"
                && u.host_str().map(|h| ALLOWED_HOSTS.contains(&h)).unwrap_or(false)
        }
        Err(_) => false,
    }
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Nuru")
        .build()
        .context("building the http client")
}

fn state_path(app_data: &Path) -> PathBuf {
    app_data.join("updates.json")
}

fn read_state(app_data: &Path) -> UpdateState {
    std::fs::read_to_string(state_path(app_data))
        .ok()
        .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

fn write_state(app_data: &Path, state: &UpdateState) {
    let _ = std::fs::create_dir_all(app_data);
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(state_path(app_data), json);
    }
}

fn remember_release(app_data: &Path, release: &ReleaseInfo) {
    let mut state = read_state(app_data);
    state.releases.retain(|r| r.version != release.version);
    state.releases.insert(0, release.clone());
    state.releases.truncate(MAX_REMEMBERED);
    write_state(app_data, &state);
}

pub fn mark_seen(app_data: &Path, version: &str) {
    let mut state = read_state(app_data);
    state.seen_version = Some(version.to_string());
    write_state(app_data, &state);
}

pub fn pending_changelog(app_data: &Path, current: &str) -> Option<ReleaseInfo> {
    let state = read_state(app_data);
    match state.seen_version.as_deref() {
        None => {
            mark_seen(app_data, current);
            None
        }
        Some(seen) if seen == current => None,
        Some(_) => state.releases.iter().find(|r| r.version == current).cloned(),
    }
}

fn pick_installer(assets: &serde_json::Value) -> Option<(String, u64)> {
    let list = assets.as_array()?;
    for a in list {
        let name = a.get("name").and_then(|v| v.as_str())?.to_lowercase();
        let url = a.get("browser_download_url").and_then(|v| v.as_str())?.to_string();
        let size = a.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
        if allowed(&url) && name.ends_with("setup.exe") {
            return Some((url, size));
        }
    }
    None
}

fn read_release(raw: &serde_json::Value) -> Option<ReleaseInfo> {
    let tag_name = raw.get("tag_name")?.as_str()?.to_string();
    let version = tag_name.trim_start_matches('v').trim_start_matches('V').to_string();
    let asset = raw.get("assets").and_then(pick_installer);
    Some(ReleaseInfo {
        title: raw
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&tag_name)
            .to_string(),
        notes: raw
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(MAX_NOTES_CHARS)
            .collect(),
        published_iso: raw.get("published_at").and_then(|v| v.as_str()).map(String::from),
        html_url: raw
            .get("html_url")
            .and_then(|v| v.as_str())
            .filter(|s| s.starts_with("https://"))
            .map(String::from)
            .unwrap_or_else(|| format!("https://github.com/{}/releases", repo_slug())),
        download_url: asset.as_ref().map(|a| a.0.clone()),
        download_size_bytes: asset.as_ref().map(|a| a.1),
        version,
        tag_name,
    })
}

fn local_feed() -> Option<ReleaseInfo> {
    let path = std::env::var("NURU_UPDATE_FEED").ok()?;
    let raw = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<ReleaseInfo>(raw.trim_start_matches('\u{feff}')) {
        Ok(r) => {
            log::info!("using the local update feed at {path}");
            Some(r)
        }
        Err(e) => {
            log::warn!("NURU_UPDATE_FEED at {path} is not a usable release: {e}");
            None
        }
    }
}

async fn fetch(path: &str) -> Result<ReleaseInfo, UpdateError> {
    let url = format!("{API_ROOT}/repos/{}/{path}", repo_slug());
    let c = client().map_err(|_| UpdateError::Offline)?;
    let res = c
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .timeout(CHECK_TIMEOUT)
        .send()
        .await
        .map_err(|_| UpdateError::Offline)?;

    match res.status().as_u16() {
        200 => {}
        404 => return Err(UpdateError::NotPublished),
        403 | 429 => return Err(UpdateError::RateLimited),
        _ => return Err(UpdateError::Offline),
    }

    let body: serde_json::Value = res.json().await.map_err(|_| UpdateError::Offline)?;
    read_release(&body).ok_or(UpdateError::NotPublished)
}

pub async fn check(app_data: &Path, current_version: &str) -> UpdateStatus {
    let checked = now_secs();

    let (release, error) = match local_feed() {
        Some(r) => (Some(r), None),
        None => match fetch("releases/latest").await {
            Ok(r) => (Some(r), None),
            Err(e) => (None, Some(e)),
        },
    };

    let mut state = read_state(app_data);
    state.last_check = Some(checked);
    write_state(app_data, &state);

    let Some(release) = release else {
        let e = error.unwrap_or(UpdateError::Offline);
        return UpdateStatus {
            current_version: current_version.to_string(),
            available: None,
            checked,
            error: Some(e),
            error_message: Some(e.message().to_string()),
            channel: channel().into(),
        };
    };

    remember_release(app_data, &release);

    let newer = compare_versions(&release.version, current_version) == std::cmp::Ordering::Greater;
    UpdateStatus {
        current_version: current_version.to_string(),
        available: if newer { Some(release) } else { None },
        checked,
        error: None,
        error_message: None,
        channel: channel().into(),
    }
}

pub async fn notes_for(app_data: &Path, version: &str) -> Option<ReleaseInfo> {
    let state = read_state(app_data);
    if let Some(found) = state.releases.iter().find(|r| r.version == version) {
        return Some(found.clone());
    }
    for tag in [format!("v{version}"), version.to_string()] {
        if let Ok(release) = fetch(&format!("releases/tags/{tag}")).await {
            remember_release(app_data, &release);
            return Some(release);
        }
    }
    None
}

fn download_dir() -> PathBuf {
    std::env::temp_dir().join("nuru-update")
}

fn file_name_for(url: &str) -> String {
    let raw = url.rsplit('/').next().unwrap_or("nuru-setup.exe");
    let safe: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if safe.to_lowercase().ends_with(".exe") {
        safe
    } else {
        format!("{safe}.exe")
    }
}

pub async fn download<F>(release: &ReleaseInfo, mut on_progress: F) -> Result<PathBuf, UpdateError>
where
    F: FnMut(u64, u64) + Send,
{
    let url = release.download_url.as_ref().ok_or(UpdateError::NoAsset)?;

    if !url.starts_with("https://") {
        let path = PathBuf::from(url);
        if path.is_file() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            on_progress(size, size);
            return Ok(path);
        }
        return Err(UpdateError::NoAsset);
    }

    if !allowed(url) {
        return Err(UpdateError::NoAsset);
    }

    let dir = download_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|_| UpdateError::DownloadFailed)?;
    let target = dir.join(file_name_for(url));

    let c = client().map_err(|_| UpdateError::Offline)?;
    let res = c
        .get(url)
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .await
        .map_err(|_| UpdateError::DownloadFailed)?;
    if !res.status().is_success() {
        return Err(UpdateError::DownloadFailed);
    }

    let total = res.content_length().or(release.download_size_bytes).unwrap_or(0);
    let mut file = std::fs::File::create(&target).map_err(|_| UpdateError::DownloadFailed)?;
    let mut received: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| UpdateError::DownloadFailed)?;
        received += chunk.len() as u64;
        std::io::Write::write_all(&mut file, &chunk).map_err(|_| UpdateError::DownloadFailed)?;
        on_progress(received, total);
    }
    drop(file);

    if let Some(expected) = release.download_size_bytes {
        if expected > 0 && received != expected {
            log::error!("downloaded {received} bytes, the release says {expected}");
            return Err(UpdateError::VerifyFailed);
        }
    }

    Ok(target)
}

pub fn install(path: &Path) -> Result<(), UpdateError> {
    let mut cmd = std::process::Command::new(path);
    cmd.arg("/S").arg("/R").arg("/UPDATE");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("could not start {}: {e}", path.display());
            Err(UpdateError::LaunchFailed)
        }
    }
}

pub fn quit_delay() -> Duration {
    QUIT_DELAY
}
