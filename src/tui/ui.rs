use crate::app::{InputMode, Mapping, MappingStatus, PopupField, TuiState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table,
};
use ratatui::Frame;

/// Render the entire TUI.
pub fn draw(f: &mut Frame, state: &TuiState, mappings: &[Mapping]) {
    let size = f.area();

    // Main layout: table area + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(size);

    draw_table(f, chunks[0], state, mappings);
    draw_status_bar(f, chunks[1], state, mappings);

    if state.mode == InputMode::Adding {
        draw_popup(f, size, state);
    }
}

fn draw_table(f: &mut Frame, area: Rect, state: &TuiState, mappings: &[Mapping]) {
    let header = Row::new(vec![
        Cell::from("Domain").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Port").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ])
    .height(1);

    let rows: Vec<Row> = mappings
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == state.selected;
            let prefix = if selected { "\u{25b8} " } else { "  " };
            let status_style = match m.status {
                MappingStatus::Active => Style::default().fg(Color::Green),
                MappingStatus::PortUnreachable => Style::default().fg(Color::Red),
                MappingStatus::Unknown => Style::default().fg(Color::DarkGray),
            };
            let status_text = match m.status {
                MappingStatus::Active => "\u{25cf} Active",
                MappingStatus::PortUnreachable => "\u{25cf} Port Unreachable",
                MappingStatus::Unknown => "\u{25cf} Unknown",
            };

            let style = if selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{}{}", prefix, m.domain)).style(style),
                Cell::from(m.port.to_string()).style(style),
                Cell::from(status_text).style(status_style),
            ])
        })
        .collect();

    let title = Line::from(vec![
        Span::styled(" portmap ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]);
    let keyhints = Line::from(vec![
        Span::styled("[a]", Style::default().fg(Color::Green)),
        Span::raw("dd "),
        Span::styled("[d]", Style::default().fg(Color::Red)),
        Span::raw("el "),
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw("uit "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_bottom(keyhints);

    let widths = [
        Constraint::Percentage(50),
        Constraint::Percentage(15),
        Constraint::Percentage(35),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, state: &TuiState, mappings: &[Mapping]) {
    let msg = state
        .status_message
        .as_deref()
        .unwrap_or("");

    let status = Line::from(vec![
        Span::styled(
            " Proxy running on :80",
            Style::default().fg(Color::Green),
        ),
        Span::raw(" \u{2502} "),
        Span::styled(
            format!("{} mapping{}", mappings.len(), if mappings.len() == 1 { "" } else { "s" }),
            Style::default().fg(Color::Cyan),
        ),
        if !msg.is_empty() {
            Span::raw(format!(" \u{2502} {}", msg))
        } else {
            Span::raw("")
        },
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let paragraph = Paragraph::new(status).block(block);
    f.render_widget(paragraph, area);
}

fn draw_popup(f: &mut Frame, area: Rect, state: &TuiState) {
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 9u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Line::from(Span::styled(
            " Add Mapping ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // domain label
            Constraint::Length(1), // domain input
            Constraint::Length(1), // spacing
            Constraint::Length(1), // port label
            Constraint::Length(1), // port input
            Constraint::Min(0),   // hints
        ])
        .split(inner);

    let domain_focused = state.popup_field == PopupField::Domain;
    let port_focused = state.popup_field == PopupField::Port;

    // Domain field
    let domain_label = Paragraph::new(Line::from(vec![
        Span::styled(
            "Domain: ",
            if domain_focused {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]));
    f.render_widget(domain_label, chunks[0]);

    let cursor_style = if domain_focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let domain_value = Paragraph::new(Line::from(vec![
        Span::styled(&state.domain_input, Style::default().fg(Color::White)),
        Span::styled(".localhost", Style::default().fg(Color::DarkGray)),
    ]))
    .style(cursor_style);
    f.render_widget(domain_value, chunks[1]);

    if domain_focused {
        f.set_cursor_position((
            chunks[1].x + state.domain_input.len() as u16,
            chunks[1].y,
        ));
    }

    // Port field
    let port_label = Paragraph::new(Line::from(vec![
        Span::styled(
            "Port: ",
            if port_focused {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]));
    f.render_widget(port_label, chunks[3]);

    let port_value = Paragraph::new(Line::from(Span::styled(
        &state.port_input,
        Style::default().fg(Color::White),
    )));
    f.render_widget(port_value, chunks[4]);

    if port_focused {
        f.set_cursor_position((
            chunks[4].x + state.port_input.len() as u16,
            chunks[4].y,
        ));
    }

    // Hints
    if chunks[5].height > 0 {
        let hints = Paragraph::new(Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" switch  "),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" submit  "),
            Span::styled("Esc", Style::default().fg(Color::Red)),
            Span::raw(" cancel"),
        ]))
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(hints, chunks[5]);
    }
}
