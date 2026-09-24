use super::style;
use crate::state::{AppState, Pane, node_label};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Row, Table, Wrap},
};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let [left, right] = super::panes(area, state.active_pane);
    let focused = state.active_pane == Pane::Left;
    let items: Vec<_> = state
        .nodes
        .iter()
        .map(|n| ListItem::new(node_label(n)))
        .collect();
    super::list(
        f,
        state,
        "param-nodes",
        List::new(items).block(style::pane("NODES", focused)),
        left,
        state.param_node_selected,
        focused,
    );
    if right.is_empty() {
        return;
    }
    let editing = state.editing_param;
    let [body, footer] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(if editing { 7.min(right.height) } else { 2 }),
    ])
    .areas(right);
    let params = state.filtered_parameters();
    let title = super::filter_title(
        format!("PARAMETERS {}", state.param_node.as_deref().unwrap_or("")),
        &state.param_filter,
    );
    if state.parameters.is_empty() {
        f.render_widget(
            Paragraph::new(if state.param_awaiting_reply {
                "Loading…"
            } else if state.param_node.is_some() {
                "No parameters"
            } else {
                "Select node; Enter loads parameters"
            })
            .block(style::pane(title, !focused)),
            body,
        );
    } else {
        let rows: Vec<_> = params
            .iter()
            .map(|p| {
                Row::new(vec![
                    p.name.clone(),
                    p.value.type_name().into(),
                    p.value.to_string(),
                ])
            })
            .collect();
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(35),
                Constraint::Length((body.width / 5).clamp(4, 13)),
                Constraint::Min(1),
            ],
        )
        .header(Row::new(["Name", "Type", "Current value"]).style(style::SECONDARY))
        .block(style::pane(title, !focused));
        super::table(
            f,
            state,
            "parameters",
            table,
            body,
            state.param_selected,
            !focused,
        );
    }
    let block = style::pane(
        if editing {
            "EDIT · Enter apply / Esc close"
        } else {
            "STATUS"
        },
        editing,
    );
    let inner = block.inner(footer);
    f.render_widget(block, footer);
    let status_area = if editing {
        let [name, current, input, status] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);
        if let Some(p) = params.get(state.param_selected) {
            f.render_widget(
                Paragraph::new(format!("({}) {}", p.value.type_name(), p.name)),
                name,
            );
            f.render_widget(Paragraph::new(format!("Current: {}", p.value)), current);
        }
        let mut spans = vec![Span::styled("New: ", style::FOCUS)];
        spans.extend(super::input_spans(&state.param_input, style::PRIMARY));
        let cursor = 5 + Span::raw(state.param_input.split().0).width();
        let offset = cursor
            .saturating_sub(input.width.saturating_sub(1) as usize)
            .min(u16::MAX as usize) as u16;
        f.render_widget(Paragraph::new(Line::from(spans)).scroll((0, offset)), input);
        status
    } else {
        inner
    };
    if let Some(status) = &state.param_status {
        let paragraph = Paragraph::new(status.as_str())
            .style(style::status(status))
            .wrap(Wrap { trim: false });
        let max = paragraph
            .line_count(status_area.width)
            .saturating_sub(status_area.height as usize)
            .min(u16::MAX as usize) as u16;
        let mut offsets = state.scroll.borrow_mut();
        let offset = offsets.entry("param-status".into()).or_default();
        *offset = (*offset).min(max);
        f.render_widget(paragraph.scroll((*offset, 0)), status_area);
    }
}
