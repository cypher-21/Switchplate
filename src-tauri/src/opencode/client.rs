use base64::Engine;
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use tracing::{debug, info, warn};

use crate::config::{Config, OpenCodeConfig};
use crate::opencode::discovery::discover_server;
use crate::opencode::events::{Decision, OpenCodeEvent, PermissionRequest, SessionInfo};
use crate::opencode::forms::{FormAnswer, FormId, OpenCodeForm};

#[derive(Default)]
pub struct PendingInputs {
    pub permissions: Vec<PermissionRequest>,
    pub forms: Vec<OpenCodeForm>,
}

pub enum FormReply {
    Sent,
    AlreadyClosed,
}

/// HTTP client for communicating with the OpenCode server.
#[derive(Debug, Clone)]
pub struct OpenCodeClient {
    base_url: String,
    connection_config: OpenCodeConfig,
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
            .timeout(std::time::Duration::from_secs(
                config.opencode.request_timeout.max(1).into(),
            ))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let client = Self {
            base_url: discovered.url,
            connection_config: config.opencode.clone(),
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

    /// Read pending inputs across the same loaded locations as the event stream.
    pub async fn fetch_pending_inputs(&self) -> PendingInputs {
        let locations = self.loaded_locations().await;
        let targets = if locations.is_empty() {
            vec![None]
        } else {
            locations.into_iter().map(Some).collect()
        };
        let batches = futures_util::stream::iter(targets)
            .map(|directory| async move {
                let directory = directory.as_deref();
                tokio::join!(
                    self.fetch_permissions_in(directory),
                    self.fetch_forms_in(directory),
                )
            })
            .buffer_unordered(8);
        futures_util::pin_mut!(batches);

        let mut pending = PendingInputs::default();
        while let Some((permissions, forms)) = batches.next().await {
            match permissions {
                Ok(batch) => pending.permissions.extend(batch),
                Err(error) => warn!("Could not load pending OpenCode permissions: {error}"),
            }
            match forms {
                Ok(batch) => pending.forms.extend(batch),
                Err(error) => debug!("Could not load pending OpenCode forms: {error}"),
            }
        }
        pending
    }

    async fn loaded_locations(&self) -> Vec<String> {
        #[derive(serde::Deserialize)]
        struct Location {
            directory: String,
        }

        let url = format!("{}/api/debug/location", self.base_url);
        let response = self
            .http
            .get(url)
            .headers(self.default_headers())
            .send()
            .await;
        let Ok(response) = response.and_then(reqwest::Response::error_for_status) else {
            return Vec::new();
        };
        response
            .json::<Vec<Location>>()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|location| location.directory)
            .collect()
    }

    async fn fetch_permissions_in(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<PermissionRequest>, String> {
        let body = self.get_location_data("/api/permission/request", directory).await?;

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

    async fn fetch_forms_in(&self, directory: Option<&str>) -> Result<Vec<OpenCodeForm>, String> {
        #[derive(serde::Deserialize)]
        struct PendingForms {
            data: Vec<OpenCodeForm>,
        }

        let body = self.get_location_data("/api/form", directory).await?;
        serde_json::from_value::<PendingForms>(body)
            .map(|pending| pending.data)
            .map_err(|error| error.to_string())
    }

    async fn get_location_data(
        &self,
        path: &str,
        directory: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}{path}", self.base_url);
        let mut request = self.http.get(&url).headers(self.default_headers());
        if let Some(directory) = directory {
            request = request.query(&[("location[directory]", directory)]);
        }
        request.send().await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| format!("GET {url} failed: {error}"))?
            .json().await.map_err(|error| error.to_string())
    }

    pub async fn reply_to_form(&self, form: &FormId, answer: FormAnswer) -> Result<FormReply, String> {
        let url = self.api_url(&["api", "session", &form.session_id, "form", &form.id, "reply"])?;
        let response = self.http.post(url)
            .headers(self.default_headers())
            .json(&serde_json::json!({ "answer": answer }))
            .send()
            .await
            .map_err(|error| format!("Could not send the answer to OpenCode: {error}"))?;
        let status = response.status();
        if status.is_success() {
            return Ok(FormReply::Sent);
        }
        let body = response.json::<serde_json::Value>().await.unwrap_or_default();
        match body.get("_tag").and_then(|value| value.as_str()) {
            Some("FormNotFoundError" | "FormAlreadySettledError" | "SessionNotFoundError") => {
                Ok(FormReply::AlreadyClosed)
            }
            _ => Err(body.get("message").and_then(|value| value.as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("OpenCode could not accept the answer ({status})."))),
        }
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

        // Stateful byte-buffering SSE parser.
        // Accumulates raw bytes across TCP chunks and only emits complete lines
        // (terminated by \n). This prevents:
        //   (a) Dropping events whose JSON payload is split across two TCP chunks.
        //   (b) Corrupting multibyte UTF-8 codepoints split at a chunk boundary.
        let event_stream = futures_util::stream::unfold(
            (byte_stream, Vec::<u8>::new()),
            |(mut stream, mut buf)| async move {
                loop {
                    // Drain all complete lines already sitting in the buffer.
                    if let Some(newline_pos) = buf.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buf.drain(..=newline_pos).collect();
                        let line = String::from_utf8(line_bytes)
                            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).to_string());
                        let line = line.trim().to_string();

                        if let Some(data) = line.strip_prefix("data: ") {
                            let data = data.trim();
                            if data.is_empty() {
                                continue; // Empty data field, skip
                            }
                            match serde_json::from_str::<OpenCodeEvent>(data) {
                                Ok(event) => {
                                    debug!("Parsed SSE event: {:?}", event);
                                    return Some((event, (stream, buf)));
                                }
                                Err(e) => {
                                    debug!("Failed to parse SSE event data: {} -- raw: {}", e, data);
                                    continue; // Malformed JSON, skip this line
                                }
                            }
                        } else {
                            // Not a data: line (could be event:, id:, retry:, comment, or blank).
                            // Per SSE spec, ignore these.
                            continue;
                        }
                    }

                    // No complete line in buffer — pull the next chunk from the network.
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            buf.extend_from_slice(&bytes);
                            // Loop back to drain any complete lines from the enlarged buffer.
                        }
                        Some(Err(e)) => {
                            debug!("SSE stream chunk error: {}", e);
                            return None; // Network error, end the stream.
                        }
                        None => {
                            // Stream ended. Flush any trailing data still in the buffer
                            // (server may have closed without a final newline).
                            if !buf.is_empty() {
                                let remaining = std::mem::take(&mut buf);
                                let line = String::from_utf8(remaining)
                                    .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).to_string());
                                let line = line.trim().to_string();
                                if let Some(data) = line.strip_prefix("data: ") {
                                    let data = data.trim();
                                    if !data.is_empty() {
                                        if let Ok(event) = serde_json::from_str::<OpenCodeEvent>(data) {
                                            debug!("Parsed final SSE event: {:?}", event);
                                            return Some((event, (stream, buf)));
                                        }
                                    }
                                }
                            }
                            return None; // Stream finished.
                        }
                    }
                }
            },
        );

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
        request: &PermissionRequest,
        decision: Decision,
    ) -> Result<(), String> {
        let request_id = &request.id;
        let v2_reply = decision.as_opencode_v2_reply();

        // 1. Try OpenCode v2 session-scoped route if session_id is available
        if !request.session_id.is_empty() {
            let sid = &request.session_id;
            let v2_url =
                self.api_url(&["api", "session", sid, "permission", request_id, "reply"])?;
            debug!("Trying OpenCode v2 permission reply at {}", v2_url);

            let body = serde_json::json!({
                "decision": v2_reply
            });

            let response = self
                .http
                .post(v2_url)
                .headers(self.default_headers())
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Failed to respond to permission {request_id}: {e}"))?;
            if response.status().is_success() {
                info!(
                    "Successfully replied to permission {} via v2 endpoint with {}",
                    request_id, v2_reply
                );
                return Ok(());
            }
            let status = response.status();
            if status != reqwest::StatusCode::NOT_FOUND
                && status != reqwest::StatusCode::METHOD_NOT_ALLOWED
            {
                return Err(format!(
                    "OpenCode returned {status} for permission {request_id}"
                ));
            }
            debug!("v2 permission reply returned HTTP {}", status);
        }

        // 2. Fallback to generic route
        let fallback_url = self.api_url(&["api", "permission", request_id, "respond"])?;
        debug!("Trying fallback permission respond at {}", fallback_url);

        let body = serde_json::json!({
            "decision": decision.as_generic_str()
        });

        let response = self
            .http
            .post(fallback_url)
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

    fn api_url(&self, segments: &[&str]) -> Result<reqwest::Url, String> {
        if segments
            .iter()
            .any(|segment| matches!(*segment, "" | "." | ".."))
        {
            return Err("Invalid OpenCode resource ID".to_string());
        }
        let mut url = reqwest::Url::parse(&self.base_url).map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "Invalid OpenCode server URL".to_string())?
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }

    pub async fn auto_approve(
        &self,
        request: &PermissionRequest,
        config: &Config,
    ) -> Result<bool, String> {
        if self.connection_config != config.opencode {
            return Ok(false);
        }
        let Some(decision) = config.behavior.automatic_decision() else {
            return Ok(false);
        };
        self.respond_permission(request, decision).await?;
        info!(
            "Automatically replied to permission {} with {}",
            request.id,
            decision.as_opencode_v2_reply()
        );
        Ok(true)
    }

    /// Returns the base URL of the connected server.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::config::AutoApproval;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    #[tokio::test]
    async fn automatic_approval_sends_the_selected_opencode_reply() {
        let path =
            std::env::temp_dir().join(format!("switchplate-test-{}.sock", std::process::id()));
        let listener = UnixListener::bind(&path).unwrap();
        let mut config = Config::default();
        let client = OpenCodeClient {
            base_url: "http://localhost".into(),
            connection_config: config.opencode.clone(),
            username: "opencode".into(),
            password: String::new(),
            http: reqwest::Client::builder()
                .unix_socket(path.clone())
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .unwrap(),
        };
        let request = PermissionRequest {
            id: "per_test".into(),
            tool: "shell".into(),
            args: serde_json::Value::Null,
            session_id: "ses_test".into(),
            timestamp: String::new(),
        };

        for (mode, decision, status) in [
            (AutoApproval::AllowOnce, "once", "204 No Content"),
            (AutoApproval::AlwaysAllow, "always", "204 No Content"),
            (AutoApproval::AllowOnce, "once", "503 Service Unavailable"),
        ] {
            config.behavior.auto_approval = mode;
            let serve = async {
                let (stream, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                reader.read_line(&mut line).await.unwrap();
                assert_eq!(
                    line,
                    "POST /api/session/ses_test/permission/per_test/reply HTTP/1.1\r\n"
                );
                let mut length = 0;
                loop {
                    line.clear();
                    let bytes = reader.read_line(&mut line).await.unwrap();
                    assert_ne!(bytes, 0);
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("content-length: ") {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).await.unwrap();
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                    serde_json::json!({ "decision": decision }),
                );
                let response =
                    format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                reader
                    .get_mut()
                    .write_all(response.as_bytes())
                    .await
                    .unwrap();
            };
            let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                tokio::join!(client.auto_approve(&request, &config), serve)
            })
            .await
            .unwrap();
            if status.starts_with("204") {
                assert_eq!(result, Ok(true));
            } else {
                assert!(result.unwrap_err().contains("503"));
            }
        }

        config.behavior.pause_monitoring = true;
        assert_eq!(client.auto_approve(&request, &config).await, Ok(false));
        std::fs::remove_file(path).unwrap();
    }
}
