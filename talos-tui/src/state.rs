use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::{DynValue, JointInfo, NodeInfo, PoseInfo, TopicInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Topics,
    Nodes,
    Log,
    Joints,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Topics, Tab::Nodes, Tab::Log, Tab::Joints];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Topics => "Topics",
            Tab::Nodes => "Nodes",
            Tab::Log => "Log",
            Tab::Joints => "Joints",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Topics => 0,
            Tab::Nodes => 1,
            Tab::Log => 2,
            Tab::Joints => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct TopicData {
    pub info: TopicInfo,
    pub latest: Option<DynValue>,
    pub last_received: Option<Instant>,
    pub msg_count: u64,
    pub hz: f64,
    pub subscription: TopicSubscriptionState,
    pub subscription_error: Option<String>,
}

impl TopicData {
    fn placeholder(name: &str) -> Self {
        Self {
            info: TopicInfo {
                name: name.to_string(),
                type_name: String::new(),
                publisher_count: 0,
                subscriber_count: 0,
            },
            latest: None,
            last_received: None,
            msg_count: 0,
            hz: 0.0,
            subscription: TopicSubscriptionState::Unsubscribed,
            subscription_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopicSubscriptionState {
    Subscribed,
    Unsubscribed,
    PendingSubscribe,
    PendingUnsubscribe,
    Error,
}

impl TopicSubscriptionState {
    pub fn label(self) -> &'static str {
        match self {
            TopicSubscriptionState::Subscribed => "subscribed",
            TopicSubscriptionState::Unsubscribed => "unsubscribed",
            TopicSubscriptionState::PendingSubscribe => "pending subscribe",
            TopicSubscriptionState::PendingUnsubscribe => "pending unsubscribe",
            TopicSubscriptionState::Error => "error",
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct JointData {
    pub info: JointInfo,
    pub position: Option<f64>,
    pub velocity: Option<f64>,
    pub effort: Option<f64>,
}

/// Which transport the TUI is currently using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Uds,
    Quic,
}

pub struct AppState {
    pub active_tab: Tab,
    pub active_pane: Pane,
    pub connected: bool,
    /// Set when connected; drives the transport-type indicator in the status bar.
    pub transport_type: Option<TransportType>,
    pub show_help: bool,

    // Topics tab
    pub topics: HashMap<String, TopicData>,
    pub topic_names: Vec<String>,
    pub topic_selected: usize,
    pub tree_expanded: HashMap<String, bool>,
    pub desired_subscriptions: HashSet<String>,
    pub subscriptions_customized: bool,

    // Nodes tab
    pub nodes: Vec<NodeInfo>,
    pub node_selected: usize,

    // Log tab
    pub log_entries: VecDeque<LogEntry>,
    pub log_max_entries: usize,
    pub log_selected: usize,
    pub log_severity_filter: LogLevel,
    pub log_node_filter: String,
    pub log_search: String,
    pub log_editing_filter: bool,

    // Joints tab
    pub joints: Vec<JointData>,
    pub joint_selected: usize,
    pub poses: Vec<PoseInfo>,
    pub pose_selected: usize,
    pub joint_focus: JointFocus,
    pub editing_joint: bool,
    pub joint_input: String,
    pub joint_input_error: Option<String>,
    pub pose_confirming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointFocus {
    JointList,
    PoseList,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Topics,
            active_pane: Pane::Left,
            connected: false,
            transport_type: None,
            show_help: false,
            topics: HashMap::new(),
            topic_names: Vec::new(),
            topic_selected: 0,
            tree_expanded: HashMap::new(),
            desired_subscriptions: HashSet::new(),
            subscriptions_customized: false,
            nodes: Vec::new(),
            node_selected: 0,
            log_entries: VecDeque::new(),
            log_max_entries: 10_000,
            log_selected: 0,
            log_severity_filter: LogLevel::All,
            log_node_filter: String::new(),
            log_search: String::new(),
            log_editing_filter: false,
            joints: Vec::new(),
            joint_selected: 0,
            poses: Vec::new(),
            pose_selected: 0,
            joint_focus: JointFocus::JointList,
            editing_joint: false,
            joint_input: String::new(),
            joint_input_error: None,
            pose_confirming: false,
        }
    }
}

impl AppState {
    pub fn handle_response(&mut self, response: Response) {
        match response {
            Response::TopicList(topics) => {
                for info in topics {
                    let name = info.name.clone();
                    if !self.subscriptions_customized {
                        self.desired_subscriptions.insert(name.clone());
                    }

                    let should_be_subscribed = self.desired_subscriptions.contains(&name);
                    self.topics
                        .entry(name.clone())
                        .and_modify(|topic| {
                            topic.info = info.clone();
                            if !matches!(
                                topic.subscription,
                                TopicSubscriptionState::PendingSubscribe
                                    | TopicSubscriptionState::PendingUnsubscribe
                            ) && topic.subscription_error.is_none()
                            {
                                topic.subscription = if should_be_subscribed {
                                    TopicSubscriptionState::Subscribed
                                } else {
                                    TopicSubscriptionState::Unsubscribed
                                };
                            }
                        })
                        .or_insert_with(|| TopicData {
                            info,
                            latest: None,
                            last_received: None,
                            msg_count: 0,
                            hz: 0.0,
                            subscription: if should_be_subscribed {
                                TopicSubscriptionState::Subscribed
                            } else {
                                TopicSubscriptionState::Unsubscribed
                            },
                            subscription_error: None,
                        });
                    if !self.topic_names.contains(&name) {
                        self.topic_names.push(name);
                    }
                }
                self.topic_names.sort();
            }
            Response::NodeList(nodes) => {
                self.nodes = nodes;
            }
            Response::TopicData {
                topic,
                type_name,
                stamp: _,
                data,
            } => {
                if !self.subscriptions_customized {
                    self.desired_subscriptions.insert(topic.clone());
                }
                let should_be_subscribed = self.desired_subscriptions.contains(&topic);
                let now = Instant::now();
                let entry = self
                    .topics
                    .entry(topic.clone())
                    .or_insert_with(|| TopicData {
                        info: TopicInfo {
                            name: topic.clone(),
                            type_name: type_name.clone(),
                            publisher_count: 0,
                            subscriber_count: 0,
                        },
                        latest: None,
                        last_received: None,
                        msg_count: 0,
                        hz: 0.0,
                        subscription: if should_be_subscribed {
                            TopicSubscriptionState::Subscribed
                        } else {
                            TopicSubscriptionState::Unsubscribed
                        },
                        subscription_error: None,
                    });

                // Update Hz estimate
                if let Some(last) = entry.last_received {
                    let dt = now.duration_since(last).as_secs_f64();
                    if dt > 0.0 {
                        // Exponential moving average
                        let instant_hz = 1.0 / dt;
                        entry.hz = entry.hz * 0.8 + instant_hz * 0.2;
                    }
                }

                entry.latest = Some(data.clone());
                entry.last_received = Some(now);
                entry.msg_count += 1;

                if !self.topic_names.contains(&topic) {
                    self.topic_names.push(topic.clone());
                    self.topic_names.sort();
                }

                // Extract log entries from /rosout
                if topic == "/rosout" {
                    if let Some(entry) = extract_log_entry(&data) {
                        self.log_entries.push_front(entry);
                        while self.log_entries.len() > self.log_max_entries {
                            self.log_entries.pop_back();
                        }
                    }
                }

                // Update joint data from /joint_states
                if topic == "/joint_states" {
                    self.update_joints_from_data(&data);
                }

                // Parse URDF from /robot_description
                if topic == "/robot_description" {
                    if let DynValue::String(urdf_xml) = &data {
                        self.update_joints_from_urdf(urdf_xml);
                    }
                }
            }
            Response::PoseList(poses) => {
                self.poses = poses;
            }
            Response::Subscribed { topics } => {
                for sub in topics {
                    let topic_name = sub.topic.clone();
                    let entry = self
                        .topics
                        .entry(topic_name.clone())
                        .or_insert_with(|| TopicData::placeholder(&topic_name));
                    entry.info.type_name = sub.type_name;
                    entry.subscription = TopicSubscriptionState::Subscribed;
                    entry.subscription_error = None;
                    if !self.topic_names.contains(&topic_name) {
                        self.topic_names.push(topic_name);
                    }
                }
                self.topic_names.sort();
            }
            Response::Unsubscribed { topics } => {
                for topic_name in topics {
                    let entry = self
                        .topics
                        .entry(topic_name.clone())
                        .or_insert_with(|| TopicData::placeholder(&topic_name));
                    entry.subscription = TopicSubscriptionState::Unsubscribed;
                    entry.subscription_error = None;
                }
            }
            Response::Ok(_) => {}
            Response::Error(_) => {}
        }
    }

    pub fn desired_topics_for_connection(&self) -> Vec<String> {
        self.topic_names
            .iter()
            .filter(|name| self.desired_subscriptions.contains(*name))
            .cloned()
            .collect()
    }

    pub fn toggle_selected_topic_subscription(&mut self) -> Option<Request> {
        let topic = self.topic_names.get(self.topic_selected)?.clone();
        self.subscriptions_customized = true;

        if self.desired_subscriptions.remove(&topic) {
            self.set_topic_subscription_state(&topic, TopicSubscriptionState::PendingUnsubscribe);
            Some(Request::Unsubscribe {
                topics: vec![topic],
            })
        } else {
            self.desired_subscriptions.insert(topic.clone());
            self.set_topic_subscription_state(&topic, TopicSubscriptionState::PendingSubscribe);
            Some(Request::Subscribe {
                topics: vec![topic],
            })
        }
    }

    pub fn mark_topics_pending_subscribe(&mut self, topics: &[String]) {
        self.set_topics_subscription_state(topics, TopicSubscriptionState::PendingSubscribe);
    }

    pub fn mark_topics_pending_unsubscribe(&mut self, topics: &[String]) {
        self.set_topics_subscription_state(topics, TopicSubscriptionState::PendingUnsubscribe);
    }

    pub fn mark_subscription_error(&mut self, topics: &[String], error: &str) {
        for topic_name in topics {
            let entry = self
                .topics
                .entry(topic_name.clone())
                .or_insert_with(|| TopicData::placeholder(topic_name));
            entry.subscription = TopicSubscriptionState::Error;
            entry.subscription_error = Some(error.to_string());
        }
    }

    fn set_topics_subscription_state(&mut self, topics: &[String], state: TopicSubscriptionState) {
        for topic_name in topics {
            self.set_topic_subscription_state(topic_name, state);
        }
    }

    fn set_topic_subscription_state(&mut self, topic_name: &str, state: TopicSubscriptionState) {
        let entry = self
            .topics
            .entry(topic_name.to_string())
            .or_insert_with(|| TopicData::placeholder(topic_name));
        entry.subscription = state;
        if state != TopicSubscriptionState::Error {
            entry.subscription_error = None;
        }
    }

    fn update_joints_from_data(&mut self, data: &DynValue) {
        if let DynValue::Struct { fields, .. } = data {
            let names: Vec<String> = fields
                .iter()
                .find(|(k, _)| k == "name")
                .and_then(|(_, v)| {
                    if let DynValue::Array(arr) = v {
                        Some(
                            arr.iter()
                                .filter_map(|v| {
                                    if let DynValue::String(s) = v {
                                        Some(s.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let positions = extract_f64_array(fields, "position");
            let velocities = extract_f64_array(fields, "velocity");
            let efforts = extract_f64_array(fields, "effort");

            for (i, name) in names.iter().enumerate() {
                if let Some(joint) = self.joints.iter_mut().find(|j| &j.info.name == name) {
                    joint.position = positions.get(i).copied();
                    joint.velocity = velocities.get(i).copied();
                    joint.effort = efforts.get(i).copied();
                }
            }
        }
    }

    fn update_joints_from_urdf(&mut self, urdf_xml: &str) {
        if let Ok(joint_infos) = talos_common::urdf::extract_joints(urdf_xml) {
            // Preserve existing position data
            let existing: HashMap<String, (Option<f64>, Option<f64>, Option<f64>)> = self
                .joints
                .iter()
                .map(|j| (j.info.name.clone(), (j.position, j.velocity, j.effort)))
                .collect();

            self.joints = joint_infos
                .into_iter()
                .map(|info| {
                    let (position, velocity, effort) = existing
                        .get(&info.name)
                        .copied()
                        .unwrap_or((None, None, None));
                    JointData {
                        info,
                        position,
                        velocity,
                        effort,
                    }
                })
                .collect();
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

fn extract_f64_array(fields: &[(String, DynValue)], name: &str) -> Vec<f64> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| {
            if let DynValue::Array(arr) = v {
                Some(
                    arr.iter()
                        .filter_map(|v| {
                            if let DynValue::F64(f) = v {
                                Some(*f)
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_list_defaults_to_subscribed_until_user_customizes() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            TopicInfo {
                name: "/camera".into(),
                type_name: "sensor_msgs/msg/Image".into(),
                publisher_count: 1,
                subscriber_count: 0,
            },
            TopicInfo {
                name: "/rosout".into(),
                type_name: "rcl_interfaces/msg/Log".into(),
                publisher_count: 1,
                subscriber_count: 0,
            },
        ]));

        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string(), "/rosout".to_string()]
        );
    }

    #[test]
    fn toggle_selected_topic_preserves_manual_subscription_choice() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            TopicInfo {
                name: "/camera".into(),
                type_name: "sensor_msgs/msg/Image".into(),
                publisher_count: 1,
                subscriber_count: 0,
            },
            TopicInfo {
                name: "/rosout".into(),
                type_name: "rcl_interfaces/msg/Log".into(),
                publisher_count: 1,
                subscriber_count: 0,
            },
        ]));
        state.topic_selected = 1;

        let request = state.toggle_selected_topic_subscription();

        assert_eq!(
            request,
            Some(Request::Unsubscribe {
                topics: vec!["/rosout".to_string()]
            })
        );
        assert!(state.subscriptions_customized);
        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string()]
        );
        assert_eq!(
            state.topics["/rosout"].subscription,
            TopicSubscriptionState::PendingUnsubscribe
        );
    }
}
