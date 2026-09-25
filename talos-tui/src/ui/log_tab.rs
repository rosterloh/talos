use super::style;
use crate::state::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Cell, Paragraph, Row, Table},
};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let [body, filter] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    let mode = if state.log_live {
        "LIVE"
    } else {
        "PAUSED · Space resume"
    };
    let filtered = state.filtered_log_entries();
    if state.log_expanded {
        let lines = state
            .log_inspected
            .as_ref()
            .map(|e| {
                let mut lines = vec![
                    Line::from(format!("{} · {} · {}", e.timestamp, e.level, e.node)),
                    Line::from(""),
                ];
                lines.extend(e.message.lines().map(|s| Line::from(s.to_string())));
                lines
            })
            .unwrap_or_else(|| vec![Line::from("No matching entries")]);
        super::scroll_text(
            f,
            state,
            "log-message",
            format!("LOG {mode} · Enter close"),
            lines,
            body,
            true,
        );
    } else {
        let time = area.width >= 80;
        let node = area.width >= 55;
        let rows: Vec<_> = filtered
            .iter()
            .map(|entry| {
                let mut cells = Vec::new();
                if time {
                    cells.push(Cell::from(entry.timestamp.clone()).style(style::SECONDARY));
                }
                cells.push(
                    Cell::from(entry.level.clone()).style(match entry.level.as_str() {
                        "WARN" => style::WARNING,
                        "ERROR" | "FATAL" => style::ERROR,
                        "DEBUG" => style::SECONDARY,
                        _ => style::PRIMARY,
                    }),
                );
                if node {
                    cells.push(Cell::from(entry.node.clone()).style(style::SECONDARY));
                }
                cells.push(Cell::from(entry.message.replace(['\n', '\r'], " ")));
                Row::new(cells)
            })
            .collect();
        let mut widths = Vec::new();
        let mut header = Vec::new();
        if time {
            widths.push(Constraint::Length(12));
            header.push("Time");
        }
        widths.push(Constraint::Length(5));
        header.push("Level");
        if node {
            widths.push(Constraint::Length((area.width / 5).min(24)));
            header.push("Node");
        }
        widths.push(Constraint::Min(1));
        header.push("Message");
        let table = Table::new(rows, widths)
            .header(Row::new(header).style(style::SECONDARY))
            .block(style::pane(format!("LOG {mode}"), true));
        super::table(f, state, "logs", table, body, state.log_selected, true);
    }
    f.render_widget(
        Paragraph::new(format!(
            " {} · {} entries · /{}",
            state.log_severity_filter.label(),
            filtered.len(),
            state.log_search
        ))
        .style(style::SECONDARY),
        filter,
    );
}
