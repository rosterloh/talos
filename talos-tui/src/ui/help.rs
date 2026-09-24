use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn draw(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);
    f.render_widget(Clear, popup_area);

    let lines = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1-5      ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch tab"),
        ]),
        Line::from(vec![
            Span::styled("  Tab      ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch pane"),
        ]),
        Line::from(vec![
            Span::styled("  ↑/↓      ", Style::default().fg(Color::Yellow)),
            Span::raw("Navigate list"),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(Color::Yellow)),
            Span::raw("Select / expand"),
        ]),
        Line::from(vec![
            Span::styled("  ←/→      ", Style::default().fg(Color::Yellow)),
            Span::raw("Collapse / expand tree"),
        ]),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(Color::Yellow)),
            Span::raw("Refresh topic and node lists"),
        ]),
        Line::from(vec![
            Span::styled("  /        ", Style::default().fg(Color::Yellow)),
            Span::raw("Filter list / search log (case-insensitive substring)"),
        ]),
        Line::from(vec![
            Span::styled("           ", Style::default().fg(Color::Yellow)),
            Span::raw("←/→ cursor, Ctrl-U clear, Enter apply, Esc cancel"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Topics Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle selected topic subscription (either pane)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Nodes Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  l        ", Style::default().fg(Color::Yellow)),
            Span::raw("Load selected node's logger level"),
        ]),
        Line::from(vec![
            Span::styled("  L        ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle selected node's logger level"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Log Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  f        ", Style::default().fg(Color::Yellow)),
            Span::raw("Cycle severity filter"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Joints Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  e        ", Style::default().fg(Color::Yellow)),
            Span::raw("Edit selected joint position"),
        ]),
        Line::from(vec![
            Span::styled("  x        ", Style::default().fg(Color::Yellow)),
            Span::raw("Execute selected pose"),
        ]),
        Line::from(vec![
            Span::styled("  j        ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch to joint list"),
        ]),
        Line::from(vec![
            Span::styled("  o        ", Style::default().fg(Color::Yellow)),
            Span::raw("Switch to pose list"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Params Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(Color::Yellow)),
            Span::raw("Load parameters for selected node"),
        ]),
        Line::from(vec![
            Span::styled("  e        ", Style::default().fg(Color::Yellow)),
            Span::raw("Edit selected parameter (right pane)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q        ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit"),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle this help"),
        ]),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .style(Style::default().fg(Color::White).bg(Color::Black)),
    );

    f.render_widget(help, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}
