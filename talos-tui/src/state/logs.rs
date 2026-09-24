use talos_common::protocol::types::DynValue;

use super::AppState;
use super::filter::matches_filter;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub node: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    All,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub const ALL_LEVELS: [LogLevel; 6] = [
        LogLevel::All,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
        LogLevel::Fatal,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            LogLevel::All => "ALL",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn matches(&self, level: &str) -> bool {
        match self {
            LogLevel::All => true,
            other => other.label() == level,
        }
    }
}

impl AppState {
    pub(crate) fn push_log_entry_from_data(&mut self, data: &DynValue) {
        if let Some(entry) = extract_log_entry(data) {
            // Newest entries go on top; keep the highlight on the same entry.
            if self.log_selected > 0 && self.log_entry_visible(&entry) {
                self.log_selected += 1;
            }
            self.log_entries.push_front(entry);
            while self.log_entries.len() > self.log_max_entries {
                self.log_entries.pop_back();
            }
        }
    }

    pub fn filtered_log_entries(&self) -> Vec<&LogEntry> {
        self.log_entries
            .iter()
            .filter(|e| self.log_entry_visible(e))
            .collect()
    }

    /// Keep the selection inside the filtered list after the filter changes.
    pub(crate) fn clamp_log_selection(&mut self) {
        let len = self.filtered_log_entries().len();
        self.log_selected = self.log_selected.min(len.saturating_sub(1));
    }

    fn log_entry_visible(&self, e: &LogEntry) -> bool {
        self.log_severity_filter.matches(&e.level)
            && (self.log_node_filter.is_empty() || e.node.contains(&self.log_node_filter))
            && matches_filter(&e.message, &self.log_search)
    }
}

fn extract_log_entry(data: &DynValue) -> Option<LogEntry> {
    if let DynValue::Struct { fields, .. } = data {
        let get_str = |name: &str| -> String {
            fields
                .iter()
                .find(|(k, _)| k == name)
                .and_then(|(_, v)| {
                    if let DynValue::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        };

        let timestamp = fields
            .iter()
            .find(|(k, _)| k == "stamp")
            .and_then(|(_, v)| {
                if let DynValue::Struct { fields, .. } = v {
                    let sec = fields.iter().find(|(k, _)| k == "sec").and_then(|(_, v)| {
                        if let DynValue::I32(s) = v {
                            Some(*s)
                        } else {
                            None
                        }
                    })?;
                    let nanosec =
                        fields
                            .iter()
                            .find(|(k, _)| k == "nanosec")
                            .and_then(|(_, v)| {
                                if let DynValue::U32(n) = v {
                                    Some(*n)
                                } else {
                                    None
                                }
                            })?;
                    let total_secs = sec as u64;
                    let hours = (total_secs / 3600) % 24;
                    let mins = (total_secs / 60) % 60;
                    let secs = total_secs % 60;
                    Some(format!(
                        "{hours:02}:{mins:02}:{secs:02}.{:03}",
                        nanosec / 1_000_000
                    ))
                } else {
                    None
                }
            })
            .unwrap_or_default();

        Some(LogEntry {
            timestamp,
            level: get_str("level"),
            node: get_str("name"),
            message: get_str("msg"),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(level: &str) -> DynValue {
        DynValue::Struct {
            type_name: "rcl_interfaces/msg/Log".into(),
            fields: vec![
                ("level".into(), DynValue::String(level.into())),
                ("name".into(), DynValue::String("n".into())),
                ("msg".into(), DynValue::String("m".into())),
            ],
        }
    }

    #[test]
    fn selection_follows_entry_as_new_logs_arrive() {
        let mut state = AppState::default();
        state.push_log_entry_from_data(&log("INFO"));
        state.push_log_entry_from_data(&log("WARN"));
        state.log_selected = 1; // the INFO entry
        state.push_log_entry_from_data(&log("ERROR"));
        assert_eq!(
            state.filtered_log_entries()[state.log_selected].level,
            "INFO"
        );
    }

    #[test]
    fn selection_is_clamped_when_filter_shrinks_list() {
        let mut state = AppState::default();
        for level in ["INFO", "INFO", "WARN"] {
            state.push_log_entry_from_data(&log(level));
        }
        state.log_selected = 2;
        state.log_severity_filter = LogLevel::Warn;
        state.clamp_log_selection();
        assert_eq!(state.log_selected, 0);
    }
}
