use crossterm::event::{KeyCode, KeyEvent};
use talos_common::protocol::messages::Request;
use talos_common::protocol::types::ParamValue;
use tokio::sync::mpsc;

use crate::state::{AppState, Pane, node_fqn};

pub(super) fn select_previous(state: &mut AppState) {
    match state.active_pane {
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
    }
}

pub(super) fn select_next(state: &mut AppState) {
    match state.active_pane {
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
    }
}

pub(super) fn begin_edit_selected(state: &mut AppState) {
    if state.active_pane == Pane::Right && state.param_selected < state.parameters.len() {
        state.editing_param = true;
        state.param_input = state.parameters[state.param_selected].value.to_string();
    }
}

pub(super) fn handle_edit_key(
    state: &mut AppState,
    cmd_tx: &mpsc::UnboundedSender<Request>,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Esc => {
            state.editing_param = false;
            state.param_input.clear();
        }
        KeyCode::Enter => handle_input_submit(state, cmd_tx),
        KeyCode::Backspace => {
            state.param_input.pop();
        }
        KeyCode::Char(c) => state.param_input.push(c),
        _ => {}
    }
}

pub(super) fn load_for_selected(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
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

fn handle_input_submit(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use talos_common::protocol::types::{ParamInfo, ParamValue};

    #[test]
    fn param_edit_preserves_existing_string_type() {
        let mut state = AppState::default();
        state.param_node = Some("/demo".into());
        state.parameters = vec![ParamInfo {
            name: "answer".into(),
            value: ParamValue::String("42".into()),
        }];
        state.editing_param = true;
        state.param_input = "42".into();

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        handle_edit_key(
            &mut state,
            &cmd_tx,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
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
}
