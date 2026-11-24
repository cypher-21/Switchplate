use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::opencode::events::{PermissionRequest, ResponseOutcome};
use crate::opencode::forms::{FormId, OpenCodeForm};

/// The application state that tracks connection status and pending permission requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotchState {
    pub connection: ConnectionStatus,
    pub pending_requests: VecDeque<PermissionRequest>,
    pub pending_forms: VecDeque<OpenCodeForm>,
    pub response_notices: VecDeque<ResponseNotice>,
    #[serde(skip)]
    next_notice_id: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IslandContent {
    Idle,
    Card,
    Form,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseNotice {
    pub id: u64,
    pub outcome: ResponseOutcome,
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
            pending_forms: VecDeque::new(),
            response_notices: VecDeque::new(),
            next_notice_id: 0,
        }
    }

    pub fn add_response_notice(&mut self, outcome: ResponseOutcome) {
        self.next_notice_id += 1;
        self.response_notices.push_back(ResponseNotice {
            id: self.next_notice_id,
            outcome,
        });
    }

    pub fn dismiss_response_notice(&mut self, id: u64) -> bool {
        if self
            .response_notices
            .front()
            .is_some_and(|notice| notice.id == id)
        {
            self.response_notices.pop_front();
            return true;
        }
        false
    }

    pub fn has_card(&self) -> bool {
        self.island_content() != IslandContent::Idle
    }

    pub fn island_content(&self) -> IslandContent {
        if !self.pending_requests.is_empty() {
            IslandContent::Card
        } else if !self.pending_forms.is_empty() {
            IslandContent::Form
        } else if !self.response_notices.is_empty() {
            IslandContent::Card
        } else {
            IslandContent::Idle
        }
    }

    pub fn add_form(&mut self, form: OpenCodeForm) -> bool {
        if form.key.id.is_empty()
            || form.key.session_id.is_empty()
            || self
                .pending_forms
                .iter()
                .any(|pending| pending.key == form.key)
        {
            return false;
        }
        self.pending_forms.push_back(form);
        true
    }

    pub fn remove_form(&mut self, key: &FormId) -> bool {
        let Some(position) = self.pending_forms.iter().position(|form| &form.key == key) else {
            return false;
        };
        self.pending_forms.remove(position);
        true
    }

    /// Add a permission request to the pending queue.
    pub fn add_request(&mut self, req: PermissionRequest) -> bool {
        if self
            .pending_requests
            .iter()
            .any(|pending| pending.id == req.id)
        {
            return false;
        }
        self.pending_requests.push_back(req);
        true
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
}
