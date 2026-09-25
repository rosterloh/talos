mod help;
mod joints_tab;
mod log_tab;
mod nodes_tab;
mod params_tab;
pub(crate) mod style;
#[cfg(test)]
mod tests;
mod topics_tab;

use crate::state::{AppState, JointFocus, Pane, Tab, TextInput};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, Paragraph, Table, Tabs, Wrap};

pub fn draw(f: &mut Frame, state: &AppState) {
    let [header, content, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());
    let titles = Tab::ALL.iter().enumerate().map(|(i, t)| {
        if header.width < 40 {
            (i + 1).to_string()
        } else if header.width < 60 {
            format!("{}{}", i + 1, ["Top", "Nod", "Log", "Jnt", "Prm"][i])
        } else {
            format!("{} {}", i + 1, t.label())
        }
    });
    f.render_widget(
        Tabs::new(titles)
            .select(state.active_tab.index())
            .highlight_style(style::FOCUS.add_modifier(Modifier::REVERSED)),
        Rect {
            height: 1.min(header.height),
            ..header
        },
    );
    if header.height > 1 {
        let connection = if state.connected {
            "connected"
        } else {
            "disconnected"
        };
        let transport = state
            .transport_type
            .map(|t| format!(" · {t:?}"))
            .unwrap_or_default();
        f.render_widget(
            Paragraph::new(format!(
                " Talos · {connection}{transport} · {}",
                state.active_tab.label()
            ))
            .style(if state.connected {
                style::SECONDARY
            } else {
                style::WARNING
            }),
            Rect::new(header.x, header.y + 1, header.width, 1),
        );
    }
    match state.active_tab {
        Tab::Topics => topics_tab::draw(f, state, content),
        Tab::Nodes => nodes_tab::draw(f, state, content),
        Tab::Log => log_tab::draw(f, state, content),
        Tab::Joints => joints_tab::draw(f, state, content),
        Tab::Params => params_tab::draw(f, state, content),
    }
    draw_status_bar(f, state, footer);
    if state.show_help {
        help::draw(f, state, f.area());
    }
    if state.no_color {
        for cell in &mut f.buffer_mut().content {
            if cell.bg != Color::Reset {
                cell.modifier.insert(Modifier::REVERSED);
            }
            cell.fg = Color::Reset;
            cell.bg = Color::Reset;
        }
    }
}

/// Bounded navigator; narrow terminals show the focused pane at full width.
fn panes(area: Rect, pane: Pane) -> [Rect; 2] {
    if area.width < 90 {
        return match pane {
            Pane::Left => [area, Rect::default()],
            Pane::Right => [Rect::default(), area],
        };
    }
    Layout::horizontal([
        Constraint::Length((area.width / 3).clamp(28, 42)),
        Constraint::Min(0),
    ])
    .areas(area)
}

fn list(
    f: &mut Frame,
    state: &AppState,
    key: &str,
    list: List<'_>,
    area: Rect,
    selected: usize,
    focused: bool,
) {
    if area.is_empty() {
        return;
    }
    let mut states = state.lists.borrow_mut();
    let viewport = states.entry(key.into()).or_default();
    viewport.select(Some(selected));
    f.render_stateful_widget(
        list.highlight_style(style::selection(focused))
            .highlight_symbol(if focused { "> " } else { "· " }),
        area,
        viewport,
    );
}

fn table(
    f: &mut Frame,
    state: &AppState,
    key: &str,
    table: Table<'_>,
    area: Rect,
    selected: usize,
    focused: bool,
) {
    if area.is_empty() {
        return;
    }
    let mut states = state.tables.borrow_mut();
    let viewport = states.entry(key.into()).or_default();
    viewport.select(Some(selected));
    f.render_stateful_widget(
        table
            .row_highlight_style(style::selection(focused))
            .highlight_symbol(if focused { "> " } else { "· " }),
        area,
        viewport,
    );
}

fn scroll_text(
    f: &mut Frame,
    state: &AppState,
    key: &str,
    title: impl Into<String>,
    lines: Vec<Line<'_>>,
    area: Rect,
    focused: bool,
) {
    if area.is_empty() {
        return;
    }
    let block = style::pane(title, focused);
    let inner = block.inner(area);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let max = paragraph
        .line_count(inner.width)
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    let mut offsets = state.scroll.borrow_mut();
    let offset = offsets.entry(key.into()).or_default();
    *offset = (*offset).min(max);
    f.render_widget(paragraph.scroll((*offset, 0)).block(block), area);
}

pub(super) fn filter_title(title: String, filter: &str) -> String {
    if filter.is_empty() {
        title
    } else {
        format!("{title} [/{filter}]")
    }
}

pub(super) fn input_spans(input: &TextInput, style: Style) -> Vec<Span<'_>> {
    let (before, rest) = input.split();
    let mut chars = rest.chars();
    let cursor = chars.next().map_or(" ".to_string(), String::from);
    vec![
        Span::styled(before, style),
        Span::styled(cursor, style.add_modifier(Modifier::REVERSED)),
        Span::styled(chars.as_str(), style),
    ]
}

fn draw_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    if let Some(prompt) = &state.filter_prompt {
        let mut spans = vec![Span::styled("/", style::FOCUS)];
        spans.extend(input_spans(&prompt.input, style::PRIMARY));
        spans.push(Span::styled("  Enter apply · Esc cancel", style::SECONDARY));
        f.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }
    let hints = if state.editing_param || state.editing_joint {
        "Enter apply · Esc cancel"
    } else if state.pose_confirming {
        "Enter/y confirm · Esc cancel"
    } else {
        match state.active_tab {
            Tab::Topics if state.active_pane == Pane::Right => "↑↓ field · ←→ branch · i QoS",
            Tab::Topics => "↑↓ topic · Enter open · s sub · / filter",
            Tab::Nodes if state.active_pane == Pane::Right => "↑↓ scroll · l level · L change",
            Tab::Nodes => "↑↓ node · Enter open · / filter",
            Tab::Log if state.log_expanded => "↑↓ scroll · Enter close · Space resume",
            Tab::Log => "↑↓ inspect · Enter expand · Space live · f severity",
            Tab::Joints if state.joint_focus == JointFocus::PoseList => {
                "↑↓ pose · x execute · j joints"
            }
            Tab::Joints => "↑↓ joint/scroll · e target · o poses",
            Tab::Params if state.active_pane == Pane::Left => "↑↓ node · Enter load",
            Tab::Params => "↑↓ parameter · e edit · / filter",
        }
    };
    let text = if area.width < 50 {
        if state.editing_param || state.editing_joint || state.pose_confirming {
            hints.to_string()
        } else if state.active_tab == Tab::Log {
            "? help · Space live · q quit".to_string()
        } else {
            "? help · Tab pane · Esc back · q quit".to_string()
        }
    } else {
        let navigation = if state.active_tab == Tab::Log {
            ""
        } else if state.active_pane == Pane::Right {
            "Esc back · "
        } else {
            "Tab pane · "
        };
        format!("? help · q quit · {navigation}{hints}")
    };
    f.render_widget(Paragraph::new(text).style(style::SECONDARY), area);
}
