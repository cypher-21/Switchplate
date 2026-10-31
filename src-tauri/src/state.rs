use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::opencode::events::PermissionRequest;

/// The application state that tracks connection status and pending permission requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotchState {
    pub connection: ConnectionStatus,
    pub pending_requests: VecDeque<PermissionRequest>,
}

/// Represents the current connection status to the OpenCode server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ConnectionStatus {
    #[serde(rename = "disconnected")]
    Disconnected,
    #[serde(rename = "connecting")]
    Connecting,
    #[serde(rename = "connected")]
    Connected { session_count: usize },
}

impl NotchState {
    /// Create a new state initialized as disconnected with no pending requests.
    pub fn new() -> Self {
        Self {
            connection: ConnectionStatus::Disconnected,
            pending_requests: VecDeque::new(),
        }
    }

    /// Add a permission request to the pending queue.
    pub fn add_request(&mut self, req: PermissionRequest) {
        self.pending_requests.push_back(req);
    }

    /// Remove and return a permission request by its ID.
    pub fn remove_request(&mut self, id: &str) -> Option<PermissionRequest> {
        if let Some(pos) = self.pending_requests.iter().position(|r| r.id == id) {
            self.pending_requests.remove(pos)
        } else {
            None
        }
    }

    /// Returns a reference to the frontmost pending request, if any.
    pub fn current_request(&self) -> Option<&PermissionRequest> {
        self.pending_requests.front()
    }

    /// Returns the number of pending permission requests.
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }
}
