use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};

use crate::state::{AppState, JointFocus, Pane};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    draw_left_pane(f, state, chunks[0]);
    draw_joint_detail(f, state, chunks[1]);

    if state.pose_confirming {
        draw_pose_confirm(f, state, area);
    }
}

fn draw_left_pane(f: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(state.poses.len() as u16 + 4),
        ])
        .split(area);

    draw_joint_list(f, state, chunks[0]);
    draw_pose_list(f, state, chunks[1]);
}

fn draw_joint_list(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .joints
        .iter()
        .enumerate()
        .map(|(i, joint)| {
            let selected = state.joint_focus == JointFocus::JointList && i == state.joint_selected;
            let marker = if selected { "▶ " } else { "  " };
            let pos_str = joint
                .position
                .map(|p| format!("{p:>8.4}"))
                .unwrap_or_else(|| "     N/A".to_string());
            let style = if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(format!("{:<18}", joint.info.name), style),
                Span::styled(pos_str, Style::default().fg(Color::Green)),
            ]))
        })
        .collect();

    let border_style = if state.active_pane == Pane::Left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let header = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("{:<18}", "JOINTS"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "     Pos",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(header)
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn draw_pose_list(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .poses
        .iter()
        .enumerate()
        .map(|(i, pose)| {
            let selected = state.joint_focus == JointFocus::PoseList && i == state.pose_selected;
            let marker = if selected { "▶ " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", pose.name),
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
            .title(" POSES ")
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn draw_joint_detail(f: &mut Frame, state: &AppState, area: Rect) {
    let border_style = if state.active_pane == Pane::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let joint = state.joints.get(state.joint_selected);

    let (title, content_lines, gauge_info) = if let Some(joint) = joint {
        let title = format!(" CONTROL: {} ", joint.info.name);
        let joint_type = format!("{:?}", joint.info.joint_type);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::DarkGray)),
                Span::raw(joint_type.clone()),
            ]),
            Line::from(vec![
                Span::styled("Parent: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&joint.info.parent_link),
            ]),
            Line::from(vec![
                Span::styled("Child: ", Style::default().fg(Color::DarkGray)),
                Span::raw(&joint.info.child_link),
            ]),
            Line::from(""),
        ];

        let gauge = if let Some(ref limits) = joint.info.limits {
            let pos = joint.position.unwrap_or(0.0);
            let range = limits.upper - limits.lower;
            let ratio = if range > 0.0 {
                ((pos - limits.lower) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };

            lines.push(Line::from(Span::styled(
                "Position:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )));

            Some((limits.lower, limits.upper, pos, ratio))
        } else {
            lines.push(Line::from(vec![
                Span::styled("Position: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    joint
                        .position
                        .map(|p| format!("{p:.4}"))
                        .unwrap_or_else(|| "N/A".into()),
                    Style::default().fg(Color::Green),
                ),
            ]));
            None
        };

        // Editing indicator
        if state.editing_joint {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Set position: ", Style::default().fg(Color::Yellow)),
                Span::styled(&state.joint_input, Style::default().fg(Color::White)),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]));
            if let Some(ref err) = state.joint_input_error {
                lines.push(Line::from(Span::styled(
                    err.as_str(),
                    Style::default().fg(Color::Red),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Velocity: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                joint
                    .velocity
                    .map(|v| format!("{v:.4}"))
                    .unwrap_or_else(|| "N/A".into()),
                Style::default().fg(Color::Green),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Effort:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                joint
                    .effort
                    .map(|e| format!("{e:.4}"))
                    .unwrap_or_else(|| "N/A".into()),
                Style::default().fg(Color::Green),
            ),
        ]));

        (title, lines, gauge)
    } else {
        (
            " CONTROL ".to_string(),
            vec![Line::from("No joint selected")],
            None,
        )
    };

    // Split the detail area for the gauge
    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Info lines
            Constraint::Length(3), // Gauge
            Constraint::Min(0),    // Remaining info
        ])
        .split(area);

    let info_top = Paragraph::new(content_lines[..content_lines.len().min(5)].to_vec()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );
    f.render_widget(info_top, detail_chunks[0]);

    if let Some((lower, upper, pos, ratio)) = gauge_info {
        let gauge_label = format!("{pos:.4}");
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_style(border_style),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .label(gauge_label)
            .ratio(ratio);
        f.render_widget(gauge, detail_chunks[1]);

        // Limits labels
        let limits_line = Line::from(vec![
            Span::styled(format!(" {lower:.2}"), Style::default().fg(Color::DarkGray)),
            Span::raw(" ".repeat(detail_chunks[1].width.saturating_sub(16).into())),
            Span::styled(format!("{upper:.2} "), Style::default().fg(Color::DarkGray)),
        ]);
        let limits_para = Paragraph::new(limits_line).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(border_style),
        );
        f.render_widget(
            limits_para,
            Rect {
                y: detail_chunks[1].y + detail_chunks[1].height,
                height: 1.min(detail_chunks[2].height),
                ..detail_chunks[1]
            },
        );
    }

    // Remaining content
    if content_lines.len() > 5 {
        let remaining = Paragraph::new(content_lines[5..].to_vec()).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(border_style),
        );
        f.render_widget(remaining, detail_chunks[2]);
    }
}

fn draw_pose_confirm(f: &mut Frame, state: &AppState, area: Rect) {
    if !state.pose_confirming {
        return;
    }
    let pose_name = state
        .poses
        .get(state.pose_selected)
        .map(|p| p.name.as_str())
        .unwrap_or("?");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Execute pose ", Style::default().fg(Color::Yellow)),
            Span::styled(
                pose_name,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [y/Enter] confirm  [any] cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let width = 44.min(area.width);
    let height = 6.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    let popup = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" CONFIRM ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(ratatui::widgets::Clear, popup_area);
    f.render_widget(popup, popup_area);
}
