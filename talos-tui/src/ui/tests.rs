use super::*;
use crate::state::{JointData, LogEntry};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use talos_common::protocol::types::{
    JointInfo, JointLimits, JointType, NodeInfo, ParamInfo, ParamValue,
};

fn render(state: &AppState, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| draw(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer) -> String {
    if buffer.area.width == 0 {
        return String::new();
    }
    buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn press(state: &mut AppState, key: KeyCode) {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    crate::input::handle_key_event(state, &tx, KeyEvent::from(key));
}

fn node(name: &str, namespace: &str) -> NodeInfo {
    NodeInfo {
        name: name.into(),
        namespace: namespace.into(),
        publishers: (0..40).map(|i| format!("/topic_{i}")).collect(),
        subscribers: vec![],
        services: vec![],
    }
}

fn joint(position: Option<f64>) -> JointData {
    JointData {
        info: JointInfo {
            name: "slide".into(),
            joint_type: JointType::Prismatic,
            parent_link: "base".into(),
            child_link: "tip".into(),
            limits: Some(JointLimits {
                lower: -1.0,
                upper: 1.0,
                effort: 5.0,
                velocity: 2.0,
            }),
        },
        position,
        velocity: None,
        effort: None,
    }
}

#[test]
fn every_tab_and_help_tolerate_narrow_and_zero_sized_viewports() {
    let mut state = AppState::default();
    for (w, h) in [(0, 0), (1, 1), (20, 6), (40, 10), (80, 24), (140, 40)] {
        for tab in Tab::ALL {
            state.active_tab = tab;
            for pane in [Pane::Left, Pane::Right] {
                state.active_pane = pane;
                for help in [false, true] {
                    state.show_help = help;
                    render(&state, w, h);
                }
            }
        }
    }
}

#[test]
fn all_five_tabs_fit_in_a_narrow_header() {
    let state = AppState {
        active_tab: Tab::Params,
        ..Default::default()
    };
    let screen = text(&render(&state, 40, 10));
    assert!(screen.lines().next().unwrap().contains("5Prm"), "{screen}");
    let screen = text(&render(&state, 20, 6));
    for number in ['1', '2', '3', '4', '5'] {
        assert!(screen.lines().next().unwrap().contains(number), "{screen}");
    }
}

#[test]
fn narrow_nodes_show_namespace_and_switch_to_scrollable_detail() {
    let mut state = AppState {
        active_tab: Tab::Nodes,
        nodes: vec![node("controller", "/left"), node("controller", "/right")],
        ..Default::default()
    };
    let screen = text(&render(&state, 40, 10));
    assert!(
        screen.contains("/left/controller") && screen.contains("/right/controller"),
        "{screen}"
    );
    press(&mut state, KeyCode::Down);
    press(&mut state, KeyCode::Enter);
    let screen = text(&render(&state, 40, 10));
    assert!(
        screen.contains("NODE /right/controller") && screen.contains("Publishers (40)"),
        "{screen}"
    );
    press(&mut state, KeyCode::PageDown);
    let screen = text(&render(&state, 40, 10));
    assert!(screen.contains("/topic_10"), "{screen}");
    let offset = state.scroll.borrow()[&state.node_scroll_key()];
    press(&mut state, KeyCode::Esc);
    render(&state, 120, 10);
    press(&mut state, KeyCode::Enter);
    render(&state, 40, 10);
    assert_eq!(state.node_selected, 1);
    assert_eq!(state.scroll.borrow()[&state.node_scroll_key()], offset);
}

#[test]
fn list_viewport_survives_focus_changes_and_resize() {
    let mut state = AppState {
        active_tab: Tab::Nodes,
        nodes: (0..50).map(|i| node(&format!("n{i}"), "/")).collect(),
        node_selected: 20,
        ..Default::default()
    };
    render(&state, 120, 12);
    let offset = state.lists.borrow()["nodes"].offset();
    assert!(offset > 0);
    press(&mut state, KeyCode::Tab);
    render(&state, 40, 12);
    press(&mut state, KeyCode::Esc);
    render(&state, 120, 12);
    assert_eq!(state.lists.borrow()["nodes"].offset(), offset);
    assert_eq!(state.node_selected, 20);
}

#[test]
fn debug_selection_has_contrast_and_monochrome_marker() {
    let mut state = AppState {
        active_tab: Tab::Log,
        no_color: false,
        ..Default::default()
    };
    state.log_entries.push_back(LogEntry {
        timestamp: "12:00:00.000".into(),
        level: "DEBUG".into(),
        node: "n".into(),
        message: "inspect me".into(),
    });
    let buffer = render(&state, 100, 12);
    let screen = text(&buffer);
    let row = screen.lines().position(|s| s.contains("DEBUG")).unwrap();
    let col = screen.lines().nth(row).unwrap().find("DEBUG").unwrap();
    let cell = &buffer[(col as u16, row as u16)];
    assert_eq!(cell.fg, Color::White);
    assert_eq!(cell.bg, Color::DarkGray);
    assert_ne!(cell.fg, cell.bg);
    assert!(screen.contains("LIVE"));
    state.no_color = true;
    let buffer = render(&state, 100, 12);
    assert!(text(&buffer).contains("> "));
    assert!(
        buffer[(col as u16, row as u16)]
            .modifier
            .contains(Modifier::REVERSED)
    );
    assert!(
        buffer
            .content
            .iter()
            .all(|c| c.fg == Color::Reset && c.bg == Color::Reset)
    );
}

#[test]
fn expanded_logs_wrap_scroll_and_resume() {
    let mut state = AppState {
        active_tab: Tab::Log,
        ..Default::default()
    };
    state.log_entries.push_back(LogEntry {
        timestamp: "now".into(),
        level: "INFO".into(),
        node: "n".into(),
        message: (0..30).map(|i| format!("message line {i}\n")).collect(),
    });
    press(&mut state, KeyCode::Enter);
    let screen = text(&render(&state, 40, 10));
    assert!(
        screen.contains("PAUSED") && screen.contains("message line 0"),
        "{screen}"
    );
    press(&mut state, KeyCode::PageDown);
    assert!(text(&render(&state, 40, 10)).contains("message line 10"));
    press(&mut state, KeyCode::Char(' '));
    assert!(state.log_live && !state.log_expanded);
}

#[test]
fn absent_joint_telemetry_never_renders_zero_or_target_as_measurement() {
    let mut state = AppState {
        active_tab: Tab::Joints,
        active_pane: Pane::Right,
        joints: vec![joint(None)],
        ..Default::default()
    };
    state.joint_targets.insert("slide".into(), 0.5);
    let screen = text(&render(&state, 50, 24));
    assert!(screen.contains("Measured: No telemetry"), "{screen}");
    assert!(screen.contains("Requested target: 0.5000 m"));
    assert!(screen.contains("Limits: -1.0000 … 1.0000 m"));
    assert!(!screen.contains("Measured 0.0000"));
    state.joints[0].position = Some(f64::NAN);
    assert!(text(&render(&state, 50, 24)).contains("No telemetry"));
    state.joints[0].position = Some(0.25);
    let screen = text(&render(&state, 50, 24));
    assert!(screen.contains("Measured: 0.2500 m"));
    assert!(screen.find("Measured:").unwrap() < screen.find("Parent:").unwrap());
}

#[test]
fn parameter_editor_keeps_type_current_input_and_failure_visible() {
    let state = AppState {
        active_tab: Tab::Params,
        active_pane: Pane::Right,
        editing_param: true,
        param_node: Some("/node".into()),
        parameters: vec![ParamInfo {
            name: "gain".into(),
            value: ParamValue::Double(1.0),
        }],
        param_input: "2.0".to_string().into(),
        param_status: Some("set 'gain' rejected: read-only".into()),
        ..Default::default()
    };
    let screen = text(&render(&state, 40, 12));
    for expected in [
        "(double) gain",
        "Current: 1.0",
        "New: 2.0",
        "rejected: read-only",
    ] {
        assert!(screen.contains(expected), "missing {expected}: {screen}");
    }
}

#[test]
fn contextual_help_scrolls_and_only_explicit_controls_close_it() {
    let mut state = AppState {
        active_tab: Tab::Joints,
        show_help: true,
        ..Default::default()
    };
    render(&state, 40, 10);
    press(&mut state, KeyCode::Char('4'));
    assert!(state.show_help);
    press(&mut state, KeyCode::PageDown);
    let screen = text(&render(&state, 40, 10));
    assert!(
        screen.contains("target") || screen.contains("telemetry"),
        "{screen}"
    );
    assert!(screen.contains("Esc/? close"));
    press(&mut state, KeyCode::Esc);
    assert!(!state.show_help);
}

#[test]
fn long_parameter_values_cannot_push_input_or_error_offscreen() {
    let state = AppState {
        active_tab: Tab::Params,
        active_pane: Pane::Right,
        editing_param: true,
        param_node: Some("/n".into()),
        parameters: vec![ParamInfo {
            name: "label".into(),
            value: ParamValue::String("界".repeat(100)),
        }],
        param_input: format!("{}END", "界".repeat(100)).into(),
        param_status: Some("error: read-only".into()),
        ..Default::default()
    };
    let screen = text(&render(&state, 40, 10));
    for expected in ["(string) label", "Current:", "END", "error: read-only"] {
        assert!(screen.contains(expected), "missing {expected}: {screen}");
    }
}

#[test]
fn parameter_arrows_select_rows_and_page_keys_scroll_status() {
    let mut state = AppState {
        active_tab: Tab::Params,
        active_pane: Pane::Right,
        parameters: ["a", "b"]
            .into_iter()
            .map(|name| ParamInfo {
                name: name.into(),
                value: ParamValue::Bool(false),
            })
            .collect(),
        ..Default::default()
    };
    press(&mut state, KeyCode::Down);
    assert_eq!(state.param_selected, 1);
    press(&mut state, KeyCode::Up);
    assert_eq!(state.param_selected, 0);
    press(&mut state, KeyCode::PageDown);
    assert_eq!(state.scroll.borrow()["param-status"], 10);
}

#[test]
fn expanded_log_snapshot_survives_ring_eviction() {
    use talos_common::protocol::types::DynValue;
    let mut state = AppState {
        active_tab: Tab::Log,
        log_max_entries: 1,
        ..Default::default()
    };
    let log = |msg: &str| DynValue::Struct {
        type_name: "Log".into(),
        fields: vec![
            ("level".into(), DynValue::String("DEBUG".into())),
            ("msg".into(), DynValue::String(msg.into())),
        ],
    };
    state.handle_topic_data("/rosout".into(), "Log".into(), log("original"));
    press(&mut state, KeyCode::Enter);
    state.handle_topic_data("/rosout".into(), "Log".into(), log("replacement"));
    let screen = text(&render(&state, 40, 10));
    assert!(screen.contains("original") && !screen.contains("replacement"));
    press(&mut state, KeyCode::Char(' '));
    assert!(text(&render(&state, 40, 10)).contains("replacement"));
}
