use talos_common::protocol::types::DynValue;

use super::AppState;

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
            self.log_entries.push_front(entry);
            while self.log_entries.len() > self.log_max_entries {
                self.log_entries.pop_back();
            }
        }
    }

    pub fn filtered_log_entries(&self) -> Vec<&LogEntry> {
        self.log_entries
            .iter()
            .filter(|e| self.log_severity_filter.matches(&e.level))
            .filter(|e| self.log_node_filter.is_empty() || e.node.contains(&self.log_node_filter))
            .filter(|e| self.log_search.is_empty() || e.message.contains(&self.log_search))
            .collect()
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
