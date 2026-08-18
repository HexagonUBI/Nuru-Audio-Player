use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

pub const UPDATE_EVENT: &str = "nuru://update";

const DEFAULT_REPO: &str = "HexagonUBI/Nuru-Audio-Player";
const API_ROOT: &str = "https://api.github.com";
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);
const QUIT_DELAY: Duration = Duration::from_millis(900);

const ALLOWED_HOSTS: [&str; 3] = [
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub checked_iso: String,
    pub error: Option<String>,
    pub channel: String,
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

fn pick_installer(assets: &serde_json::Value) -> Option<(String, u64)> {
    let list = assets.as_array()?;
    let mut msi = None;
    let mut nsis = None;
    for a in list {
        let name = a.get("name").and_then(|v| v.as_str())?.to_lowercase();
        let url = a.get("browser_download_url").and_then(|v| v.as_str())?.to_string();
        let size = a.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
        if !allowed(&url) {
            continue;
        }
        if name.ends_with(".msi") {
            msi = Some((url, size));
        } else if name.ends_with("setup.exe") {
            nsis = Some((url, size));
        }
    }
    msi.or(nsis)
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
            .take(24_000)
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
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<ReleaseInfo>(&raw).ok()
}

pub async fn check(current_version: &str) -> UpdateStatus {
    let checked_iso = now_secs().to_string();

    if let Some(release) = local_feed() {
        let newer = compare_versions(&release.version, current_version) == std::cmp::Ordering::Greater;
        return UpdateStatus {
            current_version: current_version.to_string(),
            available: if newer { Some(release) } else { None },
            checked_iso,
            error: None,
            channel: "local-feed".into(),
        };
    }

    let url = format!("{API_ROOT}/repos/{}/releases/latest", repo_slug());
    let fetched = async {
        let c = client()?;
        let res = c
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .timeout(CHECK_TIMEOUT)
            .send()
            .await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow!("No release has been published yet"));
        }
        if res.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(anyhow!("GitHub is rate limiting, try again later"));
        }
        if !res.status().is_success() {
            return Err(anyhow!("GitHub returned {}", res.status()));
        }
        let body: serde_json::Value = res.json().await?;
        read_release(&body).ok_or_else(|| anyhow!("release payload was not usable"))
    }
    .await;

    match fetched {
        Ok(release) => {
            let newer =
                compare_versions(&release.version, current_version) == std::cmp::Ordering::Greater;
            UpdateStatus {
                current_version: current_version.to_string(),
                available: if newer { Some(release) } else { None },
                checked_iso,
                error: None,
                channel: channel().into(),
            }
        }
        Err(e) => UpdateStatus {
            current_version: current_version.to_string(),
            available: None,
            checked_iso,
            error: Some(format!("{e:#}")),
            channel: channel().into(),
        },
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn download_dir() -> PathBuf {
    std::env::temp_dir().join("nuru-update")
}

fn file_name_for(url: &str) -> String {
    let raw = url.rsplit('/').next().unwrap_or("update.msi");
    raw.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub async fn download<F>(release: &ReleaseInfo, mut on_progress: F) -> Result<PathBuf>
where
    F: FnMut(u64, u64) + Send,
{
    let url = release
        .download_url
        .as_ref()
        .ok_or_else(|| anyhow!("this release has no installer attached"))?;

    if !url.starts_with("https://") {
        let path = PathBuf::from(url);
        if path.is_file() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            on_progress(size, size);
            return Ok(path);
        }
        return Err(anyhow!("local update file {} is missing", path.display()));
    }

    if !allowed(url) {
        return Err(anyhow!("refusing to download from {url}"));
    }

    let dir = download_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).context("creating the download directory")?;
    let target = dir.join(file_name_for(url));

    let c = client()?;
    let res = c.get(url).timeout(DOWNLOAD_TIMEOUT).send().await?;
    if !res.status().is_success() {
        return Err(anyhow!("download returned {}", res.status()));
    }

    let total = res.content_length().or(release.download_size_bytes).unwrap_or(0);

    let mut file = std::fs::File::create(&target).context("creating the download file")?;
    let mut received: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        received += chunk.len() as u64;
        std::io::Write::write_all(&mut file, &chunk)?;
        on_progress(received, total);
    }
    drop(file);

    if let Some(expected) = release.download_size_bytes {
        if expected > 0 && received != expected {
            return Err(anyhow!("downloaded {received} bytes, the release says {expected}"));
        }
    }

    Ok(target)
}

pub fn install(path: &PathBuf) -> Result<()> {
    let name = path.to_string_lossy().to_lowercase();
    let mut cmd = if name.ends_with(".msi") {
        let mut c = std::process::Command::new("msiexec");
        c.arg("/i").arg(path).arg("/qb").arg("/norestart");
        c
    } else {
        let mut c = std::process::Command::new(path);
        c.arg("/S");
        c
    };
    cmd.spawn().context("starting the installer")?;
    Ok(())
}

pub fn quit_delay() -> Duration {
    QUIT_DELAY
}
