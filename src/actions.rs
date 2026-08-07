use crossterm::event::KeyEvent;

use crate::tmux::TmuxSession;

/// Actions that can be dispatched through the application
#[derive(Debug, Clone)]
pub enum Action {
    /// A key was pressed
    KeyPress(KeyEvent),
    /// Sessions were updated from tmux
    SessionsUpdated(Vec<TmuxSession>),
    /// An error occurred
    Error(String),
    /// Attach to a session
    AttachSession(String),
    /// Create a new session
    CreateSession(String),
    /// Delete a session
    DeleteSession(String),
    /// Copy skeleton map to clipboard
    CopySkeleton,
}
