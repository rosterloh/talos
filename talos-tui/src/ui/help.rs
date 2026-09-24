use super::style;
use crate::state::{AppState, Tab};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Clear, Paragraph},
};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    f.render_widget(Clear, area);
    let [body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    let mut lines = vec![
        Line::from("1–5  Switch tab"),
        Line::from("Tab / Shift-Tab  Switch pane; Esc returns to list"),
        Line::from("Up / Down  Select or scroll the focused pane"),
        Line::from("PageUp / PageDown  Scroll details or help"),
        Line::from("r  Refresh topic and node lists"),
        Line::from("q / Ctrl-C  Quit (outside editors)"),
        Line::from(""),
    ];
    let actions: &[&str] = match state.active_tab {
        Tab::Topics => &[
            "Enter  Open payload / toggle selected branch",
            "Left / Right  Collapse / expand branch; Left on a leaf selects its parent",
            "Up / Down  Select visible fields, including array elements",
            "i  Toggle endpoint and QoS view; Up / Down scroll it",
            "s  Toggle subscription (pending and errors remain visible)",
            "/  Filter topic names",
        ],
        Tab::Nodes => &[
            "Enter  Open selected node",
            "Up / Down  Scroll publishers, subscribers and services",
            "l  Load logger level",
            "L  Cycle logger level",
            "/  Filter names and namespaces",
        ],
        Tab::Log => &[
            "Up / Down  Pause and inspect entries (newest first)",
            "Enter  Open / close full message",
            "Up / Down or PageUp / PageDown  Scroll full message",
            "Space  Pause / resume LIVE at newest entry",
            "f  Cycle severity",
            "/  Search message text",
            "PAUSED preserves selection as entries arrive; oldest entries eventually expire",
        ],
        Tab::Joints => &[
            "j / o  Select joints / poses",
            "Tab  Move between list and operational details",
            "e  Edit requested target; Enter sends, Esc cancels",
            "Targets are requests, not measured telemetry",
            "Limits clamp commands; absent telemetry is never zero",
            "x  Execute selected pose; y / Enter confirms, Esc cancels",
            "Published means agent accepted publication, not physical completion",
        ],
        Tab::Params => &[
            "Enter on node  Load parameters and open detail pane",
            "e  Edit selected parameter; type and current value stay visible",
            "Enter  Submit; duplicate submissions blocked while pending",
            "Rejected values remain in the editor for correction",
            "PageUp / PageDown  Scroll long reply/error messages in the editor",
            "Esc  Close editor (does not cancel an already submitted request)",
            "/  Filter parameters",
        ],
    };
    lines.extend(actions.iter().map(|s| Line::from(*s)));
    lines.extend([
        Line::from(""),
        Line::from("Text input: Left / Right cursor, Backspace delete, Ctrl-U clear"),
        Line::from("Filter: Enter applies; Esc restores the previous filter"),
        Line::from("Narrow terminals show one pane; Tab switches and Esc returns"),
        Line::from("Focus: > heading and bold selection; inactive selection: ·"),
        Line::from("NO_COLOR retains focus markers and reverse selection"),
    ]);
    super::scroll_text(
        f,
        state,
        "help",
        format!("{} help", state.active_tab.label()),
        lines,
        body,
        true,
    );
    f.render_widget(
        Paragraph::new("Esc/? close · ↑↓/PgUp/PgDn scroll").style(style::SECONDARY),
        footer,
    );
}
