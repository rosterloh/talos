use crossterm::event::{KeyCode, KeyEvent};
use talos_common::protocol::messages::Request;
use talos_common::protocol::types::ParamValue;
use tokio::sync::mpsc;

use super::filter;
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
            if state.param_selected + 1 < state.filtered_parameters().len() {
                state.param_selected += 1;
            }
        }
    }
}

pub(super) fn begin_edit_selected(state: &mut AppState) {
    if state.active_pane != Pane::Right || state.param_awaiting_reply {
        return;
    }
    let Some(param) = state
        .filtered_parameters()
        .get(state.param_selected)
        .copied()
    else {
        return;
    };
    let target = (
        state.param_node.clone().unwrap_or_default(),
        param.name.clone(),
    );
    let value = param.value.to_string();
    let retry = state.param_edit_target.as_ref() == Some(&target)
        && state
            .param_status
            .as_ref()
            .is_some_and(|s| s.contains("error") || s.contains("rejected"));
    state.param_edit_target = Some(target);
    state.editing_param = true;
    if !retry {
        state.param_input = value.into();
        state.param_status = None;
    }
}

pub(super) fn handle_edit_key(
    state: &mut AppState,
    cmd_tx: &mpsc::UnboundedSender<Request>,
    key: KeyEvent,
) {
    if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
        state.scroll_by(
            "param-status",
            if key.code == KeyCode::PageUp { -1 } else { 1 },
        );
        return;
    }
    if state.param_awaiting_reply && key.code != KeyCode::Esc {
        return;
    }
    match key.code {
        KeyCode::Esc => {
            state.editing_param = false;
        }
        KeyCode::Enter => handle_input_submit(state, cmd_tx),
        _ => {
            filter::edit_text(&mut state.param_input, key);
        }
    }
}

pub(super) fn load_for_selected(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    if state.active_pane != Pane::Left || state.param_awaiting_reply {
        return;
    }
    let Some(node) = state.nodes.get(state.param_node_selected) else {
        return;
    };
    let fqn = node_fqn(node);
    state.param_node = Some(fqn.clone());
    state.parameters.clear();
    state.param_selected = 0;
    state.active_pane = Pane::Right;
    state.param_status = Some(format!("loading parameters for {fqn}..."));
    state.param_awaiting_reply = true;
    if cmd_tx.send(Request::ListParameters { node: fqn }).is_err() {
        state.param_awaiting_reply = false;
        state.param_status = Some("error: command channel closed".into());
    }
}

fn handle_input_submit(state: &mut AppState, cmd_tx: &mpsc::UnboundedSender<Request>) {
    let Some(node) = state.param_node.clone() else {
        state.editing_param = false;
        return;
    };
    let Some(param) = state
        .filtered_parameters()
        .get(state.param_selected)
        .copied()
    else {
        state.editing_param = false;
        return;
    };
    let name = param.name.clone();
    state.scroll.borrow_mut().insert("param-status".into(), 0);
    let value = ParamValue::parse_preserving_type(state.param_input.as_str(), &param.value);
    if value.type_name() != param.value.type_name() && !matches!(param.value, ParamValue::NotSet) {
        state.param_status = Some(format!("error: expected {}", param.value.type_name()));
        return;
    }
    if cmd_tx
        .send(Request::SetParameter {
            node: node.clone(),
            name: name.clone(),
            value,
        })
        .is_err()
    {
        state.param_status = Some("error: command channel closed".into());
        return;
    }
    let _ = cmd_tx.send(Request::ListParameters { node });
    state.param_status = Some(format!("setting '{name}'..."));
    state.param_awaiting_reply = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use talos_common::protocol::types::{ParamInfo, ParamValue};

    #[test]
    fn rejected_parameter_keeps_input_for_retry_and_blocks_duplicate_submit() {
        let mut state = AppState {
            active_pane: Pane::Right,
            param_node: Some("/node".into()),
            parameters: vec![ParamInfo {
                name: "gain".into(),
                value: ParamValue::Double(1.0),
            }],
            ..Default::default()
        };
        begin_edit_selected(&mut state);
        state.param_input = "2.5".to_string().into();
        let (tx, mut rx) = mpsc::unbounded_channel();
        handle_edit_key(&mut state, &tx, KeyEvent::from(KeyCode::Enter));
        assert!(state.editing_param && state.param_awaiting_reply);
        assert!(matches!(
            rx.try_recv(),
            Ok(Request::SetParameter {
                value: ParamValue::Double(2.5),
                ..
            })
        ));
        assert!(matches!(rx.try_recv(), Ok(Request::ListParameters { .. })));
        handle_edit_key(&mut state, &tx, KeyEvent::from(KeyCode::Enter));
        assert!(rx.try_recv().is_err());
        state.handle_parameter_set("gain".into(), false, "read-only".into());
        state.handle_parameters("/node".into(), state.parameters.clone());
        assert_eq!(state.param_input.as_str(), "2.5");
        assert!(state.editing_param && !state.param_awaiting_reply);
        assert!(state.param_status.as_ref().unwrap().contains("read-only"));
        handle_edit_key(&mut state, &tx, KeyEvent::from(KeyCode::Esc));
        begin_edit_selected(&mut state);
        assert_eq!(state.param_input.as_str(), "2.5");
        handle_edit_key(&mut state, &tx, KeyEvent::from(KeyCode::Enter));
        state.handle_parameter_set("gain".into(), true, String::new());
        assert!(!state.editing_param);
        assert!(state.param_input.as_str().is_empty());
    }

    #[test]
    fn invalid_parameter_type_and_closed_channel_keep_editor() {
        let mut state = AppState {
            active_pane: Pane::Right,
            param_node: Some("/node".into()),
            parameters: vec![ParamInfo {
                name: "gain".into(),
                value: ParamValue::Integer(1),
            }],
            ..Default::default()
        };
        begin_edit_selected(&mut state);
        state.param_input = "oops".to_string().into();
        let (tx, mut rx) = mpsc::unbounded_channel();
        handle_edit_key(&mut state, &tx, KeyEvent::from(KeyCode::Enter));
        assert_eq!(
            state.param_status.as_deref(),
            Some("error: expected integer")
        );
        assert!(rx.try_recv().is_err());
        state.param_input = "2".to_string().into();
        drop(rx);
        handle_edit_key(&mut state, &tx, KeyEvent::from(KeyCode::Enter));
        assert!(state.editing_param && !state.param_awaiting_reply);
        assert_eq!(state.param_input.as_str(), "2");
        assert_eq!(
            state.param_status.as_deref(),
            Some("error: command channel closed")
        );
    }

    #[test]
    fn param_edit_preserves_existing_string_type() {
        let mut state = AppState {
            param_node: Some("/demo".into()),
            parameters: vec![ParamInfo {
                name: "answer".into(),
                value: ParamValue::String("42".into()),
            }],
            editing_param: true,
            param_input: "42".to_string().into(),
            ..Default::default()
        };

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
