use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::state::{AppState, Pane};
use talos_common::protocol::types::DynValue;

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_topic_list(f, state, chunks[0]);
    draw_topic_detail(f, state, chunks[1]);
}

fn draw_topic_list(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .topic_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let hz_str = state
                .topics
                .get(name)
                .map(|t| {
                    if t.hz > 0.5 {
                        format!("{:>5.0}Hz", t.hz)
                    } else if t.msg_count > 0 {
                        "latch".to_string()
                    } else {
                        "  -  ".to_string()
                    }
                })
                .unwrap_or_else(|| "  -  ".to_string());

            let marker = if i == state.topic_selected { "▶ " } else { "  " };
            let style = if i == state.topic_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(name, style),
                Span::styled(format!("  {hz_str}"), Style::default().fg(Color::DarkGray)),
            ]))
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
            .title(" TOPICS ")
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn draw_topic_detail(f: &mut Frame, state: &AppState, area: Rect) {
    let border_style = if state.active_pane == Pane::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let selected_topic = state
        .topic_names
        .get(state.topic_selected)
        .and_then(|name| state.topics.get(name));

    let (title, lines) = if let Some(topic) = selected_topic {
        let type_short = topic
            .info
            .type_name
            .rsplit('/')
            .next()
            .unwrap_or(&topic.info.type_name);
        let hz_str = if topic.hz > 0.5 {
            format!(" @ {:.0}Hz", topic.hz)
        } else {
            String::new()
        };
        let title = format!(" DETAIL: {} ", topic.info.name);

        let mut lines = vec![Line::from(vec![
            Span::styled(
                format!("{type_short}{hz_str}"),
                Style::default().fg(Color::DarkGray),
            ),
        ])];
        lines.push(Line::from(""));

        if let Some(ref data) = topic.latest {
            render_dynvalue(data, &mut lines, 0, &topic.info.name, state);
        }

        (title, lines)
    } else {
        (" DETAIL ".to_string(), vec![Line::from("No topic selected")])
    };

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    f.render_widget(paragraph, area);
}

fn render_dynvalue(
    value: &DynValue,
    lines: &mut Vec<Line<'static>>,
    indent: usize,
    path: &str,
    state: &AppState,
) {
    let pad = "  ".repeat(indent);
    match value {
        DynValue::Struct { type_name: _, fields } => {
            for (name, val) in fields {
                let field_path = format!("{path}.{name}");
                match val {
                    DynValue::Struct { .. } => {
                        let expanded = state
                            .tree_expanded
                            .get(&field_path)
                            .copied()
                            .unwrap_or(false);
                        let arrow = if expanded { "▼" } else { "▶" };
                        lines.push(Line::from(vec![
                            Span::raw(format!("{pad}  ")),
                            Span::styled(
                                format!("{arrow} "),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(name.clone(), Style::default().fg(Color::White)),
                        ]));
                        if expanded {
                            render_dynvalue(val, lines, indent + 2, &field_path, state);
                        }
                    }
                    DynValue::Array(arr) if arr.iter().any(|v| matches!(v, DynValue::Struct { .. })) => {
                        let expanded = state
                            .tree_expanded
                            .get(&field_path)
                            .copied()
                            .unwrap_or(false);
                        let arrow = if expanded { "▼" } else { "▶" };
                        lines.push(Line::from(vec![
                            Span::raw(format!("{pad}  ")),
                            Span::styled(
                                format!("{arrow} "),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(
                                format!("{name} [{} items]", arr.len()),
                                Style::default().fg(Color::White),
                            ),
                        ]));
                        if expanded {
                            for (i, item) in arr.iter().enumerate() {
                                let item_path = format!("{field_path}[{i}]");
                                render_dynvalue(item, lines, indent + 2, &item_path, state);
                            }
                        }
                    }
                    _ => {
                        lines.push(Line::from(vec![
                            Span::raw(format!("{pad}    ")),
                            Span::styled(
                                format!("{name}: "),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format_value(val),
                                Style::default().fg(Color::Green),
                            ),
                        ]));
                    }
                }
            }
        }
        _ => {
            lines.push(Line::from(vec![
                Span::raw(format!("{pad}  ")),
                Span::styled(format_value(value), Style::default().fg(Color::Green)),
            ]));
        }
    }
}

fn format_value(value: &DynValue) -> String {
    match value {
        DynValue::Bool(b) => b.to_string(),
        DynValue::I8(v) => v.to_string(),
        DynValue::I16(v) => v.to_string(),
        DynValue::I32(v) => v.to_string(),
        DynValue::I64(v) => v.to_string(),
        DynValue::U8(v) => v.to_string(),
        DynValue::U16(v) => v.to_string(),
        DynValue::U32(v) => v.to_string(),
        DynValue::U64(v) => v.to_string(),
        DynValue::F32(v) => format!("{v:.4}"),
        DynValue::F64(v) => format!("{v:.4}"),
        DynValue::String(s) => {
            if s.len() > 80 {
                format!("{}...", &s[..77])
            } else {
                s.clone()
            }
        }
        DynValue::Bytes(b) => format!("[{} bytes]", b.len()),
        DynValue::Array(arr) => {
            if arr.len() <= 6 {
                let items: Vec<String> = arr.iter().map(format_value).collect();
                format!("[{}]", items.join(", "))
            } else {
                format!("[{} items]", arr.len())
            }
        }
        DynValue::Struct { type_name, .. } => format!("{{{type_name}}}"),
    }
}
