use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

mod actions;
mod app;
mod skeleton;
mod tmux;

use actions::Action;
use app::App;
use tmux::TmuxClient;

/// Ensures the terminal is restored even if the process panics.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Create event channel
    let (tx, mut rx) = mpsc::unbounded_channel::<Action>();

    // Shared flag: when true, input/poller tasks pause (during tmux attach)
    let suspended = Arc::new(AtomicBool::new(false));

    // Initialize terminal (restored on drop / panic)
    let mut terminal = ratatui::init();
    let _guard = TerminalGuard;

    // Spawn input handler — skips reading while suspended so keys go to tmux
    let input_tx = tx.clone();
    let input_suspended = Arc::clone(&suspended);
    tokio::spawn(async move {
        loop {
            if input_suspended.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }

            if event::poll(Duration::from_millis(100)).unwrap_or(false)
                && let Ok(Event::Key(key)) = event::read()
                && key.kind == KeyEventKind::Press
            {
                let _ = input_tx.send(Action::KeyPress(key));
            }
        }
    });

    // Spawn tmux poller — pauses while attached
    let tmux_tx = tx.clone();
    let poller_suspended = Arc::clone(&suspended);
    tokio::spawn(async move {
        let client = TmuxClient::new();
        loop {
            if poller_suspended.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }

            match client.list_sessions().await {
                Ok(sessions) => {
                    let _ = tmux_tx.send(Action::SessionsUpdated(sessions));
                }
                Err(e) => {
                    let _ = tmux_tx.send(Action::Error(format!("Tmux: {}", e)));
                }
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    let tmux_client = TmuxClient::new();
    let mut app = App::new();

    let result = loop {
        terminal.draw(|f| app.render(f))?;

        for pending_action in app.take_pending_actions() {
            match pending_action {
                Action::AttachSession(ref session_id) => {
                    // Pause background tasks so they don't steal keys or flood the channel
                    suspended.store(true, Ordering::SeqCst);
                    ratatui::restore();

                    let cmd = tmux_client.attach_command(session_id);
                    let status = std::process::Command::new(&cmd[0])
                        .args(&cmd[1..])
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .status();

                    // Resume TUI and drain any stale events queued before suspend
                    terminal = ratatui::init();
                    while rx.try_recv().is_ok() {}
                    suspended.store(false, Ordering::SeqCst);

                    if let Err(e) = status {
                        app.set_error(format!("Failed to attach: {}", e));
                    }
                }
                Action::CreateSession(ref name) => {
                    match tmux_client.create_session(name).await {
                        Ok(_) => {
                            app.set_success(format!("Session '{}' created", name));
                        }
                        Err(e) => {
                            app.set_error(format!("Failed to create: {}", e));
                        }
                    }
                }
                Action::DeleteSession(ref session_id) => {
                    match tmux_client.kill_session(session_id).await {
                        Ok(_) => {
                            app.set_success("Session deleted".to_string());
                        }
                        Err(e) => {
                            app.set_error(format!("Failed to delete: {}", e));
                        }
                    }
                }
                Action::CopySkeleton => {
                    match skeleton::generate_skeleton(".").await {
                        Ok(tree) => match arboard::Clipboard::new() {
                            Ok(mut clipboard) => {
                                if let Err(e) = clipboard.set_text(&tree) {
                                    app.set_error(format!("Clipboard error: {}", e));
                                } else {
                                    app.set_success("Skeleton copied to clipboard!".to_string());
                                }
                            }
                            Err(e) => {
                                app.set_error(format!("Clipboard error: {}", e));
                            }
                        },
                        Err(e) => {
                            app.set_error(format!("Skeleton error: {}", e));
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(action) = rx.recv().await {
            match app.handle_action(action) {
                Ok(should_quit) => {
                    if should_quit {
                        break Ok(());
                    }
                }
                Err(e) => {
                    break Err(e);
                }
            }
        } else {
            break Ok(());
        }
    };

    // TerminalGuard also restores on drop; explicit restore for normal exit path
    drop(_guard);
    result
}
