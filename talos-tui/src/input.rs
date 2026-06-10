use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use talos_common::protocol::messages::Request;
use talos_common::protocol::types::{DynValue, ParamValue};
use tokio::sync::mpsc;

use crate::state::{AppState, JointFocus, LogLevel, Pane, Tab, node_fqn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

pub fn handle_key_event(
    state: &mut AppState,
    cmd_tx: &mpsc::UnboundedSender<Request>,
    key: KeyEvent,
) -> AppAction {
    if state.show_help {
        state.show_help = false;
        return AppAction::Continue;
    }

    if state.editing_joint {
        handle_joint_edit_key(state, cmd_tx, key);
        return AppAction::Continue;
    }

    if state.pose_confirming {
        handle_pose_confirmation_key(state, cmd_tx, key);
        return AppAction::Continue;
    }

    if state.editing_param {
        handle_param_edit_key(state, cmd_tx, key);
        return AppAction::Continue;
    }

    match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Char('?') => {
            state.show_help = true;
            AppAction::Continue
        }
        KeyCode::Char('1') => {
            state.active_tab = Tab::Topics;
            AppAction::Continue
        }
        KeyCode::Char('2') => {
            state.active_tab = Tab::Nodes;
            AppAction::Continue
        }
        KeyCode::Char('3') => {
            state.active_tab = Tab::Log;
            AppAction::Continue
        }
        KeyCode::Char('4') => {
            state.active_tab = Tab::Joints;
            AppAction::Continue
        }
        KeyCode::Char('5') => {
            state.active_tab = Tab::Params;
            AppAction::Continue
        }
        KeyCode::Tab => {
            state.active_pane = match state.active_pane {
                Pane::Left => Pane::Right,
                Pane::Right => Pane::Left,
            };
            AppAction::Continue
        }
        KeyCode::Up => {
            handle_up(state);
            AppAction::Continue
        }
        KeyCode::Down => {
            handle_down(state);
            AppAction::Continue
        }
        KeyCode::Left => {
            handle_left(state);
            AppAction::Continue
        }
        KeyCode::Right => {
            handle_right(state);
            AppAction::Continue
        }
        KeyCode::Enter if state.active_tab == Tab::Params => {
            load_params_for_selected(state, cmd_tx);
            AppAction::Continue
        }
        KeyCode::Enter => {
            handle_enter(state);
            AppAction::Continue
        }
        KeyCode::Char('f') if state.active_tab == Tab::Log => {
            cycle_log_severity_filter(state);
            AppAction::Continue
        }
        KeyCode::Char('s') if state.active_tab == Tab::Topics => {
            handle_topic_subscription_toggle(state, cmd_tx);
            AppAction::Continue
        }
        KeyCode::Char('j') if state.active_tab == Tab::Joints => {
            state.joint_focus = JointFocus::JointList;
            AppAction::Continue
        }
        KeyCode::Char('o') if state.active_tab == Tab::Joints => {
            state.joint_focus = JointFocus::PoseList;
            AppAction::Continue
        }
        KeyCode::Char('e')
            if state.active_tab == Tab::Joints && state.joint_focus == JointFocus::JointList =>
        {
            if !state.joints.is_empty() {
                state.editing_joint = true;
                state.joint_input.clear();
                state.joint_input_error = None;
            }
            AppAction::Continue
        }
        KeyCode::Char('x')
            if state.active_tab == Tab::Joints && state.joint_focus == JointFocus::PoseList =>
        {
            if !state.poses.is_empty() {
                state.pose_confirming = true;
            }
            AppAction::Continue
        }
        KeyCode::Char('e')
            if state.active_tab == Tab::Params
                && state.active_pane == Pane::Right
                && state.param_selected < state.parameters.len() =>
        {
            state.editing_param = true;
            state.param_input = state.parameters[state.param_selected].value.to_string();
            AppAction::Continue
        }
        _ => AppAction::Continue,
    }
}

fn handle_joint_edit_key(
    state: &mut AppState,
    cmd_tx: &mpsc::UnboundedSender<Request>,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Esc => {
            state.editing_joint = false;
            state.joint_input.clear();
            state.joint_input_error = None;
        }
        KeyCode::Enter => {
            handle_joint_input_submit(state, cmd_tx);
        }
        KeyCode::Backspace => {
            state.joint_input.pop();
            state.joint_input_error = None;
        }
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
            state.joint_input.push(c);
            state.joint_input_error = None;
        }
        _ => {}
    }
}

fn handle_pose_confirmation_key(
    state: &mut AppState,
    cmd_tx: &mpsc::UnboundedSender<Request>,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            handle_pose_confirm(state, cmd_tx);
        }
        _ => {
            state.pose_confirming = false;
        }
    }
}

fn handle_param_edit_key(
    state: &mut AppState,
    cmd_tx: &mpsc::UnboundedSender<Request>,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Esc => {
            state.editing_param = false;
            state.param_input.clear();
        }
        KeyCode::Enter => handle_param_input_submit(state, cmd_tx),
        KeyCode::Backspace => {
            state.param_input.pop();
        }
        KeyCode::Char(c) => state.param_input.push(c),
        _ => {}
    }
}

fn handle_topic_subscription_toggle(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    if let Some(toggle) = state.prepare_selected_topic_subscription_toggle()
        && cmd_tx.send(toggle.request.clone()).is_err()
    {
        state.revert_topic_subscription_toggle(toggle);
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
        Tab::Params => match state.active_pane {
            Pane::Left => {
                if state.param_node_selected > 0 {
                    state.param_node_selected -= 1;
                }
            }
            Pane::Right => {
                if state.param_selected > 0 {
                    state.param_selected -= 1;
                }
            }
        },
    }
}

fn handle_down(state: &mut AppState) {
    match state.active_tab {
        Tab::Topics => {
            if state.active_pane == Pane::Left && state.topic_selected + 1 < state.topic_names.len()
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
        Tab::Params => match state.active_pane {
            Pane::Left => {
                if state.param_node_selected + 1 < state.nodes.len() {
                    state.param_node_selected += 1;
                }
            }
            Pane::Right => {
                if state.param_selected + 1 < state.parameters.len() {
                    state.param_selected += 1;
                }
            }
        },
    }
}

fn handle_left(state: &mut AppState) {
    if state.active_tab == Tab::Topics && state.active_pane == Pane::Right {
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
        if let Some(topic_name) = state.topic_names.get(state.topic_selected)
            && let Some(topic_data) = state.topics.get(topic_name)
            && let Some(ref data) = topic_data.latest
        {
            expand_first_level(data, topic_name, &mut state.tree_expanded);
        }
    }
}

fn expand_first_level(value: &DynValue, path: &str, expanded: &mut HashMap<String, bool>) {
    if let DynValue::Struct { fields, .. } = value {
        for (name, val) in fields {
            let field_path = format!("{path}.{name}");
            if matches!(val, DynValue::Struct { .. }) {
                expanded.insert(field_path, true);
            }
        }
    }
}

fn handle_enter(state: &mut AppState) {
    if state.active_tab == Tab::Topics && state.active_pane == Pane::Right {
        if let Some(topic_name) = state.topic_names.get(state.topic_selected).cloned()
            && let Some(topic_data) = state.topics.get(&topic_name)
            && let Some(ref data) = topic_data.latest
        {
            toggle_first_level(data, &topic_name, &mut state.tree_expanded);
        }
    }
}

fn toggle_first_level(value: &DynValue, path: &str, expanded: &mut HashMap<String, bool>) {
    if let DynValue::Struct { fields, .. } = value {
        for (name, val) in fields {
            let field_path = format!("{path}.{name}");
            if matches!(val, DynValue::Struct { .. }) {
                let current = expanded.get(&field_path).copied().unwrap_or(false);
                expanded.insert(field_path, !current);
                return;
            }
        }
    }
}

fn load_params_for_selected(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    if state.active_pane != Pane::Left {
        return;
    }
    let Some(node) = state.nodes.get(state.param_node_selected) else {
        return;
    };
    let fqn = node_fqn(node);
    state.param_node = Some(fqn.clone());
    state.param_selected = 0;
    state.param_status = Some(format!("loading parameters for {fqn}..."));
    let _ = cmd_tx.send(Request::ListParameters { node: fqn });
}

fn handle_param_input_submit(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    let Some(node) = state.param_node.clone() else {
        state.editing_param = false;
        return;
    };
    let Some(param) = state.parameters.get(state.param_selected) else {
        state.editing_param = false;
        return;
    };
    let name = param.name.clone();
    let value = ParamValue::parse_preserving_type(&state.param_input, &param.value);
    let _ = cmd_tx.send(Request::SetParameter {
        node: node.clone(),
        name: name.clone(),
        value,
    });
    let _ = cmd_tx.send(Request::ListParameters { node });
    state.editing_param = false;
    state.param_input.clear();
    state.param_status = Some(format!("setting '{name}'..."));
}

fn cycle_log_severity_filter(state: &mut AppState) {
    let levels = LogLevel::ALL_LEVELS;
    let idx = levels
        .iter()
        .position(|l| *l == state.log_severity_filter)
        .unwrap_or(0);
    state.log_severity_filter = levels[(idx + 1) % levels.len()];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TopicSubscriptionState;
    use crossterm::event::KeyModifiers;
    use talos_common::protocol::messages::Response;
    use talos_common::protocol::types::{ParamInfo, ParamValue, TopicInfo, TopicSub};

    fn topic(name: &str, type_name: &str) -> TopicInfo {
        TopicInfo {
            name: name.into(),
            type_name: type_name.into(),
            publisher_count: 1,
            subscriber_count: 0,
        }
    }

    #[test]
    fn param_edit_preserves_existing_string_type() {
        let mut state = AppState::default();
        state.active_tab = Tab::Params;
        state.active_pane = Pane::Right;
        state.param_node = Some("/demo".into());
        state.parameters = vec![ParamInfo {
            name: "answer".into(),
            value: ParamValue::String("42".into()),
        }];
        state.editing_param = true;
        state.param_input = "42".into();

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        assert_eq!(
            handle_key_event(
                &mut state,
                &cmd_tx,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            ),
            AppAction::Continue
        );

        match cmd_rx.try_recv().expect("set request") {
            Request::SetParameter { node, name, value } => {
                assert_eq!(node, "/demo");
                assert_eq!(name, "answer");
                assert_eq!(value, ParamValue::String("42".into()));
            }
            other => panic!("unexpected request: {other:?}"),
        }
    }

    #[test]
    fn failed_send_reverts_topic_toggle_state() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));
        state.handle_response(Response::Subscribed {
            topics: vec![TopicSub {
                topic: "/camera".into(),
                type_name: "sensor_msgs/msg/Image".into(),
            }],
        });

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        drop(cmd_rx);

        assert_eq!(
            handle_key_event(
                &mut state,
                &cmd_tx,
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            ),
            AppAction::Continue
        );

        assert!(state.desired_subscriptions.contains("/camera"));
        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::Subscribed
        );
        assert_eq!(state.topics["/camera"].subscription_error, None);
    }
}
