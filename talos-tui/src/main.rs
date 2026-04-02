mod client;
mod state;
mod ui;

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use talos_common::protocol::messages::Request;
use tokio::sync::mpsc;

use state::*;

fn main() -> io::Result<()> {
    let socket_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/talos.sock".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = Arc::new(Mutex::new(AppState::default()));
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Request>();

    // Spawn IPC client on tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client_state = Arc::clone(&state);
    rt.spawn(async move {
        client::run(socket_path, client_state, cmd_rx).await;
    });

    let result = run_app(&mut terminal, &state, &cmd_tx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &Arc<Mutex<AppState>>,
    cmd_tx: &mpsc::UnboundedSender<Request>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(50); // ~20 FPS

    loop {
        {
            let s = state.lock().unwrap();
            terminal.draw(|f| ui::draw(f, &s))?;
        }

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let mut s = state.lock().unwrap();

                if s.show_help {
                    s.show_help = false;
                    continue;
                }

                // Joint position editing mode
                if s.editing_joint {
                    match key.code {
                        KeyCode::Esc => {
                            s.editing_joint = false;
                            s.joint_input.clear();
                            s.joint_input_error = None;
                        }
                        KeyCode::Enter => {
                            handle_joint_input_submit(&mut s, cmd_tx);
                        }
                        KeyCode::Backspace => {
                            s.joint_input.pop();
                            s.joint_input_error = None;
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                            s.joint_input.push(c);
                            s.joint_input_error = None;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Pose confirmation mode
                if s.pose_confirming {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Enter => {
                            handle_pose_confirm(&mut s, cmd_tx);
                        }
                        _ => {
                            s.pose_confirming = false;
                        }
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => s.show_help = true,

                    // Tab switching
                    KeyCode::Char('1') => s.active_tab = Tab::Topics,
                    KeyCode::Char('2') => s.active_tab = Tab::Nodes,
                    KeyCode::Char('3') => s.active_tab = Tab::Log,
                    KeyCode::Char('4') => s.active_tab = Tab::Joints,
                    KeyCode::Tab => {
                        s.active_pane = match s.active_pane {
                            Pane::Left => Pane::Right,
                            Pane::Right => Pane::Left,
                        };
                    }

                    // Navigation
                    KeyCode::Up => handle_up(&mut s),
                    KeyCode::Down => handle_down(&mut s),
                    KeyCode::Left => handle_left(&mut s),
                    KeyCode::Right => handle_right(&mut s),
                    KeyCode::Enter => handle_enter(&mut s),

                    // Log tab specific
                    KeyCode::Char('f') if s.active_tab == Tab::Log => {
                        let levels = LogLevel::ALL_LEVELS;
                        let idx = levels
                            .iter()
                            .position(|l| *l == s.log_severity_filter)
                            .unwrap_or(0);
                        s.log_severity_filter = levels[(idx + 1) % levels.len()];
                    }

                    // Joints tab specific
                    KeyCode::Char('j') if s.active_tab == Tab::Joints => {
                        s.joint_focus = JointFocus::JointList;
                    }
                    KeyCode::Char('o') if s.active_tab == Tab::Joints => {
                        s.joint_focus = JointFocus::PoseList;
                    }
                    KeyCode::Char('e')
                        if s.active_tab == Tab::Joints
                            && s.joint_focus == JointFocus::JointList =>
                    {
                        if !s.joints.is_empty() {
                            s.editing_joint = true;
                            s.joint_input.clear();
                            s.joint_input_error = None;
                        }
                    }
                    KeyCode::Char('x')
                        if s.active_tab == Tab::Joints
                            && s.joint_focus == JointFocus::PoseList =>
                    {
                        if !s.poses.is_empty() {
                            s.pose_confirming = true;
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

fn handle_joint_input_submit(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    let value: f64 = match state.joint_input.parse() {
        Ok(v) => v,
        Err(_) => {
            state.joint_input_error = Some("invalid number".into());
            return;
        }
    };

    if let Some(joint) = state.joints.get(state.joint_selected) {
        // Clamp to limits if available
        let clamped = if let Some(ref limits) = joint.info.limits {
            if value < limits.lower {
                state.joint_input_error =
                    Some(format!("clamped to lower limit {:.4}", limits.lower));
                limits.lower
            } else if value > limits.upper {
                state.joint_input_error =
                    Some(format!("clamped to upper limit {:.4}", limits.upper));
                limits.upper
            } else {
                value
            }
        } else {
            value
        };

        let _ = cmd_tx.send(Request::SetJointPosition {
            joint: joint.info.name.clone(),
            position: clamped,
        });

        state.editing_joint = false;
        state.joint_input.clear();
    }
}

fn handle_pose_confirm(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    if let Some(pose) = state.poses.get(state.pose_selected) {
        let _ = cmd_tx.send(Request::ExecutePose {
            name: pose.name.clone(),
        });
    }
    state.pose_confirming = false;
}

fn handle_up(state: &mut AppState) {
    match state.active_tab {
        Tab::Topics => {
            if state.active_pane == Pane::Left && state.topic_selected > 0 {
                state.topic_selected -= 1;
            }
        }
        Tab::Nodes => {
            if state.active_pane == Pane::Left && state.node_selected > 0 {
                state.node_selected -= 1;
            }
        }
        Tab::Log => {
            if state.log_selected > 0 {
                state.log_selected -= 1;
            }
        }
        Tab::Joints => match state.joint_focus {
            JointFocus::JointList => {
                if state.joint_selected > 0 {
                    state.joint_selected -= 1;
                }
            }
            JointFocus::PoseList => {
                if state.pose_selected > 0 {
                    state.pose_selected -= 1;
                }
            }
        },
    }
}

fn handle_down(state: &mut AppState) {
    match state.active_tab {
        Tab::Topics => {
            if state.active_pane == Pane::Left
                && state.topic_selected + 1 < state.topic_names.len()
            {
                state.topic_selected += 1;
            }
        }
        Tab::Nodes => {
            if state.active_pane == Pane::Left && state.node_selected + 1 < state.nodes.len() {
                state.node_selected += 1;
            }
        }
        Tab::Log => {
            let filtered_count = state.filtered_log_entries().len();
            if state.log_selected + 1 < filtered_count {
                state.log_selected += 1;
            }
        }
        Tab::Joints => match state.joint_focus {
            JointFocus::JointList => {
                if state.joint_selected + 1 < state.joints.len() {
                    state.joint_selected += 1;
                }
            }
            JointFocus::PoseList => {
                if state.pose_selected + 1 < state.poses.len() {
                    state.pose_selected += 1;
                }
            }
        },
    }
}

fn handle_left(state: &mut AppState) {
    if state.active_tab == Tab::Topics && state.active_pane == Pane::Right {
        // Collapse the currently focused tree node
        // For simplicity, collapse all at current topic level
        if let Some(topic_name) = state.topic_names.get(state.topic_selected) {
            let prefix = format!("{topic_name}.");
            let keys_to_collapse: Vec<String> = state
                .tree_expanded
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            for key in keys_to_collapse {
                state.tree_expanded.insert(key, false);
            }
        }
    }
}

fn handle_right(state: &mut AppState) {
    if state.active_tab == Tab::Topics && state.active_pane == Pane::Right {
        // Expand all at current topic level
        if let Some(topic_name) = state.topic_names.get(state.topic_selected) {
            if let Some(topic_data) = state.topics.get(topic_name) {
                if let Some(ref data) = topic_data.latest {
                    expand_first_level(data, topic_name, &mut state.tree_expanded);
                }
            }
        }
    }
}

fn expand_first_level(
    value: &talos_common::protocol::types::DynValue,
    path: &str,
    expanded: &mut std::collections::HashMap<String, bool>,
) {
    if let talos_common::protocol::types::DynValue::Struct { fields, .. } = value {
        for (name, val) in fields {
            let field_path = format!("{path}.{name}");
            if matches!(
                val,
                talos_common::protocol::types::DynValue::Struct { .. }
            ) {
                expanded.insert(field_path, true);
            }
        }
    }
}

fn handle_enter(state: &mut AppState) {
    if state.active_tab == Tab::Topics && state.active_pane == Pane::Right {
        // Toggle expand on the selected topic's first-level struct fields
        if let Some(topic_name) = state.topic_names.get(state.topic_selected).cloned() {
            if let Some(topic_data) = state.topics.get(&topic_name) {
                if let Some(ref data) = topic_data.latest {
                    toggle_first_level(data, &topic_name, &mut state.tree_expanded);
                }
            }
        }
    }
}

fn toggle_first_level(
    value: &talos_common::protocol::types::DynValue,
    path: &str,
    expanded: &mut std::collections::HashMap<String, bool>,
) {
    if let talos_common::protocol::types::DynValue::Struct { fields, .. } = value {
        // Find the first collapsed struct field and toggle it
        for (name, val) in fields {
            let field_path = format!("{path}.{name}");
            if matches!(
                val,
                talos_common::protocol::types::DynValue::Struct { .. }
            ) {
                let current = expanded.get(&field_path).copied().unwrap_or(false);
                expanded.insert(field_path, !current);
                return;
            }
        }
    }
}
