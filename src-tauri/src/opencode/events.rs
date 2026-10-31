use serde::{Deserialize, Serialize};

/// Top-level event envelope received from the OpenCode SSE stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenCodeEvent {
    #[serde(rename = "permission.asked")]
    PermissionAsked(PermissionAskedPayload),
    #[serde(rename = "permission.replied")]
    PermissionReplied(PermissionRepliedPayload),
    #[serde(rename = "session.created")]
    SessionCreated(SessionEventPayload),
    #[serde(rename = "session.ended")]
    SessionEnded(SessionEventPayload),
    #[serde(rename = "session.deleted")]
    SessionDeleted(SessionEventPayload),
    #[serde(other)]
    Unknown,
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
            let req_id = val.get("id").and_then(|v| v.as_str()).unwrap_or(&self.id).to_string();
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
            self.decision.clone().unwrap_or_else(|| "unknown".to_string())
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
