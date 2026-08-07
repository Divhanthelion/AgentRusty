use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Status of an AI agent session (best-effort from pane scrape)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentStatus {
    /// Agent is actively processing (spinning, thinking)
    Busy,
    /// Agent is idle, waiting at a shell/agent prompt
    Idle,
    /// Agent is waiting for user input (permission, confirmation, question)
    WaitingForInput,
    /// Agent recently hit an error on the visible prompt area
    Error,
    /// Status cannot be determined
    #[default]
    Unknown,
}

/// Permission / confirmation prompts across common agent CLIs
static RE_WAITING_INPUT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(\[y/n\]|\(y/n\)|\(y/N\)|\(Y/n\)|\[Y/n\]|\[yes/no\]|\byes\s*/\s*no\b|\bAllow\b|\bDeny\b|\bApprove\b|\bpermission\b|\bWaiting for (your )?(input|confirmation|approval|permission)\b|\bDo you want to\b|\bContinue\?\b|\bPress Enter\b|\bType a message\b|tool approval|needs? (your )?approval|>\s*$)",
    )
    .unwrap()
});

static RE_BUSY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)(Thinking\.{3}|Processing\.{3}|Loading\.{3}|Working\b|⠋|⠙|⠹|⠸|⠼|⠴|⠦|⠧|⠇|⠏)",
    )
    .unwrap()
});

static RE_ERROR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)(^Error:|^error:|Exception|FAILED|panic!|fatal error|\bcrash\b)").unwrap()
});

static RE_IDLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)(^\$\s*$|^❯\s*$|^>\s*$|^➜\s*$|claude>|aider>)").unwrap()
});

/// Engine for inferring agent status from pane content
pub struct StateInferenceEngine;

impl StateInferenceEngine {
    /// Analyze pane content and determine agent status.
    ///
    /// Focuses on the last few non-empty lines so old scrollback (e.g. a past
    /// "Error:") does not sticky-label the session. Priority:
    /// WaitingForInput > Busy > Error > Idle > Unknown.
    pub fn analyze(content: &str) -> AgentStatus {
        let recent = recent_tail(content, 8);
        if recent.is_empty() {
            return AgentStatus::Unknown;
        }

        // Needs you first — the main reason to open this dashboard
        if RE_WAITING_INPUT.is_match(&recent) {
            return AgentStatus::WaitingForInput;
        }

        if RE_BUSY.is_match(&recent) {
            return AgentStatus::Busy;
        }

        // Errors only count if they're at the very end (avoid sticky old scrollback)
        let tip = recent_tail(content, 2);
        if RE_ERROR.is_match(&tip) {
            return AgentStatus::Error;
        }

        if RE_IDLE.is_match(&recent) {
            return AgentStatus::Idle;
        }

        AgentStatus::Unknown
    }
}

/// Last `n` non-empty lines, joined (bottom of pane is most relevant).
fn recent_tail(content: &str, n: usize) -> String {
    content
        .lines()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_yn_permission_prompt() {
        let content = "Edit src/main.rs?\nAllow this change? [y/n]";
        assert_eq!(
            StateInferenceEngine::analyze(content),
            AgentStatus::WaitingForInput
        );
    }

    #[test]
    fn detects_allow_deny_prompt() {
        let content = "Tool: bash\nRun command: rm -rf /tmp/foo\nAllow / Deny";
        assert_eq!(
            StateInferenceEngine::analyze(content),
            AgentStatus::WaitingForInput
        );
    }

    #[test]
    fn detects_waiting_for_approval() {
        let content = "Some work done\nWaiting for your approval";
        assert_eq!(
            StateInferenceEngine::analyze(content),
            AgentStatus::WaitingForInput
        );
    }

    #[test]
    fn detects_busy_spinner() {
        let content = "Working on the task\nThinking...";
        assert_eq!(StateInferenceEngine::analyze(content), AgentStatus::Busy);
    }

    #[test]
    fn detects_error_near_end() {
        let content = "Something went wrong\nError: connection refused";
        assert_eq!(StateInferenceEngine::analyze(content), AgentStatus::Error);
    }

    #[test]
    fn old_error_does_not_sticky_when_idle() {
        // Error buried above; recent lines look idle
        let content = "Error: connection refused\n\nfixed later\nmore output\nok\n$\n";
        assert_eq!(StateInferenceEngine::analyze(content), AgentStatus::Idle);
    }

    #[test]
    fn waiting_beats_busy_and_error_in_tail() {
        let content = "Thinking...\nError: oops\nContinue? [y/n]";
        assert_eq!(
            StateInferenceEngine::analyze(content),
            AgentStatus::WaitingForInput
        );
    }

    #[test]
    fn ellipsis_alone_is_not_busy() {
        let content = "See docs...\n$\n";
        assert_eq!(StateInferenceEngine::analyze(content), AgentStatus::Idle);
    }

    #[test]
    fn detects_idle_prompt() {
        let content = "Previous output\n$ ";
        assert_eq!(StateInferenceEngine::analyze(content), AgentStatus::Idle);
    }

    #[test]
    fn empty_is_unknown() {
        assert_eq!(StateInferenceEngine::analyze(""), AgentStatus::Unknown);
        assert_eq!(StateInferenceEngine::analyze("\n\n"), AgentStatus::Unknown);
    }
}
