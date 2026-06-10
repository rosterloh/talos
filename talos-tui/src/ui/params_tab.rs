use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::state::{AppState, Pane, node_label};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_node_list(f, state, chunks[0]);
    draw_param_pane(f, state, chunks[1]);
}

fn draw_node_list(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let marker = if i == state.param_node_selected {
                "▶ "
            } else {
                "  "
            };
            let style = if i == state.param_node_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", node_label(node)),
                style,
            )))
        })
        .collect();

    let border_style = if state.active_pane == Pane::Left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" NODES ")
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn draw_param_pane(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    draw_param_list(f, state, chunks[0]);
    draw_footer(f, state, chunks[1]);
}

fn draw_param_list(f: &mut Frame, state: &AppState, area: Rect) {
    let border_style = if state.active_pane == Pane::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = match &state.param_node {
        Some(node) => format!(" PARAMETERS · {node} "),
        None => " PARAMETERS ".to_string(),
    };

    if state.parameters.is_empty() {
        let hint = if state.param_node.is_some() {
            "No parameters."
        } else {
            "Select a node (left pane) and press Enter to load its parameters."
        };
        let para = Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        );
        f.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = state
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = state.active_pane == Pane::Right && i == state.param_selected;
            let marker = if selected { "▶ " } else { "  " };
            let name_style = if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, name_style),
                Span::styled(format!("{:<32}", p.name), name_style),
                Span::styled(p.value.to_string(), Style::default().fg(Color::Green)),
                Span::styled(
                    format!("  ({})", p.value.type_name()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, state: &AppState, area: Rect) {
    let (title, line) = if state.editing_param {
        let name = state
            .parameters
            .get(state.param_selected)
            .map(|p| p.name.as_str())
            .unwrap_or("");
        (
            " EDIT (Enter to apply, Esc to cancel) ",
            Line::from(vec![
                Span::styled(format!("set {name} = "), Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{}\u{2588}", state.param_input),
                    Style::default().fg(Color::White),
                ),
            ]),
        )
    } else if let Some(status) = &state.param_status {
        (
            " STATUS ",
            Line::from(Span::styled(
                status.clone(),
                Style::default().fg(Color::Gray),
            )),
        )
    } else {
        (
            " STATUS ",
            Line::from(Span::styled(
                "Enter: load · e: edit value · Tab: switch pane",
                Style::default().fg(Color::DarkGray),
            )),
        )
    };

    let para = Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(para, area);
}
