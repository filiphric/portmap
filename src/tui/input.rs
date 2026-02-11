use crate::app::{InputMode, Mapping, MappingStatus, PopupField, TuiState};
use crossterm::event::{KeyCode, KeyEvent};

/// Result of processing a key event.
pub enum InputResult {
    /// Continue running
    Continue,
    /// User wants to quit
    Quit,
}

/// Process a key event in Normal mode.
pub fn handle_normal_key(
    key: KeyEvent,
    state: &mut TuiState,
    mappings: &[Mapping],
) -> InputResult {
    match key.code {
        KeyCode::Char('q') => InputResult::Quit,
        KeyCode::Char('a') => {
            state.mode = InputMode::Adding;
            state.domain_input.clear();
            state.port_input.clear();
            state.popup_field = PopupField::Domain;
            state.status_message = None;
            InputResult::Continue
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !mappings.is_empty() {
                state.selected = (state.selected + 1).min(mappings.len() - 1);
            }
            InputResult::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.selected = state.selected.saturating_sub(1);
            InputResult::Continue
        }
        KeyCode::Char('d') => {
            // Delete will be handled by the caller since it needs mutable access to mappings
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

/// Process a key event in Adding mode.
pub fn handle_adding_key(key: KeyEvent, state: &mut TuiState) -> InputResult {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            InputResult::Continue
        }
        KeyCode::Tab | KeyCode::BackTab => {
            state.popup_field = match state.popup_field {
                PopupField::Domain => PopupField::Port,
                PopupField::Port => PopupField::Domain,
            };
            InputResult::Continue
        }
        KeyCode::Backspace => {
            match state.popup_field {
                PopupField::Domain => {
                    state.domain_input.pop();
                }
                PopupField::Port => {
                    state.port_input.pop();
                }
            }
            InputResult::Continue
        }
        KeyCode::Char(c) => {
            match state.popup_field {
                PopupField::Domain => {
                    // Only allow valid domain characters
                    if c.is_ascii_alphanumeric() || c == '-' {
                        state.domain_input.push(c);
                    }
                }
                PopupField::Port => {
                    // Only allow digits
                    if c.is_ascii_digit() {
                        state.port_input.push(c);
                    }
                }
            }
            InputResult::Continue
        }
        KeyCode::Enter => {
            // Validation is handled by the caller
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

/// Validate and create a mapping from the current popup input.
/// Returns Ok(Mapping) or Err(error message).
pub fn validate_input(state: &TuiState) -> Result<Mapping, String> {
    let domain_base = state.domain_input.trim().to_lowercase();
    if domain_base.is_empty() {
        return Err("Domain cannot be empty".to_string());
    }
    if !domain_base
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("Domain can only contain letters, numbers, and hyphens".to_string());
    }
    if domain_base.starts_with('-') || domain_base.ends_with('-') {
        return Err("Domain cannot start or end with a hyphen".to_string());
    }

    let port: u16 = state
        .port_input
        .trim()
        .parse()
        .map_err(|_| "Port must be a number between 1 and 65535".to_string())?;

    if port == 0 {
        return Err("Port must be between 1 and 65535".to_string());
    }

    let domain = format!("{}.localhost", domain_base);

    Ok(Mapping {
        domain,
        port,
        status: MappingStatus::Unknown,
    })
}

/// Check if a port is reachable by attempting a TCP connection.
pub async fn check_port(port: u16) -> MappingStatus {
    match tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
        Ok(_) => MappingStatus::Active,
        Err(_) => MappingStatus::PortUnreachable,
    }
}
