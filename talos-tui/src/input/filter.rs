use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::state::{AppState, FilterPrompt, TextInput};

/// Shared editing keys for every text prompt. Returns true if the text changed.
pub(super) fn edit_text(input: &mut TextInput, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Left => input.left(),
        KeyCode::Right => input.right(),
        KeyCode::Backspace => {
            input.backspace();
            return true;
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            input.clear();
            return true;
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            input.insert(c);
            return true;
        }
        _ => {}
    }
    false
}

/// `/`: open the filter prompt for the active tab's list, prefilled with the
/// current filter.
pub(super) fn open(state: &mut AppState) {
    if let Some(filter) = state.active_filter_mut() {
        let previous = filter.clone();
        state.filter_prompt = Some(FilterPrompt {
            input: previous.clone().into(),
            previous,
        });
    }
}

/// The filter applies live while typing; Esc restores the previous filter.
pub(super) fn handle_key(state: &mut AppState, key: KeyEvent) {
    let Some(prompt) = state.filter_prompt.as_mut() else {
        return;
    };
    let text = match key.code {
        KeyCode::Enter => {
            state.filter_prompt = None;
            return;
        }
        KeyCode::Esc => {
            let previous = std::mem::take(&mut prompt.previous);
            state.filter_prompt = None;
            previous
        }
        _ if edit_text(&mut prompt.input, key) => prompt.input.as_str().to_string(),
        _ => return,
    };
    if let Some(filter) = state.active_filter_mut() {
        *filter = text;
    }
    state.clamp_active_selection();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Tab;

    fn press(state: &mut AppState, code: KeyCode) {
        handle_key(state, KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn esc_restores_previous_filter_and_enter_keeps_edit() {
        let mut state = AppState {
            active_tab: Tab::Nodes,
            node_filter: "cam".into(),
            ..Default::default()
        };
        open(&mut state);
        press(&mut state, KeyCode::Char('x'));
        assert_eq!(state.node_filter, "camx");
        press(&mut state, KeyCode::Esc);
        assert_eq!(state.node_filter, "cam");
        assert!(state.filter_prompt.is_none());

        open(&mut state);
        handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        press(&mut state, KeyCode::Enter);
        assert_eq!(state.node_filter, "");
        assert!(state.filter_prompt.is_none());
    }
}
