use aineer_release_channel::ReleaseChannel;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AutoUpdateError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("No update available")]
    NoUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub release_notes: Option<String>,
}

pub struct AutoUpdater {
    channel: ReleaseChannel,
    current_version: String,
    check_url: String,
}

impl AutoUpdater {
    pub fn new(current_version: String) -> Self {
        let channel = ReleaseChannel::current();
        let check_url = format!(
            "https://updates.aineer.dev/api/check?channel={}&version={}&platform={}",
            channel,
            current_version,
            std::env::consts::OS,
        );
        Self {
            channel,
            current_version,
            check_url,
        }
    }

    pub fn channel(&self) -> ReleaseChannel {
        self.channel
    }

    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    pub async fn check_for_update(&self) -> Result<Option<UpdateInfo>, AutoUpdateError> {
        let resp = reqwest::get(&self.check_url).await?;
        if resp.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let info: UpdateInfo = resp.json().await?;
        if info.version != self.current_version {
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }
}
