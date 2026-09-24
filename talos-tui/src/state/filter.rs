use super::{AppState, Tab};

/// Single-line text buffer with a cursor, shared by every TUI prompt.
///
/// `cursor` is a byte offset that always sits on a char boundary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInput {
    text: String,
    cursor: usize,
}

impl From<String> for TextInput {
    fn from(text: String) -> Self {
        let cursor = text.len();
        Self { text, cursor }
    }
}

impl TextInput {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Text split at the cursor.
    pub fn split(&self) -> (&str, &str) {
        self.text.split_at(self.cursor)
    }

    pub fn insert(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if let Some(c) = self.text[..self.cursor].chars().next_back() {
            self.cursor -= c.len_utf8();
            self.text.remove(self.cursor);
        }
    }

    pub fn left(&mut self) {
        if let Some(c) = self.text[..self.cursor].chars().next_back() {
            self.cursor -= c.len_utf8();
        }
    }

    pub fn right(&mut self) {
        if let Some(c) = self.text[self.cursor..].chars().next() {
            self.cursor += c.len_utf8();
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }
}

/// The open `/` prompt: live text plus the filter to restore on Esc.
#[derive(Debug, Clone)]
pub struct FilterPrompt {
    pub input: TextInput,
    pub previous: String,
}

/// Case-insensitive substring match; an empty filter matches everything.
pub(crate) fn matches_filter(haystack: &str, filter: &str) -> bool {
    filter.is_empty() || haystack.to_lowercase().contains(&filter.to_lowercase())
}

pub(crate) fn clamp_selection(selected: &mut usize, len: usize) {
    *selected = (*selected).min(len.saturating_sub(1));
}

impl AppState {
    /// Filter string for the list on the active tab, if that tab has one.
    pub(crate) fn active_filter_mut(&mut self) -> Option<&mut String> {
        match self.active_tab {
            Tab::Topics => Some(&mut self.topic_filter),
            Tab::Nodes => Some(&mut self.node_filter),
            Tab::Log => Some(&mut self.log_search),
            Tab::Params => Some(&mut self.param_filter),
            Tab::Joints => None,
        }
    }

    /// Keep the active tab's selection inside its (possibly shrunk) filtered list.
    pub(crate) fn clamp_active_selection(&mut self) {
        match self.active_tab {
            Tab::Topics => {
                let len = self.filtered_topic_names().len();
                clamp_selection(&mut self.topic_selected, len);
            }
            Tab::Nodes => {
                let len = self.filtered_nodes().len();
                clamp_selection(&mut self.node_selected, len);
            }
            Tab::Log => self.clamp_log_selection(),
            Tab::Params => {
                let len = self.filtered_parameters().len();
                clamp_selection(&mut self.param_selected, len);
            }
            Tab::Joints => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_moves_and_edits_at_char_boundaries() {
        let mut input = TextInput::from("aé🙂".to_string());
        input.left();
        assert_eq!(input.split(), ("aé", "🙂"));
        input.backspace();
        assert_eq!(input.split(), ("a", "🙂"));
        input.insert('ß');
        assert_eq!(input.as_str(), "aß🙂");
        input.right();
        input.right(); // already at end
        assert_eq!(input.split(), ("aß🙂", ""));
        input.backspace();
        input.left();
        input.left();
        input.left(); // already at start
        input.backspace(); // no-op at start
        assert_eq!(input.split(), ("", "aß"));
        input.clear();
        assert_eq!(input, TextInput::default());
    }

    #[test]
    fn filter_is_case_insensitive_substring() {
        assert!(matches_filter("/Camera/Image", "camera/im"));
        assert!(matches_filter("anything", ""));
        assert!(!matches_filter("/lidar", "cam"));
    }

    #[test]
    fn selection_clamps_to_shrunk_list() {
        let mut selected = 5;
        clamp_selection(&mut selected, 3);
        assert_eq!(selected, 2);
        clamp_selection(&mut selected, 0);
        assert_eq!(selected, 0);
        selected = 1;
        clamp_selection(&mut selected, 3);
        assert_eq!(selected, 1);
    }
}
