use super::AppState;
use talos_common::protocol::types::DynValue;

pub struct TreeRow<'a> {
    pub path: String,
    pub parent: Option<String>,
    pub label: String,
    pub depth: usize,
    pub value: &'a DynValue,
    pub branch: bool,
    pub expanded: bool,
}

impl AppState {
    pub fn tree_rows(&self) -> Vec<TreeRow<'_>> {
        fn visit<'a>(
            state: &AppState,
            rows: &mut Vec<TreeRow<'a>>,
            value: &'a DynValue,
            path: String,
            parent: Option<String>,
            label: String,
            depth: usize,
        ) {
            let branch = matches!(value, DynValue::Struct { .. } | DynValue::Array(_));
            let expanded = state.tree_expanded.get(&path).copied().unwrap_or(false);
            rows.push(TreeRow {
                path: path.clone(),
                parent,
                label,
                depth,
                value,
                branch,
                expanded,
            });
            if !expanded {
                return;
            }
            match value {
                DynValue::Struct { fields, .. } => {
                    for (name, value) in fields {
                        visit(
                            state,
                            rows,
                            value,
                            format!("{path}.{name}"),
                            Some(path.clone()),
                            name.clone(),
                            depth + 1,
                        );
                    }
                }
                DynValue::Array(values) => {
                    for (i, value) in values.iter().enumerate() {
                        visit(
                            state,
                            rows,
                            value,
                            format!("{path}[{i}]"),
                            Some(path.clone()),
                            format!("[{i}]"),
                            depth + 1,
                        );
                    }
                }
                _ => {}
            }
        }
        let mut rows = Vec::new();
        if let Some(topic) = self.selected_topic_name()
            && let Some(data) = self.topics.get(&topic).and_then(|t| t.latest.as_ref())
        {
            if let DynValue::Struct { fields, .. } = data {
                for (name, value) in fields {
                    visit(
                        self,
                        &mut rows,
                        value,
                        format!("{topic}.{name}"),
                        None,
                        name.clone(),
                        0,
                    );
                }
            } else {
                visit(self, &mut rows, data, topic, None, "value".into(), 0);
            }
        }
        rows
    }

    pub fn tree_index(&self, rows: &[TreeRow<'_>]) -> usize {
        let selected = self
            .selected_topic_name()
            .and_then(|t| self.tree_selection.get(&t));
        selected
            .and_then(|path| {
                rows.iter().position(|r| &r.path == path).or_else(|| {
                    rows.iter().rposition(|r| {
                        path.starts_with(&format!("{}.", r.path))
                            || path.starts_with(&format!("{}[", r.path))
                    })
                })
            })
            .unwrap_or(0)
    }

    pub fn tree_move(&mut self, delta: isize) {
        let rows = self.tree_rows();
        let index = self
            .tree_index(&rows)
            .saturating_add_signed(delta)
            .min(rows.len().saturating_sub(1));
        let path = rows.get(index).map(|r| r.path.clone());
        if let (Some(topic), Some(path)) = (self.selected_topic_name(), path) {
            self.tree_selection.insert(topic, path);
        }
    }

    pub fn tree_toggle(&mut self, expand: Option<bool>) {
        let rows = self.tree_rows();
        let Some(row) = rows.get(self.tree_index(&rows)) else {
            return;
        };
        let (path, parent, branch, expanded) = (
            row.path.clone(),
            row.parent.clone(),
            row.branch,
            row.expanded,
        );
        if let Some(topic) = self.selected_topic_name() {
            self.tree_selection.insert(topic, path.clone());
        }
        if expand == Some(false) && !expanded {
            if let (Some(topic), Some(parent)) = (self.selected_topic_name(), parent) {
                self.tree_selection.insert(topic, parent);
            }
        } else if branch {
            self.tree_expanded.insert(path, expand.unwrap_or(!expanded));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talos_common::protocol::{messages::Response, types::TopicInfo};

    fn payload(extra: bool) -> DynValue {
        let mut fields = vec![(
            "items".into(),
            DynValue::Array(vec![DynValue::Struct {
                type_name: "Item".into(),
                fields: vec![("x".into(), DynValue::F64(1.0))],
            }]),
        )];
        if extra {
            fields.insert(0, ("new".into(), DynValue::Bool(true)));
        }
        DynValue::Struct {
            type_name: "Sample".into(),
            fields,
        }
    }

    #[test]
    fn nested_array_navigation_survives_payload_and_catalog_refresh() {
        let mut state = AppState::default();
        let info = TopicInfo {
            name: "/sample".into(),
            type_name: "Sample".into(),
            publisher_count: 1,
            subscriber_count: 0,
        };
        state.handle_response(Response::TopicList(vec![info.clone()]));
        state.handle_topic_data("/sample".into(), "Sample".into(), payload(false));
        state.tree_toggle(Some(true));
        assert_eq!(state.tree_rows().len(), 2);
        state.tree_move(1);
        state.tree_toggle(Some(true));
        state.tree_move(1);
        assert_eq!(state.tree_selection["/sample"], "/sample.items[0].x");
        state.handle_response(Response::TopicList(vec![info]));
        state.handle_topic_data("/sample".into(), "Sample".into(), payload(true));
        let rows = state.tree_rows();
        assert_eq!(rows[state.tree_index(&rows)].path, "/sample.items[0].x");
        state.tree_toggle(Some(false));
        assert_eq!(state.tree_selection["/sample"], "/sample.items[0]");
        state.tree_toggle(Some(false));
        assert_eq!(state.tree_rows().len(), 3);
        state.tree_toggle(None);
        assert_eq!(state.tree_rows().len(), 4);
        state.tree_move(99);
        assert_eq!(state.tree_selection["/sample"], "/sample.items[0].x");
        state.tree_move(-99);
        assert_eq!(state.tree_selection["/sample"], "/sample.new");
    }

    #[test]
    fn disappearing_array_element_falls_back_to_visible_parent() {
        let mut state = AppState::default();
        state.handle_topic_data("/sample".into(), "Sample".into(), payload(false));
        state.tree_toggle(Some(true));
        state.tree_move(1);
        state.tree_toggle(Some(true));
        state.tree_move(1);
        state.handle_topic_data(
            "/sample".into(),
            "Sample".into(),
            DynValue::Struct {
                type_name: "Sample".into(),
                fields: vec![("items".into(), DynValue::Array(vec![]))],
            },
        );
        let rows = state.tree_rows();
        assert_eq!(rows[state.tree_index(&rows)].path, "/sample.items");
    }
}
