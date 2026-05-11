use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::state::{AppState, Pane};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_node_list(f, state, chunks[0]);
    draw_node_detail(f, state, chunks[1]);
}

fn draw_node_list(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let marker = if i == state.node_selected {
                "▶ "
            } else {
                "  "
            };
            let style = if i == state.node_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", node.name),
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

fn draw_node_detail(f: &mut Frame, state: &AppState, area: Rect) {
    let border_style = if state.active_pane == Pane::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (title, lines) = if let Some(node) = state.nodes.get(state.node_selected) {
        let title = format!(" NODE: {} ", node.name);
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Namespace: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&node.namespace),
            ]),
            Line::from(""),
        ];

        lines.push(Line::from(Span::styled(
            "Publishers:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        if node.publishers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for pub_topic in &node.publishers {
                lines.push(Line::from(Span::styled(
                    format!("  {pub_topic}"),
                    Style::default().fg(Color::Green),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Subscribers:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        if node.subscribers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for sub_topic in &node.subscribers {
                lines.push(Line::from(Span::styled(
                    format!("  {sub_topic}"),
                    Style::default().fg(Color::Yellow),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Services:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        if node.services.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for svc in &node.services {
                lines.push(Line::from(Span::styled(
                    format!("  {svc}"),
                    Style::default().fg(Color::Magenta),
                )));
            }
        }

        (title, lines)
    } else {
        (" NODE ".to_string(), vec![Line::from("No node selected")])
    };

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    f.render_widget(paragraph, area);
}
