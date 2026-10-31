use base64::Engine;
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use tracing::{debug, info};

use crate::config::Config;
use crate::opencode::discovery::discover_server;
use crate::opencode::events::{Decision, OpenCodeEvent, PermissionRequest, SessionInfo};

/// HTTP client for communicating with the OpenCode server.
#[derive(Debug, Clone)]
pub struct OpenCodeClient {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl OpenCodeClient {
    /// Create a new client, discover the server, and verify connectivity.
    pub async fn connect(config: &Config) -> Result<Self, String> {
        let discovered = discover_server(config)
            .await
            .ok_or_else(|| "Could not discover OpenCode server".to_string())?;

        // Prefer discovered password from service.json, fallback to config/env
        let password = discovered
            .password
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| config.resolve_password());

        let username = String::from("opencode");

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let client = Self {
            base_url: discovered.url,
            username,
            password,
            http,
        };

        // Verify connectivity by hitting the session endpoint
        client
            .list_sessions()
            .await
            .map_err(|e| format!("Failed to verify connection: {}", e))?;

        Ok(client)
    }

    /// Build the Basic Authorization header value.
    fn auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Build common headers including authorization.
    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&self.auth_header()) {
            headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    /// List all active sessions from the OpenCode server.
    /// Handles both `{"data": [...]}` (OpenCode v2) and direct array `[...]`.
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>, String> {
        let url = format!("{}/api/session", self.base_url);

        let response = self
            .http
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await
            .map_err(|e| format!("Request to {} failed: {}", url, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Server returned status {} for GET {}",
                response.status(),
                url
            ));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse session response: {}", e))?;

        if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
            let sessions = arr
                .iter()
                .filter_map(|item| {
                    item.get("id").and_then(|v| v.as_str()).map(|id| SessionInfo {
                        id: id.to_string(),
                        status: item.get("status").and_then(|s| s.as_str()).unwrap_or("active").to_string(),
                    })
                })
                .collect();
            Ok(sessions)
        } else if let Some(arr) = body.as_array() {
            let sessions = arr
                .iter()
                .filter_map(|item| {
                    item.get("id").and_then(|v| v.as_str()).map(|id| SessionInfo {
                        id: id.to_string(),
                        status: item.get("status").and_then(|s| s.as_str()).unwrap_or("active").to_string(),
                    })
                })
                .collect();
            Ok(sessions)
        } else {
            Ok(Vec::new())
        }
    }

    /// Fetch any active pending permission requests (GET /api/permission/request).
    pub async fn fetch_pending_permissions(&self) -> Result<Vec<PermissionRequest>, String> {
        let url = format!("{}/api/permission/request", self.base_url);

        let response = match self.http.get(&url).headers(self.default_headers()).send().await {
            Ok(r) => r,
            Err(e) => {
                debug!("Failed to query pending permission requests: {}", e);
                return Ok(Vec::new());
            }
        };

        if !response.status().is_success() {
            debug!("GET /api/permission/request returned HTTP {}", response.status());
            return Ok(Vec::new());
        }

        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

        let items = body
            .get("data")
            .and_then(|v| v.as_array())
            .or_else(|| body.as_array());

        let mut requests = Vec::new();
        if let Some(arr) = items {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() {
                    continue;
                }
                let session_id = item
                    .get("sessionID")
                    .or_else(|| item.get("session_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tool = item
                    .get("action")
                    .or_else(|| item.get("permission"))
                    .or_else(|| item.get("tool"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("general")
                    .to_string();

                let args = if let Some(resources) = item.get("resources") {
                    if let Some(r_arr) = resources.as_array() {
                        let lines: Vec<&str> = r_arr.iter().filter_map(|x| x.as_str()).collect();
                        serde_json::Value::String(lines.join(" "))
                    } else {
                        resources.clone()
                    }
                } else if let Some(metadata) = item.get("metadata") {
                    metadata.clone()
                } else {
                    serde_json::Value::Null
                };

                requests.push(PermissionRequest {
                    id,
                    tool,
                    args,
                    session_id,
                    timestamp: String::new(),
                });
            }
        }

        Ok(requests)
    }

    /// Subscribe to the SSE event stream from the OpenCode server.
    /// Tries /api/event (OpenCode v2), falls back to /api/events.
    pub async fn subscribe_events(&self) -> Result<impl Stream<Item = OpenCodeEvent>, String> {
        let endpoints = ["/api/event", "/api/events"];
        let mut target_response = None;

        let stream_client = reqwest::Client::builder()
            .build()
            .unwrap_or_else(|_| self.http.clone());

        for ep in &endpoints {
            let url = format!("{}{}", self.base_url, ep);
            debug!("Attempting SSE connection at {}", url);

            let mut headers = self.default_headers();
            headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

            match stream_client
                .get(&url)
                .headers(headers)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("Connected to SSE stream at {}", url);
                    target_response = Some(resp);
                    break;
                }
                Ok(resp) => {
                    debug!("SSE endpoint {} returned status {}", url, resp.status());
                }
                Err(e) => {
                    debug!("SSE connection to {} failed: {}", url, e);
                }
            }
        }

        let response = target_response.ok_or_else(|| "Could not connect to SSE stream on any endpoint".to_string())?;
        let byte_stream = response.bytes_stream();

        let event_stream = byte_stream
            .map(|chunk_result| {
                chunk_result
                    .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                    .unwrap_or_default()
            })
            .flat_map(|text| {
                let lines: Vec<String> = text.lines().map(String::from).collect();
                futures_util::stream::iter(lines)
            })
            .filter_map(|line| async move {
                let line = line.trim().to_string();
                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data.is_empty() {
                        return None;
                    }
                    match serde_json::from_str::<OpenCodeEvent>(data) {
                        Ok(event) => {
                            debug!("Parsed SSE event: {:?}", event);
                            Some(event)
                        }
                        Err(e) => {
                            debug!("Failed to parse SSE event data: {} -- raw: {}", e, data);
                            None
                        }
                    }
                } else {
                    None
                }
            });

        Ok(event_stream)
    }

    /// Respond to a permission request with the given decision.
    ///
    /// Tries OpenCode v2 path: POST /api/session/{sessionID}/permission/{requestID}/reply
    /// with decision: "once" | "reject" | "always"
    ///
    /// Falls back to generic path: POST /api/permission/{requestID}/respond
    /// with decision: "allow" | "deny" | "always"
    pub async fn respond_permission(
        &self,
        request_id: &str,
        session_id: Option<&str>,
        decision: Decision,
    ) -> Result<(), String> {
        let v2_reply = decision.as_opencode_v2_reply();

        // 1. Try OpenCode v2 session-scoped route if session_id is available
        if let Some(sid) = session_id.filter(|s| !s.is_empty()) {
            let v2_url = format!(
                "{}/api/session/{}/permission/{}/reply",
                self.base_url, sid, request_id
            );
            debug!("Trying OpenCode v2 permission reply at {}", v2_url);

            let body = serde_json::json!({
                "decision": v2_reply
            });

            if let Ok(resp) = self
                .http
                .post(&v2_url)
                .headers(self.default_headers())
                .json(&body)
                .send()
                .await
            {
                if resp.status().is_success() {
                    info!("Successfully replied to permission {} via v2 endpoint with {}", request_id, v2_reply);
                    return Ok(());
                } else {
                    debug!("v2 permission reply returned HTTP {}", resp.status());
                }
            }
        }

        // 2. Fallback to generic route
        let fallback_url = format!("{}/api/permission/{}/respond", self.base_url, request_id);
        debug!("Trying fallback permission respond at {}", fallback_url);

        let body = serde_json::json!({
            "decision": decision.as_generic_str()
        });

        let response = self
            .http
            .post(&fallback_url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to respond to permission {}: {}", request_id, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Server returned {} for permission response: {}",
                status, body_text
            ));
        }

        info!("Successfully responded to permission {} via fallback route", request_id);
        Ok(())
    }

    /// Returns the base URL of the connected server.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
