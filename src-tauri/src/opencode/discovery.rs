use crate::config::Config;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{debug, info};

#[derive(Debug, Clone, Deserialize)]
struct ServiceJson {
    pub url: Option<String>,
    pub password: Option<String>,
}

/// Discovered server information: base URL and optional password found in service state.
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    pub url: String,
    pub password: Option<String>,
}

/// Discover a running OpenCode server by trying multiple sources in priority order:
/// 1. Explicit server_url from configuration
/// 2. OPENCODE_SERVER_URL environment variable
/// 3. ~/.local/state/opencode/service.json and ~/.config/opencode/service.json
/// 4. Default http://127.0.0.1:4096
///
/// Each candidate is probed with a quick HTTP request to verify it is reachable.
pub async fn discover_server(config: &Config) -> Option<DiscoveredServer> {
    let mut candidates: Vec<(String, Option<String>)> = Vec::new();

    // Priority 1: config file server_url (if not default)
    if !config.opencode.server_url.is_empty() && config.opencode.server_url != "http://localhost:4096" {
        candidates.push((normalize_url(&config.opencode.server_url), None));
    }

    // Priority 2: environment variable
    if let Ok(env_url) = std::env::var("OPENCODE_SERVER_URL") {
        let normalized = normalize_url(&env_url);
        candidates.push((normalized, std::env::var("OPENCODE_SERVER_PASSWORD").ok()));
    }

    // Priority 3: check service.json in OpenCode state directories
    for path in service_json_paths() {
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(svc) = serde_json::from_str::<ServiceJson>(&contents) {
                    if let Some(url) = svc.url {
                        let normalized = normalize_url(&url);
                        info!("Found running OpenCode service in {} at {}", path.display(), normalized);
                        candidates.push((normalized, svc.password));
                    }
                }
            }
        }
    }

    // Priority 4: default config URL and localhost:4096 fallback
    let default_url = normalize_url(&config.opencode.server_url);
    candidates.push((default_url, None));
    candidates.push((String::from("http://127.0.0.1:4096"), None));

    let probe_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    for (url, password) in candidates {
        debug!("Probing OpenCode server at {}", url);
        let probe_url = format!("{}/api/session", url.trim_end_matches('/'));

        match probe_client.get(&probe_url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_client_error() => {
                // Any 2xx, 3xx, 4xx (including 401 Unauthorized) means an active server responded!
                info!("Discovered active OpenCode server at {}", url);
                return Some(DiscoveredServer { url, password });
            }
            Ok(resp) => {
                debug!("Server at {} responded with unexpected status: {}", url, resp.status());
            }
            Err(e) => {
                debug!("Server at {} is not reachable: {}", url, e);
            }
        }
    }

    None
}

/// Paths to check for OpenCode's service.json state file.
fn service_json_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local/state/opencode/service.json"));
        paths.push(home.join(".config/opencode/service.json"));
    }

    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("opencode/service.json"));
    }

    paths
}

/// Ensure the URL has a scheme and no trailing slash.
fn normalize_url(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("http://{}", url)
    } else {
        url.to_string()
    }
}
