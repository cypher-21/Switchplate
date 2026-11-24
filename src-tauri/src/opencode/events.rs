use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use super::forms::{FormCreated, FormSettled};

/// Top-level event envelope received from the OpenCode SSE stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenCodeEvent {
    #[serde(rename = "permission.asked")]
    PermissionAsked(PermissionAskedPayload),
    #[serde(rename = "permission.replied")]
    PermissionReplied(PermissionRepliedPayload),
    #[serde(rename = "form.created")]
    FormCreated(FormCreated),
    #[serde(rename = "form.replied", alias = "form.cancelled")]
    FormSettled(FormSettled),
    #[serde(rename = "session.created")]
    SessionCreated(SessionEventPayload),
    #[serde(rename = "session.ended")]
    SessionEnded(SessionEventPayload),
    #[serde(rename = "session.deleted")]
    SessionDeleted(SessionEventPayload),
    #[serde(rename = "session.execution.started")]
    ExecutionStarted(SessionProgress),
    #[serde(
        rename = "session.execution.succeeded",
        alias = "turn.completed",
        alias = "response.completed"
    )]
    ExecutionSucceeded(SessionProgress),
    #[serde(rename = "session.execution.failed")]
    ExecutionFailed(SessionProgress),
    #[serde(rename = "session.execution.interrupted")]
    ExecutionInterrupted(SessionProgress),
    #[serde(rename = "session.status")]
    SessionStatus(SessionProgress),
    #[serde(rename = "session.idle")]
    SessionIdle(SessionProgress),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProgress {
    #[serde(default)]
    data: Option<ProgressData>,
    #[serde(default)]
    properties: Option<ProgressData>,
    #[serde(flatten)]
    flat: ProgressData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProgressData {
    #[serde(default, rename = "sessionID", alias = "session_id")]
    session_id: String,
    #[serde(default)]
    status: serde_json::Value,
}

impl SessionProgress {
    fn details(&self) -> &ProgressData {
        self.data
            .as_ref()
            .or(self.properties.as_ref())
            .unwrap_or(&self.flat)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseOutcome {
    Ready,
    Failed,
    Interrupted,
}

#[derive(Default)]
pub struct ResponseTracker {
    // The last transition for each session. An idle event after an execution
    // result must not announce the same response a second time.
    sessions: HashMap<String, SessionPhase>,
}

enum SessionPhase {
    Working,
    Finished,
}

impl ResponseTracker {
    pub fn observe(&mut self, event: &OpenCodeEvent) -> Option<ResponseOutcome> {
        let (progress, outcome, requires_activity) = match event {
            OpenCodeEvent::ExecutionStarted(progress) => (progress, None, false),
            OpenCodeEvent::ExecutionSucceeded(progress) => {
                (progress, Some(ResponseOutcome::Ready), false)
            }
            OpenCodeEvent::ExecutionFailed(progress) => {
                (progress, Some(ResponseOutcome::Failed), false)
            }
            OpenCodeEvent::ExecutionInterrupted(progress) => {
                (progress, Some(ResponseOutcome::Interrupted), false)
            }
            OpenCodeEvent::SessionIdle(progress) => (progress, Some(ResponseOutcome::Ready), true),
            OpenCodeEvent::SessionStatus(progress) => {
                let status = &progress.details().status;
                match status
                    .as_str()
                    .or_else(|| status.get("type").and_then(|value| value.as_str()))
                {
                    Some("busy" | "retry") => (progress, None, false),
                    Some("idle") => (progress, Some(ResponseOutcome::Ready), true),
                    _ => return None,
                }
            }
            OpenCodeEvent::SessionEnded(session) | OpenCodeEvent::SessionDeleted(session) => {
                self.sessions.remove(&session.session_id());
                return None;
            }
            _ => return None,
        };

        let session_id = &progress.details().session_id;
        if session_id.is_empty() {
            return None;
        }
        if outcome.is_none() {
            self.sessions
                .insert(session_id.clone(), SessionPhase::Working);
            return None;
        }
        match self.sessions.get(session_id) {
            Some(SessionPhase::Finished) => return None,
            None if requires_activity => return None,
            _ => {}
        }
        self.sessions
            .insert(session_id.clone(), SessionPhase::Finished);
        outcome
    }
}

/// Flexible payload for permission.asked events.
/// Handles OpenCode v2 format (nested in `data` or `properties`) and flat format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAskedPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    // Flat fallbacks
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub args: Option<serde_json::Value>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

impl PermissionAskedPayload {
    /// Convert this event payload into a normalized PermissionRequest.
    pub fn into_permission_request(self) -> Option<PermissionRequest> {
        let nested = self.data.as_ref().or(self.properties.as_ref());

        if let Some(val) = nested {
            let req_id = val
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&self.id)
                .to_string();
            let session_id = val
                .get("sessionID")
                .or_else(|| val.get("session_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let tool = val
                .get("permission")
                .or_else(|| val.get("tool"))
                .or_else(|| val.get("action"))
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string();

            let args = if let Some(patterns) = val.get("patterns") {
                if let Some(arr) = patterns.as_array() {
                    let items: Vec<&str> = arr.iter().filter_map(|x| x.as_str()).collect();
                    serde_json::Value::String(items.join(" "))
                } else {
                    patterns.clone()
                }
            } else if let Some(args_val) = val.get("args") {
                args_val.clone()
            } else if let Some(resources) = val.get("resources") {
                resources.clone()
            } else if let Some(meta) = val.get("metadata") {
                meta.clone()
            } else {
                serde_json::Value::Null
            };

            Some(PermissionRequest {
                id: req_id,
                tool,
                args,
                session_id,
                timestamp: self.timestamp.unwrap_or_default(),
            })
        } else if let Some(tool) = self.tool {
            Some(PermissionRequest {
                id: self.id,
                tool,
                args: self.args.unwrap_or(serde_json::Value::Null),
                session_id: self.session_id.unwrap_or_default(),
                timestamp: self.timestamp.unwrap_or_default(),
            })
        } else {
            None
        }
    }
}

/// Flexible payload for permission.replied events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRepliedPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    #[serde(default)]
    pub decision: Option<String>,
}

impl PermissionRepliedPayload {
    pub fn request_id(&self) -> String {
        let nested = self.data.as_ref().or(self.properties.as_ref());
        if let Some(val) = nested {
            val.get("requestID")
                .or_else(|| val.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or(&self.id)
                .to_string()
        } else {
            self.id.clone()
        }
    }

    pub fn decision_str(&self) -> String {
        let nested = self.data.as_ref().or(self.properties.as_ref());
        if let Some(val) = nested {
            val.get("reply")
                .or_else(|| val.get("decision"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string()
        } else {
            self.decision
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        }
    }
}

/// Flexible session payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEventPayload {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    #[serde(default)]
    pub status: String,
}

impl SessionEventPayload {
    pub fn session_id(&self) -> String {
        let nested = self.data.as_ref().or(self.properties.as_ref());
        if let Some(val) = nested {
            val.get("id")
                .or_else(|| val.get("sessionID"))
                .and_then(|v| v.as_str())
                .unwrap_or(&self.id)
                .to_string()
        } else {
            self.id.clone()
        }
    }
}

/// A normalized permission request that needs user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub tool: String,
    pub args: serde_json::Value,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub timestamp: String,
}

/// Basic session metadata from OpenCode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    #[serde(default)]
    pub status: String,
}

/// The decision a user can make on a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny,
    Always,
}

impl Decision {
    /// String expected by OpenCode v2 API (once / reject / always).
    pub fn as_opencode_v2_reply(&self) -> &'static str {
        match self {
            Decision::Allow => "once",
            Decision::Deny => "reject",
            Decision::Always => "always",
        }
    }

    /// Generic decision string (allow / deny / always).
    pub fn as_generic_str(&self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
            Decision::Always => "always",
        }
    }

    /// Parse a decision from a string, case-insensitive, supporting all aliases.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "allow" | "approve" | "accept" | "once" => Some(Decision::Allow),
            "deny" | "reject" => Some(Decision::Deny),
            "always" | "always_approve" | "always_allow" => Some(Decision::Always),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observe(tracker: &mut ResponseTracker, event: serde_json::Value) -> Option<ResponseOutcome> {
        tracker.observe(&serde_json::from_value(event).unwrap())
    }

    #[test]
    fn execution_success_announces_once_after_the_whole_response() {
        let mut tracker = ResponseTracker::default();
        for kind in [
            "session.execution.started",
            "session.step.ended",
            "message.completed",
        ] {
            assert_eq!(
                observe(
                    &mut tracker,
                    serde_json::json!({
                        "type": kind, "data": { "sessionID": "ses_1" }
                    })
                ),
                None
            );
        }
        let success = serde_json::json!({
            "type": "session.execution.succeeded", "data": { "sessionID": "ses_1" },
            "durable": { "aggregateID": "ses_1", "seq": 11, "version": 1 }
        });
        assert_eq!(
            observe(&mut tracker, success.clone()),
            Some(ResponseOutcome::Ready)
        );
        assert_eq!(observe(&mut tracker, success), None);
        assert_eq!(
            observe(
                &mut tracker,
                serde_json::json!({
                    "type": "session.idle", "data": { "sessionID": "ses_1" }
                })
            ),
            None
        );
    }

    #[test]
    fn legacy_idle_requires_work_and_terminal_errors_keep_their_outcome() {
        let mut tracker = ResponseTracker::default();
        let idle = serde_json::json!({
            "type": "session.status", "properties": { "sessionID": "ses_1", "status": { "type": "idle" } }
        });
        assert_eq!(observe(&mut tracker, idle.clone()), None);
        let busy = serde_json::json!({
            "type": "session.status", "properties": { "sessionID": "ses_1", "status": { "type": "busy" } }
        });
        assert_eq!(observe(&mut tracker, busy.clone()), None);
        assert_eq!(
            observe(&mut tracker, idle.clone()),
            Some(ResponseOutcome::Ready)
        );
        assert_eq!(observe(&mut tracker, idle.clone()), None);
        assert_eq!(observe(&mut tracker, busy), None);
        assert_eq!(
            observe(
                &mut tracker,
                serde_json::json!({
                    "type": "session.execution.failed", "data": { "sessionID": "ses_1" }
                })
            ),
            Some(ResponseOutcome::Failed)
        );
        assert_eq!(observe(&mut tracker, idle), None);
        assert_eq!(
            observe(
                &mut tracker,
                serde_json::json!({
                    "type": "session.execution.interrupted", "data": { "sessionID": "ses_2" }
                })
            ),
            Some(ResponseOutcome::Interrupted)
        );
    }
}
