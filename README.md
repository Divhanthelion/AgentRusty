# AgentRusty

Terminal dashboard for managing multiple AI coding agents via tmux.

Run Kimi, OpenCode, Aider, Claude, or anything else in separate tmux sessions. AgentRusty lists them, scrapes each pane for a best-effort status (especially **needs input** / permission prompts), and lets you attach to the right session fast.

It does **not** configure MCP or own the agents' tools — those stay with each CLI.

## Requirements

- [Rust](https://rustup.rs/) (edition 2024)
- [tmux](https://github.com/tmux/tmux)

## Run

```bash
cargo run --release
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` or arrows | Navigate sessions |
| `Enter` | Attach to selected session (detach with tmux prefix + `d`) |
| `n` | Create empty detached session |
| `d` | Delete selected session (confirm with `y`) |
| `y` | Copy project skeleton tree to clipboard |
| `q` / `Ctrl+C` | Quit |

Sessions waiting for input sort to the top and show a `needs you` badge.

## Status (best-effort)

Status comes from `tmux capture-pane` + heuristics on the last few lines:

- **Needs input** — permission / confirm / approval prompts
- **Busy** — spinners / “Thinking…” / “Working”
- **Error** — recent error text at the tip of the pane
- **Idle** — shell/agent prompt
- **Unknown** — couldn't tell

False positives happen; treat it as a glanceable hint, not truth.

## Create flow

`n` creates an empty tmux session. Attach and launch your agent yourself inside it.
