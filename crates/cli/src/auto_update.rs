//! Auto-update infrastructure for checking and applying Aineer updates.
//!
//! Queries the GitHub Releases API for newer versions, persists check state
//! to disk, respects release channels and dismissed versions, and renders
//! update notifications in the terminal.

use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::style;
use crate::VERSION;

const GITHUB_REPO: &str = "andeya/aineer";
const BINARY_NAME: &str = "aineer";

fn user_agent() -> String {
    format!("{BINARY_NAME}/{VERSION}")
}

/// Truncate a string at a character boundary, appending "…" if truncated.
fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut chars = s.char_indices();
    if let Some((byte_idx, _)) = chars.nth(max_chars) {
        format!("{}…", &s[..byte_idx])
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Version information from the GitHub Releases API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    /// GitHub release page URL.
    pub download_url: String,
    /// Direct download URL for the platform-specific binary archive (if available).
    pub asset_download_url: Option<String>,
    pub release_notes: String,
    pub published_at: String,
    pub prerelease: bool,
}

/// Result of an update check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckResult {
    UpToDate {
        current: String,
    },
    UpdateAvailable {
        current: String,
        latest: ReleaseInfo,
    },
    Dismissed {
        current: String,
        latest: ReleaseInfo,
    },
    CheckFailed {
        reason: String,
    },
}

/// Which release channel to track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Nightly,
}

/// Configuration for the auto-update system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub enabled: bool,
    pub check_interval_hours: u64,
    pub release_channel: ReleaseChannel,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_hours: 24,
            release_channel: ReleaseChannel::Stable,
        }
    }
}

/// Persisted state for update checking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateCheckState {
    pub last_checked_epoch_secs: Option<u64>,
    pub dismissed_version: Option<String>,
    pub last_seen_version: Option<String>,
}

impl UpdateCheckState {
    fn last_checked(&self) -> Option<SystemTime> {
        self.last_checked_epoch_secs
            .map(|s| SystemTime::UNIX_EPOCH + Duration::from_secs(s))
    }

    /// Whether enough time has passed since the last check.
    #[must_use]
    pub fn should_check(&self, interval_hours: u64) -> bool {
        let Some(last) = self.last_checked() else {
            return true;
        };
        let elapsed = SystemTime::now().duration_since(last).unwrap_or_default();
        elapsed.as_secs() >= interval_hours * 3600
    }

    /// Record that a check just happened.
    pub fn mark_checked(&mut self) {
        self.last_checked_epoch_secs = Some(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Dismiss a specific version so the user isn't nagged again.
    pub fn dismiss_version(&mut self, version: &str) {
        self.dismissed_version = Some(version.to_string());
    }

    /// Check whether a version has been dismissed.
    #[must_use]
    pub fn is_dismissed(&self, version: &str) -> bool {
        self.dismissed_version.as_deref() == Some(version)
    }
}

// ---------------------------------------------------------------------------
// State persistence
// ---------------------------------------------------------------------------

fn state_file_path() -> PathBuf {
    let home = std::env::var_os("AINEER_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs_home().join(".aineer"));
    home.join("update-state.json")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Load persisted update check state from disk.
pub fn load_state() -> UpdateCheckState {
    load_state_from(&state_file_path())
}

fn load_state_from(path: &Path) -> UpdateCheckState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Save update check state to disk.
pub fn save_state(state: &UpdateCheckState) {
    save_state_to(state, &state_file_path());
}

fn save_state_to(state: &UpdateCheckState, path: &Path) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, json);
    }
}

// ---------------------------------------------------------------------------
// Version comparison (simple semver)
// ---------------------------------------------------------------------------

/// Parsed semver: (major, minor, patch, is_prerelease).
/// Pre-release versions sort lower than the same stable version,
/// e.g. 1.0.0-beta.1 < 1.0.0.
type Semver = (u32, u32, u32, bool);

fn parse_semver(v: &str) -> Option<Semver> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let (numeric, pre) = match v.split_once('-') {
        Some((n, _)) => (n, true),
        None => (v, false),
    };
    let parts: Vec<&str> = numeric.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
        pre,
    ))
}

/// Comparable key: pre-release sorts before stable at the same version.
fn semver_sort_key(v: &Semver) -> (u32, u32, u32, u8) {
    (v.0, v.1, v.2, if v.3 { 0 } else { 1 })
}

fn is_newer(current: &str, candidate: &str) -> bool {
    match (parse_semver(current), parse_semver(candidate)) {
        (Some(c), Some(n)) => semver_sort_key(&n) > semver_sort_key(&c),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// GitHub Releases API
// ---------------------------------------------------------------------------

/// Constructed at first use; avoids duplicating repo name.
fn github_releases_api() -> String {
    format!("https://api.github.com/repos/{GITHUB_REPO}/releases")
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    prerelease: bool,
    draft: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

impl GithubRelease {
    fn into_release_info(self) -> ReleaseInfo {
        let version = self
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&self.tag_name)
            .to_string();

        let url = if platform_target() == "unknown-platform" {
            None
        } else {
            let expected = asset_filename(&version);
            // Prefer the verified URL from GitHub API (asset actually exists).
            // Fall back to locally constructed URL if the API didn't return assets
            // (e.g. draft release, truncated response, or CI naming change).
            let verified = self
                .assets
                .iter()
                .find(|a| a.name == expected)
                .map(|a| a.browser_download_url.clone());
            Some(verified.unwrap_or_else(|| asset_download_url(&version)))
        };

        ReleaseInfo {
            version,
            download_url: self.html_url,
            asset_download_url: url,
            release_notes: self.body.unwrap_or_default(),
            published_at: self.published_at.unwrap_or_default(),
            prerelease: self.prerelease,
        }
    }
}

/// Check for updates by querying GitHub Releases.
pub fn check_for_updates(config: &UpdateConfig) -> UpdateCheckResult {
    check_for_updates_with_state(config, &mut load_state())
}

/// Check for updates, also reading/writing the persisted state.
pub fn check_for_updates_with_state(
    config: &UpdateConfig,
    state: &mut UpdateCheckState,
) -> UpdateCheckResult {
    let current = VERSION.to_string();

    let releases = match fetch_releases() {
        Ok(r) => r,
        Err(e) => {
            state.mark_checked();
            save_state(state);
            return UpdateCheckResult::CheckFailed {
                reason: e.to_string(),
            };
        }
    };

    state.mark_checked();

    let best = releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter(|r| matches_channel(r, config.release_channel))
        .map(GithubRelease::into_release_info)
        .filter(|r| is_newer(&current, &r.version))
        .max_by(|a, b| {
            let ka = parse_semver(&a.version).map(|v| semver_sort_key(&v));
            let kb = parse_semver(&b.version).map(|v| semver_sort_key(&v));
            ka.cmp(&kb)
        });

    match best {
        Some(latest) if state.is_dismissed(&latest.version) => {
            state.last_seen_version = Some(latest.version.clone());
            save_state(state);
            UpdateCheckResult::Dismissed { current, latest }
        }
        Some(latest) => {
            state.last_seen_version = Some(latest.version.clone());
            save_state(state);
            UpdateCheckResult::UpdateAvailable { current, latest }
        }
        None => {
            save_state(state);
            UpdateCheckResult::UpToDate { current }
        }
    }
}

fn matches_channel(release: &GithubRelease, channel: ReleaseChannel) -> bool {
    match channel {
        ReleaseChannel::Stable => !release.prerelease,
        ReleaseChannel::Beta => true, // beta sees both stable and prerelease
        ReleaseChannel::Nightly => true, // nightly sees everything
    }
}

fn fetch_releases() -> Result<Vec<GithubRelease>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(user_agent())
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .get(github_releases_api())
        .query(&[("per_page", "20")])
        .send()
        .map_err(|e| format!("network error: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }

    resp.json::<Vec<GithubRelease>>()
        .map_err(|e| format!("JSON parse error: {e}"))
}

// ---------------------------------------------------------------------------
// Terminal notification
// ---------------------------------------------------------------------------

/// Render a colored update notification for the terminal.
pub fn render_update_notification(current: &str, latest: &ReleaseInfo) -> String {
    let p = style::Palette::for_stderr();
    let mut out = String::new();
    let _ = write!(
        out,
        "\n  {b}📦 Update available:{r} {d}{current}{r} → {g}{ver}{r}\n",
        b = p.bold_white,
        r = p.r,
        d = p.dim,
        g = p.bold_green,
        ver = latest.version,
    );
    let _ = writeln!(
        out,
        "  {d}Run:{r} {c}cargo install aineer-cli{r}  {d}or{r}  {c}brew upgrade aineer{r}",
        d = p.dim,
        r = p.r,
        c = p.cyan_fg,
    );
    if let Some(first_line) = latest
        .release_notes
        .lines()
        .next()
        .filter(|l| !l.is_empty())
    {
        let truncated = truncate_str(first_line, 79);
        let _ = writeln!(out, "  {d}{truncated}{r}", d = p.dim, r = p.r);
    }
    let _ = writeln!(
        out,
        "  {d}Update: /update apply   Dismiss: /update dismiss   Release: {url}{r}",
        d = p.dim,
        r = p.r,
        url = latest.download_url,
    );
    out
}

/// Run a background update check and print notification to stderr if needed.
/// Intended to be called once at CLI startup. Non-blocking: spawns a thread.
pub fn background_update_check(config: UpdateConfig) -> Option<std::thread::JoinHandle<()>> {
    if !config.enabled {
        return None;
    }
    let mut state = load_state();
    if !state.should_check(config.check_interval_hours) {
        return None;
    }

    Some(std::thread::spawn(move || {
        let result = check_for_updates_with_state(&config, &mut state);
        if let UpdateCheckResult::UpdateAvailable {
            ref current,
            ref latest,
        } = result
        {
            let notification = render_update_notification(current, latest);
            let _ = std::io::stderr().write_all(notification.as_bytes());
        }
    }))
}

// ---------------------------------------------------------------------------
// Self-update: download and replace binary
// ---------------------------------------------------------------------------

/// Rust target triple for the current platform, matching CI build matrix.
fn platform_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => "unknown-platform",
    }
}

/// Archive extension used by CI for the current platform.
fn platform_archive_ext() -> &'static str {
    if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    }
}

/// Build the release asset filename, matching the CI naming convention:
/// `aineer-v{version}-{target}.{ext}`
fn asset_filename(version: &str) -> String {
    format!(
        "aineer-v{version}-{target}.{ext}",
        target = platform_target(),
        ext = platform_archive_ext(),
    )
}

/// Build the full download URL for the release asset:
/// `https://github.com/{repo}/releases/download/v{version}/{asset}`
fn asset_download_url(version: &str) -> String {
    format!(
        "https://github.com/{GITHUB_REPO}/releases/download/v{version}/{asset}",
        asset = asset_filename(version),
    )
}

/// Download the release asset and replace the current binary.
/// Returns Ok(path) on success with the path of the new binary.
pub fn self_update(release: &ReleaseInfo) -> Result<PathBuf, String> {
    let asset_url = release.asset_download_url.as_deref().ok_or_else(|| {
        format!(
            "No prebuilt binary for this platform ({}/{}). \
             Install manually: cargo install aineer-cli",
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })?;

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot determine current binary path: {e}"))?;

    eprintln!("  Downloading aineer {}…", release.version);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent(user_agent())
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .get(asset_url)
        .send()
        .map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download returned HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| format!("Read error: {e}"))?;

    let extracted = if asset_url.ends_with(".tar.gz") || asset_url.ends_with(".tgz") {
        extract_tar_gz(&bytes)?
    } else if asset_url.ends_with(".zip") {
        extract_zip(&bytes)?
    } else {
        bytes.to_vec()
    };

    if extracted.len() < 1024 {
        return Err("Downloaded binary is suspiciously small; aborting".to_string());
    }

    // Atomic replacement: write to a temp file next to the current binary,
    // then rename.
    let dir = current_exe
        .parent()
        .ok_or("Cannot determine binary directory")?;
    let tmp_path = dir.join(".aineer-update-tmp");
    std::fs::write(&tmp_path, &extracted)
        .map_err(|e| format!("Failed to write temp binary: {e}"))?;

    // On Unix, set executable permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755));
    }

    // Back up the old binary (best-effort).
    let backup_path = dir.join(".aineer-old");
    let _ = std::fs::rename(&current_exe, &backup_path);

    // Move new binary into place.
    std::fs::rename(&tmp_path, &current_exe).map_err(|e| {
        // Try to restore backup.
        let _ = std::fs::rename(&backup_path, &current_exe);
        format!("Failed to replace binary: {e}")
    })?;

    // Clean up backup.
    let _ = std::fs::remove_file(&backup_path);

    Ok(current_exe)
}

fn is_target_binary(name: &str) -> bool {
    name == BINARY_NAME || name == format!("{BINARY_NAME}.exe")
}

fn extract_tar_gz(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let gz = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries().map_err(|e| format!("tar error: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry error: {e}"))?;
        let path = entry.path().map_err(|e| format!("tar path error: {e}"))?;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_target_binary(name) {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("tar read error: {e}"))?;
            return Ok(buf);
        }
    }
    Err(format!("Binary '{BINARY_NAME}' not found in archive"))
}

fn extract_zip(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("zip error: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("zip entry error: {e}"))?;
        let name = file
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
        if name.as_deref().is_some_and(is_target_binary) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("zip read error: {e}"))?;
            return Ok(buf);
        }
    }
    Err(format!("Binary '{BINARY_NAME}' not found in archive"))
}

// ---------------------------------------------------------------------------
// Slash command handler: /update [dismiss|check|status|apply]
// ---------------------------------------------------------------------------

/// Handle the `/update` slash command. Returns a user-facing message.
pub fn handle_update_command(action: Option<&str>) -> String {
    match action {
        Some("dismiss") => {
            let mut state = load_state();
            if let Some(ver) = state.last_seen_version.clone() {
                state.dismiss_version(&ver);
                save_state(&state);
                format!("Version {ver} dismissed. You won't be notified about it again.")
            } else {
                "No pending update to dismiss.".to_string()
            }
        }
        Some("check") | None => {
            let config = UpdateConfig::default();
            let mut state = load_state();
            let result = check_for_updates_with_state(&config, &mut state);
            match result {
                UpdateCheckResult::UpToDate { current } => {
                    format!("✓ Aineer {current} is up to date.")
                }
                UpdateCheckResult::UpdateAvailable { current, latest } => {
                    render_update_notification(&current, &latest)
                }
                UpdateCheckResult::Dismissed { current, latest } => {
                    format!(
                        "Update {version} available (current: {current}), but dismissed.\n\
                         Run `/update check` after un-dismissing to see it again.",
                        version = latest.version,
                    )
                }
                UpdateCheckResult::CheckFailed { reason } => {
                    format!("✗ Update check failed: {reason}")
                }
            }
        }
        Some("status") => {
            let state = load_state();
            let mut lines = vec![format!("Current version: {VERSION}")];
            if let Some(epoch) = state.last_checked_epoch_secs {
                let ago = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH + Duration::from_secs(epoch))
                    .unwrap_or_default();
                let mins = ago.as_secs() / 60;
                if mins < 60 {
                    lines.push(format!("Last checked: {mins} minute(s) ago"));
                } else {
                    lines.push(format!("Last checked: {} hour(s) ago", mins / 60));
                }
            } else {
                lines.push("Last checked: never".to_string());
            }
            if let Some(ref ver) = state.last_seen_version {
                lines.push(format!("Latest seen: {ver}"));
            }
            if let Some(ref ver) = state.dismissed_version {
                lines.push(format!("Dismissed: {ver}"));
            }
            lines.join("\n")
        }
        Some("apply") => {
            let config = UpdateConfig::default();
            let mut state = load_state();
            let result = check_for_updates_with_state(&config, &mut state);
            match result {
                UpdateCheckResult::UpdateAvailable { current, latest }
                | UpdateCheckResult::Dismissed { current, latest } => {
                    do_self_update(&current, &latest)
                }
                UpdateCheckResult::UpToDate { current } => {
                    format!("✓ Aineer {current} is already up to date.")
                }
                UpdateCheckResult::CheckFailed { reason } => {
                    format!("✗ Update check failed: {reason}")
                }
            }
        }
        Some(other) => {
            format!(
                "Unknown /update action: {other}\n\
                 Usage: /update [check|apply|dismiss|status]"
            )
        }
    }
}

fn do_self_update(current: &str, latest: &ReleaseInfo) -> String {
    let p = style::Palette::for_stderr();
    eprintln!(
        "\n  {b}Updating:{r} {d}{current}{r} → {g}{v}{r}\n",
        b = p.bold_white,
        r = p.r,
        d = p.dim,
        g = p.bold_green,
        v = latest.version,
    );
    match self_update(latest) {
        Ok(path) => {
            format!(
                "✓ Aineer updated to {}.\n  Binary: {}\n  Restart your shell to use the new version.",
                latest.version,
                path.display(),
            )
        }
        Err(e) => {
            format!(
                "✗ Self-update failed: {e}\n\n  Manual install:\n    cargo install aineer-cli\n    brew upgrade aineer"
            )
        }
    }
}

// ---------------------------------------------------------------------------
// CLI subcommand: `aineer update`
// ---------------------------------------------------------------------------

/// Entry point for `aineer update` CLI subcommand.
pub fn run_update_command() {
    let config = UpdateConfig::default();
    let result = check_for_updates(&config);

    match result {
        UpdateCheckResult::UpToDate { current } => {
            println!("✓ Aineer {current} is up to date.");
        }
        UpdateCheckResult::UpdateAvailable { current, latest }
        | UpdateCheckResult::Dismissed { current, latest } => {
            println!("Update available: {current} → {}\n", latest.version,);
            if !latest.release_notes.is_empty() {
                let preview: String = latest
                    .release_notes
                    .lines()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("Release notes:\n{preview}\n");
            }
            if latest.asset_download_url.is_some() {
                eprintln!("Downloading and installing…");
                let msg = do_self_update(&current, &latest);
                println!("{msg}");
            } else {
                println!(
                    "No prebuilt binary for {}/{}.\n\nManual install:\n  cargo install aineer-cli\n  brew upgrade aineer\n\nRelease: {}",
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                    latest.download_url,
                );
            }
        }
        UpdateCheckResult::CheckFailed { reason } => {
            eprintln!("✗ Update check failed: {reason}");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_release_info(version: &str) -> ReleaseInfo {
        ReleaseInfo {
            version: version.into(),
            download_url: format!("https://github.com/andeya/aineer/releases/tag/v{version}"),
            asset_download_url: None,
            release_notes: String::new(),
            published_at: "2026-04-08".into(),
            prerelease: false,
        }
    }

    #[test]
    fn update_config_defaults() {
        let config = UpdateConfig::default();
        assert!(config.enabled);
        assert_eq!(config.check_interval_hours, 24);
        assert_eq!(config.release_channel, ReleaseChannel::Stable);
    }

    #[test]
    fn should_check_when_never_checked() {
        let state = UpdateCheckState::default();
        assert!(state.should_check(24));
    }

    #[test]
    fn should_not_check_when_recently_checked() {
        let epoch_now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let state = UpdateCheckState {
            last_checked_epoch_secs: Some(epoch_now),
            dismissed_version: None,
            last_seen_version: None,
        };
        assert!(!state.should_check(24));
    }

    #[test]
    fn check_result_all_variants_used() {
        let up = UpdateCheckResult::UpToDate {
            current: "0.6.9".into(),
        };
        assert!(matches!(up, UpdateCheckResult::UpToDate { .. }));

        let info = test_release_info("0.7.0");
        let avail = UpdateCheckResult::UpdateAvailable {
            current: "0.6.9".into(),
            latest: info.clone(),
        };
        assert!(matches!(avail, UpdateCheckResult::UpdateAvailable { .. }));

        let dismissed = UpdateCheckResult::Dismissed {
            current: "0.6.9".into(),
            latest: info,
        };
        assert!(matches!(dismissed, UpdateCheckResult::Dismissed { .. }));

        let failed = UpdateCheckResult::CheckFailed {
            reason: "timeout".into(),
        };
        assert!(matches!(failed, UpdateCheckResult::CheckFailed { .. }));
    }

    #[test]
    fn semver_parsing() {
        assert_eq!(parse_semver("0.6.9"), Some((0, 6, 9, false)));
        assert_eq!(parse_semver("v1.2.3"), Some((1, 2, 3, false)));
        assert_eq!(parse_semver("1.0.0-beta.1"), Some((1, 0, 0, true)));
        assert_eq!(parse_semver("bad"), None);
    }

    #[test]
    fn version_comparison() {
        assert!(is_newer("0.6.9", "0.7.0"));
        assert!(is_newer("0.6.9", "1.0.0"));
        assert!(!is_newer("0.6.9", "0.6.9"));
        assert!(!is_newer("0.7.0", "0.6.9"));
        assert!(is_newer("0.6.9", "0.6.10"));
    }

    #[test]
    fn prerelease_sorts_before_stable() {
        assert!(
            is_newer("1.0.0-beta.1", "1.0.0"),
            "stable > pre-release at same version"
        );
        assert!(
            !is_newer("1.0.0", "1.0.0-beta.1"),
            "pre-release < stable at same version"
        );
        assert!(
            is_newer("0.9.0", "1.0.0-beta.1"),
            "pre-release of higher version > lower stable"
        );
    }

    #[test]
    fn truncate_str_handles_ascii_and_unicode() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 5), "hello…");
        assert_eq!(truncate_str("你好世界测试", 3), "你好世…");
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn dismissed_version_tracking() {
        let mut state = UpdateCheckState::default();
        assert!(!state.is_dismissed("1.0.0"));
        state.dismiss_version("1.0.0");
        assert!(state.is_dismissed("1.0.0"));
        assert!(!state.is_dismissed("1.0.1"));
    }

    #[test]
    fn state_persistence_roundtrip() {
        let dir = std::env::temp_dir().join("aineer-test-update-state");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.json");

        let mut state = UpdateCheckState::default();
        state.mark_checked();
        state.dismiss_version("0.8.0");
        state.last_seen_version = Some("0.8.0".to_string());
        save_state_to(&state, &path);

        let loaded = load_state_from(&path);
        assert_eq!(loaded.dismissed_version, Some("0.8.0".to_string()));
        assert_eq!(loaded.last_seen_version, Some("0.8.0".to_string()));
        assert!(loaded.last_checked_epoch_secs.is_some());
        assert!(!loaded.should_check(24));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_channel_filtering() {
        let stable = GithubRelease {
            tag_name: "v0.7.0".into(),
            html_url: String::new(),
            body: None,
            published_at: None,
            prerelease: false,
            draft: false,
            assets: vec![],
        };
        let beta = GithubRelease {
            tag_name: "v0.8.0-beta.1".into(),
            html_url: String::new(),
            body: None,
            published_at: None,
            prerelease: true,
            draft: false,
            assets: vec![],
        };

        assert!(matches_channel(&stable, ReleaseChannel::Stable));
        assert!(!matches_channel(&beta, ReleaseChannel::Stable));
        assert!(matches_channel(&stable, ReleaseChannel::Beta));
        assert!(matches_channel(&beta, ReleaseChannel::Beta));
        assert!(matches_channel(&beta, ReleaseChannel::Nightly));
    }

    #[test]
    fn notification_rendering() {
        let mut info = test_release_info("0.7.0");
        info.release_notes = "Major improvements to context caching\nAnd more".into();
        let text = render_update_notification("0.6.9", &info);
        assert!(text.contains("0.7.0"));
        assert!(text.contains("0.6.9"));
        assert!(text.contains("cargo install"));
        assert!(text.contains("Major improvements"));
    }

    #[test]
    fn handle_update_status() {
        let output = handle_update_command(Some("status"));
        assert!(output.contains(VERSION));
    }

    #[test]
    fn handle_update_unknown_action() {
        let output = handle_update_command(Some("foo"));
        assert!(output.contains("Unknown"));
    }

    #[test]
    fn serde_roundtrip_config() {
        let config = UpdateConfig {
            enabled: true,
            check_interval_hours: 12,
            release_channel: ReleaseChannel::Beta,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: UpdateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.release_channel, ReleaseChannel::Beta);
        assert_eq!(parsed.check_interval_hours, 12);
    }

    #[test]
    fn serde_roundtrip_state() {
        let mut state = UpdateCheckState::default();
        state.mark_checked();
        state.dismissed_version = Some("1.0.0".into());
        let json = serde_json::to_string(&state).unwrap();
        let parsed: UpdateCheckState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.dismissed_version, Some("1.0.0".into()));
    }

    #[test]
    fn mark_checked_updates_epoch() {
        let mut state = UpdateCheckState::default();
        assert!(state.last_checked_epoch_secs.is_none());
        state.mark_checked();
        assert!(state.last_checked_epoch_secs.is_some());
        assert!(!state.should_check(24));
    }

    #[test]
    fn platform_target_is_known() {
        let target = platform_target();
        assert_ne!(target, "unknown-platform", "platform should be detected");
    }

    #[test]
    fn asset_filename_matches_ci_convention() {
        let name = asset_filename("0.7.0");
        let target = platform_target();
        let ext = platform_archive_ext();
        assert_eq!(name, format!("aineer-v0.7.0-{target}.{ext}"));
    }

    #[test]
    fn asset_download_url_matches_ci_convention() {
        let url = asset_download_url("0.7.0");
        let expected_name = asset_filename("0.7.0");
        assert_eq!(
            url,
            format!("https://github.com/andeya/aineer/releases/download/v0.7.0/{expected_name}")
        );
    }

    #[test]
    fn self_update_rejects_missing_asset() {
        let info = test_release_info("99.0.0");
        let err = self_update(&info).unwrap_err();
        assert!(err.contains("No prebuilt binary"), "got: {err}");
    }

    #[test]
    fn handle_update_status_shows_version() {
        let output = handle_update_command(Some("status"));
        assert!(output.contains(VERSION));
    }

    #[test]
    fn do_self_update_renders_failure() {
        let info = test_release_info("99.0.0");
        let msg = do_self_update("0.6.9", &info);
        assert!(msg.contains("Self-update failed") || msg.contains("No prebuilt binary"));
    }

    #[test]
    fn into_release_info_prefers_verified_api_url() {
        let api_url = "https://cdn.example.com/aineer-special.tar.gz";
        let expected_name = asset_filename("1.0.0");
        let release = GithubRelease {
            tag_name: "v1.0.0".into(),
            html_url: "https://github.com/andeya/aineer/releases/tag/v1.0.0".into(),
            body: None,
            published_at: None,
            prerelease: false,
            draft: false,
            assets: vec![GithubAsset {
                name: expected_name,
                browser_download_url: api_url.into(),
            }],
        };
        let info = release.into_release_info();
        assert_eq!(
            info.asset_download_url.as_deref(),
            Some(api_url),
            "should use the verified URL from GitHub API assets"
        );
    }

    #[test]
    fn into_release_info_falls_back_to_constructed_url() {
        let release = GithubRelease {
            tag_name: "v2.0.0".into(),
            html_url: "https://github.com/andeya/aineer/releases/tag/v2.0.0".into(),
            body: None,
            published_at: None,
            prerelease: false,
            draft: false,
            assets: vec![],
        };
        let info = release.into_release_info();
        let expected = asset_download_url("2.0.0");
        assert_eq!(
            info.asset_download_url.as_deref(),
            Some(expected.as_str()),
            "should fall back to locally constructed URL when API has no matching asset"
        );
    }
}
