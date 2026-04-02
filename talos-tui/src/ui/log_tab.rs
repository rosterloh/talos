use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

use crate::state::AppState;

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    draw_log_table(f, state, chunks[0]);
    draw_filter_bar(f, state, chunks[1]);
}

fn draw_log_table(f: &mut Frame, state: &AppState, area: Rect) {
    let filtered = state.filtered_log_entries();

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let level_style = match entry.level.as_str() {
                "DEBUG" => Style::default().fg(Color::DarkGray),
                "INFO" => Style::default().fg(Color::Green),
                "WARN" => Style::default().fg(Color::Yellow),
                "ERROR" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                "FATAL" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                _ => Style::default(),
            };

            let row_style = if i == state.log_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                ratatui::widgets::Cell::from(entry.timestamp.clone()),
                ratatui::widgets::Cell::from(entry.level.clone()).style(level_style),
                ratatui::widgets::Cell::from(entry.node.clone()),
                ratatui::widgets::Cell::from(entry.message.clone()),
            ])
            .style(row_style)
        })
        .collect();

    let header = Row::new(vec!["Time", "Level", "Node", "Message"])
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(25),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" LOG "));

    f.render_widget(table, area);
}

fn draw_filter_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let filter_text = Line::from(vec![
        Span::styled("  Filter: ", Style::default().fg(Color::DarkGray)),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled(
            state.log_severity_filter.label(),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled("]  ", Style::default().fg(Color::DarkGray)),
        Span::styled("node: ", Style::default().fg(Color::DarkGray)),
        if state.log_node_filter.is_empty() {
            Span::styled("(all)", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(&state.log_node_filter, Style::default().fg(Color::White))
        },
        Span::raw("  "),
        Span::styled("search: ", Style::default().fg(Color::DarkGray)),
        if state.log_search.is_empty() {
            Span::styled("(none)", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(&state.log_search, Style::default().fg(Color::White))
        },
    ]);

    let bar = Paragraph::new(filter_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(bar, area);
}
