use crate::app::{InputMode, Mapping, TuiState};
use crate::hosts::manager::HostsManager;
use crate::tui::input::{
    check_port, handle_adding_key, handle_normal_key, validate_input, InputResult,
};
use crate::tui::ui;
use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use tokio::sync::watch;

/// Run the TUI event loop.
pub async fn run_tui(
    mappings_tx: watch::Sender<Vec<Mapping>>,
    hosts_manager: HostsManager,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new();
    let mut reader = EventStream::new();
    let mut status_check_interval = tokio::time::interval(Duration::from_secs(3));

    let result = loop {
        // Draw
        let mappings = mappings_tx.borrow().clone();
        terminal.draw(|f| ui::draw(f, &state, &mappings))?;

        tokio::select! {
            // Terminal events
            maybe_event = reader.next() => {
                let Some(Ok(event)) = maybe_event else {
                    break Ok(());
                };
                if let Event::Key(key) = event {
                    // crossterm sends both Press and Release on some platforms
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match state.mode {
                        InputMode::Normal => {
                            // Handle delete specially since it needs mutable mappings
                            if key.code == KeyCode::Char('d') {
                                let mut mappings = mappings_tx.borrow().clone();
                                if !mappings.is_empty() && state.selected < mappings.len() {
                                    let removed = mappings.remove(state.selected);
                                    let _ = hosts_manager.remove_entry(&removed.domain);
                                    if state.selected > 0 && state.selected >= mappings.len() {
                                        state.selected = mappings.len().saturating_sub(1);
                                    }
                                    state.status_message = Some(format!("Removed {}", removed.domain));
                                    mappings_tx.send(mappings)?;
                                }
                                continue;
                            }

                            match handle_normal_key(key, &mut state, &mappings_tx.borrow()) {
                                InputResult::Quit => break Ok(()),
                                InputResult::Continue => {}
                            }
                        }
                        InputMode::Adding => {
                            if key.code == KeyCode::Enter {
                                match validate_input(&state) {
                                    Ok(mut mapping) => {
                                        // Check port reachability
                                        mapping.status = check_port(mapping.port).await;

                                        // Add to hosts file
                                        match hosts_manager.add_entry(&mapping.domain) {
                                            Ok(true) => {
                                                let mut mappings = mappings_tx.borrow().clone();
                                                state.status_message = Some(format!(
                                                    "Added {} \u{2192} :{}",
                                                    mapping.domain, mapping.port
                                                ));
                                                mappings.push(mapping);
                                                mappings_tx.send(mappings)?;
                                                state.mode = InputMode::Normal;
                                            }
                                            Ok(false) => {
                                                state.status_message = Some("Mapping already exists".to_string());
                                            }
                                            Err(e) => {
                                                state.status_message = Some(format!("Error: {}", e));
                                            }
                                        }
                                    }
                                    Err(msg) => {
                                        state.status_message = Some(msg);
                                    }
                                }
                                continue;
                            }
                            handle_adding_key(key, &mut state);
                        }
                    }
                }
            }
            // Periodic port status checks
            _ = status_check_interval.tick() => {
                let mut mappings = mappings_tx.borrow().clone();
                let mut changed = false;
                for mapping in &mut mappings {
                    let new_status = check_port(mapping.port).await;
                    if new_status != mapping.status {
                        mapping.status = new_status;
                        changed = true;
                    }
                }
                if changed {
                    mappings_tx.send(mappings)?;
                }
            }
            // Shutdown signal
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break Ok(());
                }
            }
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}
