mod help;
mod joints_tab;
mod log_tab;
mod nodes_tab;
mod topics_tab;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use crate::state::{AppState, Tab, TransportType};

pub fn draw(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),   // Content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_tab_bar(f, state, chunks[0]);

    match state.active_tab {
        Tab::Topics => topics_tab::draw(f, state, chunks[1]),
        Tab::Nodes => nodes_tab::draw(f, state, chunks[1]),
        Tab::Log => log_tab::draw(f, state, chunks[1]),
        Tab::Joints => joints_tab::draw(f, state, chunks[1]),
    }

    draw_status_bar(f, state, chunks[2]);

    if state.show_help {
        help::draw(f, f.area());
    }
}

fn draw_tab_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            Line::from(vec![
                Span::styled(
                    format!("[{}]", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(tab.label()),
            ])
        })
        .collect();

    let connection = if state.connected {
        let label = match state.transport_type {
            Some(TransportType::Uds) => " ● connected (uds) ",
            Some(TransportType::Quic) => " ● connected (quic) ",
            None => " ● connected ",
        };
        Span::styled(label, Style::default().fg(Color::Green))
    } else {
        Span::styled(" ● disconnected ", Style::default().fg(Color::Red))
    };
    let indicator_width = connection.content.len() as u16;

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Talos ")
                .title_alignment(ratatui::layout::Alignment::Left),
        )
        .select(state.active_tab.index())
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);

    // Connection indicator in top-right
    let indicator = Paragraph::new(connection);
    let indicator_area = Rect {
        x: area.right().saturating_sub(indicator_width),
        y: area.y,
        width: indicator_width.min(area.width),
        height: 1,
    };
    f.render_widget(indicator, indicator_area);
}

fn draw_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let hints = match state.active_tab {
        Tab::Topics => "↑↓ navigate  Enter select  ←→ expand/collapse  Tab pane  q quit  ? help",
        Tab::Nodes => "↑↓ navigate  Enter select  Tab pane  q quit  ? help",
        Tab::Log => "↑↓ scroll  f filter severity  n filter node  / search  q quit  ? help",
        Tab::Joints => "↑↓ navigate  ←→ adjust  Enter edit  p execute pose  q quit  ? help",
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(bar, area);
}
