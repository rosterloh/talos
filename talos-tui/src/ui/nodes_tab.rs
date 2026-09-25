use super::style;
use crate::state::{AppState, Pane, node_fqn};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{List, ListItem},
};
use talos_common::protocol::types::logger_level_name;

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let [left, right] = super::panes(area, state.active_pane);
    let focused = state.active_pane == Pane::Left;
    let nodes = state.filtered_nodes();
    let items: Vec<_> = nodes
        .iter()
        .map(|node| ListItem::new(node_fqn(node)))
        .collect();
    super::list(
        f,
        state,
        "nodes",
        List::new(items).block(style::pane(
            super::filter_title("NODES".into(), &state.node_filter),
            focused,
        )),
        left,
        state.node_selected,
        focused,
    );
    let Some(node) = nodes.get(state.node_selected) else {
        super::scroll_text(
            f,
            state,
            "node:empty",
            "NODE",
            vec![Line::from("No node selected")],
            right,
            !focused,
        );
        return;
    };
    let fqn = node_fqn(node);
    let mut lines = vec![logger_line(state, &fqn)];
    for (label, items) in [
        ("Publishers", &node.publishers),
        ("Subscribers", &node.subscribers),
        ("Services", &node.services),
    ] {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!("{label} ({})", items.len()),
            style::SECONDARY,
        ));
        lines.extend(items.iter().map(|s| Line::from(s.as_str())));
        if items.is_empty() {
            lines.push(Line::styled("(none)", style::SECONDARY));
        }
    }
    super::scroll_text(
        f,
        state,
        &state.node_scroll_key(),
        format!("NODE {fqn}"),
        lines,
        right,
        !focused,
    );
}

fn logger_line(state: &AppState, fqn: &str) -> Line<'static> {
    if state.logger_node.as_deref() != Some(fqn) {
        return Line::styled("Logger: press l to load", style::SECONDARY);
    }
    let level = state
        .logger_level
        .map(|l| logger_level_name(l).map_or_else(|| l.to_string(), str::to_string))
        .unwrap_or_else(|| "loading…".into());
    let mut spans = vec![Span::raw(format!("Logger: {level}"))];
    if let Some(status) = &state.logger_status {
        spans.push(Span::styled(format!("  {status}"), style::status(status)));
    }
    Line::from(spans)
}
