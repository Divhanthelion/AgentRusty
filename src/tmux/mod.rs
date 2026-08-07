mod client;
mod heuristics;

pub use client::TmuxClient;
pub use heuristics::AgentStatus;

use serde::{Deserialize, Serialize};

/// Represents a tmux session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxSession {
    /// Session ID (e.g., "$0")
    pub id: String,
    /// Session name
    pub name: String,
    /// Unix timestamp when session was created
    pub created_at: u64,
    /// Whether at least one client is attached
    pub attached: bool,
    /// Number of attached clients (best-effort)
    pub attached_clients: usize,
    /// Detected agent status
    pub status: AgentStatus,
}
