use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const DEFAULT_REMOTE_BASE_URL: &str = "https://api.anthropic.com";
pub const DEFAULT_SESSION_TOKEN_PATH: &str = "/run/ccr/session_token";
pub const DEFAULT_SYSTEM_CA_BUNDLE: &str = "/etc/ssl/certs/ca-certificates.crt";

pub const UPSTREAM_PROXY_ENV_KEYS: [&str; 8] = [
    "HTTPS_PROXY",
    "https_proxy",
    "NO_PROXY",
    "no_proxy",
    "SSL_CERT_FILE",
    "NODE_EXTRA_CA_CERTS",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
];

pub const NO_PROXY_HOSTS: [&str; 16] = [
    "localhost",
    "127.0.0.1",
    "::1",
    "169.254.0.0/16",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "anthropic.com",
    ".anthropic.com",
    "*.anthropic.com",
    "github.com",
    "api.github.com",
    "*.github.com",
    "*.githubusercontent.com",
    "registry.npmjs.org",
    "index.crates.io",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSessionContext {
    pub enabled: bool,
    pub session_id: Option<String>,
    pub base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamProxyBootstrap {
    pub remote: RemoteSessionContext,
    pub upstream_proxy_enabled: bool,
