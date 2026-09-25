use super::style;
use crate::state::{AppState, JointFocus, Pane};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Clear, Gauge, List, ListItem, Paragraph, Wrap},
};
use talos_common::protocol::types::JointType;

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let [left, right] = super::panes(area, state.active_pane);
    if !left.is_empty() {
        let [joints, poses] = if left.height < 12 {
            if state.joint_focus == JointFocus::JointList {
                [left, Rect::default()]
            } else {
                [Rect::default(), left]
            }
        } else {
            Layout::vertical([
                Constraint::Min(4),
                Constraint::Length((left.height / 3).min(state.poses.len() as u16 + 1).max(2)),
            ])
            .areas(left)
        };
        let focused = state.active_pane == Pane::Left && state.joint_focus == JointFocus::JointList;
        let items: Vec<_> = state
            .joints
            .iter()
            .map(|j| ListItem::new(j.info.name.clone()))
            .collect();
        super::list(
            f,
            state,
            "joints",
            List::new(items).block(style::pane("JOINTS · o poses", focused)),
            joints,
            state.joint_selected,
            focused,
        );
        let focused = state.active_pane == Pane::Left && state.joint_focus == JointFocus::PoseList;
        let items: Vec<_> = state
            .poses
            .iter()
            .map(|p| ListItem::new(p.name.clone()))
            .collect();
        super::list(
            f,
            state,
            "poses",
            List::new(items).block(style::pane("POSES · j joints", focused)),
            poses,
            state.pose_selected,
            focused,
        );
    }
    if !right.is_empty() {
        draw_detail(f, state, right);
    }
    if state.pose_confirming {
        f.render_widget(Clear, area);
        let name = state
            .poses
            .get(state.pose_selected)
            .map(|p| p.name.as_str())
            .unwrap_or("?");
        f.render_widget(
            Paragraph::new(format!(
                "Execute pose '{name}'?\n\nEnter/y confirms · Esc cancels"
            ))
            .wrap(Wrap { trim: false })
            .block(style::pane("CONFIRM POSE", true)),
            area,
        );
    }
}

fn draw_detail(f: &mut Frame, state: &AppState, area: Rect) {
    let focused = state.active_pane == Pane::Right;
    let Some(joint) = state.joints.get(state.joint_selected) else {
        f.render_widget(
            Paragraph::new(state.joint_status.as_deref().unwrap_or("No joint selected"))
                .style(
                    state
                        .joint_status
                        .as_deref()
                        .map(style::status)
                        .unwrap_or(style::PRIMARY),
                )
                .wrap(Wrap { trim: false })
                .block(style::pane("JOINT / COMMAND", focused)),
            area,
        );
        return;
    };
    let (unit, velocity_unit, effort_unit) = match joint.info.joint_type {
        JointType::Prismatic => ("m", "m/s", "N"),
        JointType::Revolute | JointType::Continuous => ("rad", "rad/s", "N·m"),
        _ => ("", "", ""),
    };
    let measured = joint.position.filter(|p| p.is_finite());
    let position = measured
        .map(|p| format!("{p:.4} {unit}"))
        .unwrap_or_else(|| "No telemetry".into());
    let mut lines = vec![Line::from(format!("Measured: {position}"))];
    if let Some(target) = state.joint_targets.get(&joint.info.name) {
        lines.push(Line::from(format!("Requested target: {target:.4} {unit}")));
    } else {
        lines.push(Line::styled("Requested target: —", style::SECONDARY));
    }
    if state.editing_joint {
        lines.push(Line::from(format!(
            "New target ({unit}): {}▏",
            state.joint_input
        )));
        if let Some(error) = &state.joint_input_error {
            lines.push(Line::styled(error.as_str(), style::ERROR));
        }
    }
    if let Some(status) = &state.joint_status {
        lines.push(Line::styled(status.as_str(), style::status(status)));
    }
    if let Some(limits) = &joint.info.limits {
        lines.push(Line::from(format!(
            "Limits: {:.4} … {:.4} {unit}",
            limits.lower, limits.upper
        )));
    } else {
        lines.push(Line::styled("Limits: unbounded", style::SECONDARY));
    }
    lines.push(Line::from(format!(
        "Velocity: {}",
        joint
            .velocity
            .filter(|v| v.is_finite())
            .map(|v| format!("{v:.4} {velocity_unit}"))
            .unwrap_or_else(|| "No telemetry".into())
    )));
    lines.push(Line::from(format!(
        "Effort: {}",
        joint
            .effort
            .filter(|v| v.is_finite())
            .map(|v| format!("{v:.4} {effort_unit}"))
            .unwrap_or_else(|| "No telemetry".into())
    )));
    lines.push(Line::from(""));
    lines.push(Line::styled(
        format!("Type: {:?}", joint.info.joint_type),
        style::SECONDARY,
    ));
    lines.push(Line::styled(
        format!("Parent: {}", joint.info.parent_link),
        style::SECONDARY,
    ));
    lines.push(Line::styled(
        format!("Child: {}", joint.info.child_link),
        style::SECONDARY,
    ));
    let ratio = measured
        .zip(joint.info.limits.as_ref())
        .and_then(|(pos, limits)| {
            let range = limits.upper - limits.lower;
            (range.is_finite() && range > 0.0 && limits.lower.is_finite())
                .then(|| ((pos - limits.lower) / range).clamp(0.0, 1.0))
        });
    let [body, gauge] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(if ratio.is_some() && area.height >= 12 {
            1
        } else {
            0
        }),
    ])
    .areas(area);
    super::scroll_text(
        f,
        state,
        "joint-detail",
        format!("JOINT {}", joint.info.name),
        lines,
        body,
        focused,
    );
    if let Some(ratio) = ratio {
        f.render_widget(
            Gauge::default()
                .ratio(ratio)
                .label(format!("Measured {position}"))
                .gauge_style(style::SECONDARY),
            gauge,
        );
    }
}
