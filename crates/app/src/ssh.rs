use serde::{Deserialize, Serialize};

/// Saved SSH connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub identity_file: Option<String>,
}

impl SshProfile {
    /// Build the command-line arguments for the system `ssh` command.
    pub fn to_ssh_args(&self) -> Vec<String> {
        let mut args = vec!["-o".to_string(), "ServerAliveInterval=30".to_string()];
        if self.port != 22 {
            args.push("-p".to_string());
            args.push(self.port.to_string());
        }
        if let Some(ref key) = self.identity_file {
            if !key.is_empty() {
                args.push("-i".to_string());
                args.push(key.clone());
            }
        }
        args.push(format!("{}@{}", self.user, self.host));
        args
    }
}

impl Default for SshProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: 22,
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_default(),
            identity_file: None,
        }
    }
}

/// Manager for SSH profiles (persisted to ~/.aineer/ssh_profiles.json).
pub struct SshManager {
    pub profiles: Vec<SshProfile>,
}

impl SshManager {
    pub fn new() -> Self {
        Self {
            profiles: load_profiles(),
        }
    }

    pub fn add_profile(&mut self, profile: SshProfile) {
        self.profiles.push(profile);
        self.save();
    }

    pub fn remove_profile(&mut self, index: usize) {
        if index < self.profiles.len() {
            self.profiles.remove(index);
            self.save();
        }
    }

    fn save(&self) {
        let dir = ssh_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("ssh_profiles.json");
        if let Ok(json) = serde_json::to_string_pretty(&self.profiles) {
            let _ = std::fs::write(path, json);
        }
    }
}

fn ssh_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    home.join(".aineer")
}

fn load_profiles() -> Vec<SshProfile> {
    let path = ssh_dir().join("ssh_profiles.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
