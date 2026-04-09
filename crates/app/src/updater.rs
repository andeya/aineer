use serde::Deserialize;

const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/andeya/aineer/releases/latest";

/// Result of a version check.
#[derive(Debug, Clone)]
pub enum UpdateStatus {
    UpToDate,
    Available {
        tag: String,
        url: String,
        body: String,
    },
    Error(String),
    Checking,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

/// Check GitHub for a newer release (blocking HTTP, run from a background thread).
pub fn check_for_update() -> UpdateStatus {
    let current = env!("CARGO_PKG_VERSION");
    if let Err(e) = std::net::TcpStream::connect("api.github.com:443") {
        return UpdateStatus::Error(format!("Network unavailable: {e}"));
    }

    // Use a minimal HTTP client via ureq-style approach
    // For simplicity, shell out to curl (available on macOS/Linux)
    let output = std::process::Command::new("curl")
        .args([
            "-sL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: aineer-updater",
            GITHUB_RELEASES_URL,
        ])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => {
            return UpdateStatus::Error(format!("GitHub API returned {}", o.status));
        }
        Err(e) => return UpdateStatus::Error(format!("Failed to check: {e}")),
    };

    let release: GhRelease = match serde_json::from_slice(&output) {
        Ok(r) => r,
        Err(e) => return UpdateStatus::Error(format!("Failed to parse response: {e}")),
    };

    let remote_version = release.tag_name.trim_start_matches('v');
    if version_is_newer(remote_version, current) {
        UpdateStatus::Available {
            tag: release.tag_name,
            url: release.html_url,
            body: release.body.unwrap_or_default(),
        }
    } else {
        UpdateStatus::UpToDate
    }
}

fn version_is_newer(remote: &str, current: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v
            .split('.')
            .take(3)
            .filter_map(|s| s.parse().ok())
            .collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    parse(remote) > parse(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compare() {
        assert!(version_is_newer("0.8.0", "0.7.0"));
        assert!(!version_is_newer("0.7.0", "0.7.0"));
        assert!(!version_is_newer("0.6.0", "0.7.0"));
        assert!(version_is_newer("1.0.0", "0.9.9"));
        assert!(!version_is_newer("1.0", "1.0.0"));
        assert!(!version_is_newer("1.0.0-beta", "1.0.0"));
    }
}
