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
pub(crate) struct PendingTopicSubscriptionToggle {
    pub(crate) request: Request,
    topic: String,
    previous_subscription: TopicSubscriptionState,
    previous_subscription_error: Option<String>,
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

type JointSnapshot = (Option<f64>, Option<f64>, Option<f64>);

/// Which transport the TUI is currently using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Uds,
    #[cfg(feature = "quic")]
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
    // Sticky on purpose: once a user makes any manual choice, later topic
    // catalogs should keep honoring that explicit desired set across reconnects.
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
                let auto_subscribe_all = !self.subscriptions_customized;
                let mut current_topics = std::mem::take(&mut self.topics);
                let mut next_topics = HashMap::with_capacity(topics.len());
                let mut next_topic_names = Vec::with_capacity(topics.len());
                // Missing topics are intentionally dropped from the visible
                // catalog and cached samples until the agent advertises them again.

                for info in topics {
                    let name = info.name.clone();
                    next_topic_names.push(name.clone());

                    let should_be_subscribed =
                        auto_subscribe_all || self.desired_subscriptions.contains(&name);
                    // `ListTopics` currently arrives once per connection, so a fresh
                    // catalog snapshot resets the per-connection subscription baseline.
                    // Keep pending manual toggles visible, but otherwise wait for the
                    // subscribe ack or live data before showing a topic as on again.
                    let mut topic = current_topics
                        .remove(&name)
                        .unwrap_or_else(|| TopicData::placeholder(&name));

                    topic.info = info;
                    if matches!(
                        topic.subscription,
                        TopicSubscriptionState::PendingSubscribe
                            | TopicSubscriptionState::PendingUnsubscribe
                    ) {
                        // Keep in-flight manual changes visible until the matching
                        // ack or retry path resolves them.
                    } else if topic.subscription == TopicSubscriptionState::Error
                        && !should_be_subscribed
                    {
                        topic.subscription = TopicSubscriptionState::Unsubscribed;
                        topic.subscription_error = None;
                    } else if topic.subscription != TopicSubscriptionState::Error {
                        topic.subscription = TopicSubscriptionState::Unsubscribed;
                    }

                    if auto_subscribe_all {
                        self.desired_subscriptions.insert(name.clone());
                    }
                    next_topics.insert(name, topic);
                }

                if auto_subscribe_all {
                    self.desired_subscriptions = next_topic_names.iter().cloned().collect();
                }
                self.topics = next_topics;
                self.tree_expanded
                    .retain(|topic_name, _| self.topics.contains_key(topic_name));
                self.replace_topic_names(next_topic_names);
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
                let keep_pending_unsubscribe_data = self.topics.get(&topic).is_some_and(|entry| {
                    entry.subscription == TopicSubscriptionState::PendingUnsubscribe
                });
                if !should_be_subscribed && !keep_pending_unsubscribe_data {
                    return;
                }

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
                if should_be_subscribed
                    && !matches!(
                        entry.subscription,
                        TopicSubscriptionState::PendingUnsubscribe
                    )
                {
                    entry.subscription = TopicSubscriptionState::Subscribed;
                    entry.subscription_error = None;
                }

                self.ensure_topic_name(&topic);

                // Extract log entries from /rosout
                if topic == "/rosout"
                    && let Some(entry) = extract_log_entry(&data)
                {
                    self.log_entries.push_front(entry);
                    while self.log_entries.len() > self.log_max_entries {
                        self.log_entries.pop_back();
                    }
                }

                // Update joint data from /joint_states
                if topic == "/joint_states" {
                    self.update_joints_from_data(&data);
                }

                // Parse URDF from /robot_description
                if topic == "/robot_description"
                    && let DynValue::String(urdf_xml) = &data
                {
                    self.update_joints_from_urdf(urdf_xml);
                }
            }
            Response::PoseList(poses) => {
                self.poses = poses;
            }
            Response::Subscribed { topics } => {
                for sub in topics {
                    let topic_name = sub.topic.clone();
                    if !self.desired_subscriptions.contains(&topic_name) {
                        continue;
                    }
                    let entry = self
                        .topics
                        .entry(topic_name.clone())
                        // Defensive fallback: the normal path sees TopicList first.
                        .or_insert_with(|| TopicData {
                            info: TopicInfo {
                                name: topic_name.clone(),
                                type_name: sub.type_name.clone(),
                                publisher_count: 0,
                                subscriber_count: 0,
                            },
                            latest: None,
                            last_received: None,
                            msg_count: 0,
                            hz: 0.0,
                            subscription: TopicSubscriptionState::Unsubscribed,
                            subscription_error: None,
                        });
                    entry.info.type_name = sub.type_name;
                    entry.subscription = TopicSubscriptionState::Subscribed;
                    entry.subscription_error = None;
                    self.ensure_topic_name(&topic_name);
                }
            }
            Response::Unsubscribed { topics } => {
                for topic_name in topics {
                    if self.desired_subscriptions.contains(&topic_name) {
                        continue;
                    }
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

    /// Returns the desired topic set for the next connection, limited to topics
    /// that still exist in the latest server-advertised catalog.
    pub fn desired_topics_for_connection(&self) -> Vec<String> {
        self.topic_names
            .iter()
            .filter(|name| {
                self.desired_subscriptions.contains(*name) && self.topics.contains_key(*name)
            })
            .cloned()
            .collect()
    }

    /// Optimistically updates desired subscription intent so a failed manual
    /// toggle is retried automatically after reconnect.
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

    pub(crate) fn prepare_selected_topic_subscription_toggle(
        &mut self,
    ) -> Option<PendingTopicSubscriptionToggle> {
        let topic = self.topic_names.get(self.topic_selected)?.clone();
        let previous_subscription = self
            .topics
            .get(&topic)
            .map(|entry| entry.subscription)
            .unwrap_or(TopicSubscriptionState::Unsubscribed);
        let previous_subscription_error = self
            .topics
            .get(&topic)
            .and_then(|entry| entry.subscription_error.clone());
        let request = self.toggle_selected_topic_subscription()?;

        Some(PendingTopicSubscriptionToggle {
            request,
            topic,
            previous_subscription,
            previous_subscription_error,
        })
    }

    pub(crate) fn revert_topic_subscription_toggle(
        &mut self,
        toggle: PendingTopicSubscriptionToggle,
    ) {
        match &toggle.request {
            Request::Subscribe { topics } => {
                for topic_name in topics {
                    self.desired_subscriptions.remove(topic_name);
                }
            }
            Request::Unsubscribe { topics } => {
                for topic_name in topics {
                    self.desired_subscriptions.insert(topic_name.clone());
                }
            }
            _ => return,
        }

        let entry = self
            .topics
            .entry(toggle.topic.clone())
            .or_insert_with(|| TopicData::placeholder(&toggle.topic));
        entry.subscription = toggle.previous_subscription;
        entry.subscription_error = toggle.previous_subscription_error;
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

    fn selected_topic_name(&self) -> Option<String> {
        self.topic_names.get(self.topic_selected).cloned()
    }

    fn ensure_topic_name(&mut self, topic_name: &str) {
        if self.topic_names.iter().any(|name| name == topic_name) {
            return;
        }

        let selected_topic = self.selected_topic_name();
        self.topic_names.push(topic_name.to_string());
        self.topic_names.sort();
        self.restore_topic_selection(selected_topic.as_deref());
    }

    fn replace_topic_names(&mut self, topic_names: Vec<String>) {
        let selected_topic = self.selected_topic_name();
        self.topic_names = topic_names;
        self.topic_names.sort();
        self.topic_names.dedup();
        self.restore_topic_selection(selected_topic.as_deref());
    }

    fn restore_topic_selection(&mut self, selected_topic: Option<&str>) {
        if self.topic_names.is_empty() {
            self.topic_selected = 0;
            return;
        }

        if let Some(selected_topic) = selected_topic
            && let Some(index) = self
                .topic_names
                .iter()
                .position(|topic_name| topic_name == selected_topic)
        {
            self.topic_selected = index;
            return;
        }

        self.topic_selected = self.topic_selected.min(self.topic_names.len() - 1);
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
            let existing: HashMap<String, JointSnapshot> = self
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
    use talos_common::protocol::types::{Timestamp, TopicSub};

    fn topic(name: &str, type_name: &str) -> TopicInfo {
        TopicInfo {
            name: name.into(),
            type_name: type_name.into(),
            publisher_count: 1,
            subscriber_count: 0,
        }
    }

    #[test]
    fn topic_list_defaults_reconnect_intent_to_all_topics_until_user_customizes() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));

        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string(), "/rosout".to_string()]
        );
    }

    #[test]
    fn topic_list_does_not_speculatively_mark_desired_topics_subscribed() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::Unsubscribed
        );
    }

    #[test]
    fn toggle_selected_topic_preserves_manual_subscription_choice() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
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

    fn ignored_subscribed_ack_keeps_selected_topic_stable() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/beta", "std_msgs/msg/String"),
            topic("/delta", "std_msgs/msg/String"),
        ]));
        state.topic_selected = 1;

        state.handle_response(Response::Subscribed {
            topics: vec![TopicSub {
                topic: "/alpha".into(),
                type_name: "std_msgs/msg/String".into(),
            }],
        });

        assert_eq!(
            state.topic_names,
            vec!["/beta".to_string(), "/delta".to_string()]
        );
        assert_eq!(state.topic_names[state.topic_selected], "/delta");
    }

    #[test]
    fn latest_topic_list_removes_missing_topics_from_reconnect_intent() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));
        state.topic_selected = 1;

        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(state.topic_names, vec!["/camera".to_string()]);
        assert_eq!(state.topic_selected, 0);
        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string()]
        );
    }

    #[test]
    fn new_topics_after_customization_default_to_unsubscribed() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));
        state.topic_selected = 0;

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/lidar", "sensor_msgs/msg/LaserScan"),
        ]));

        assert!(!state.desired_subscriptions.contains("/lidar"));
        assert_eq!(
            state.topics["/lidar"].subscription,
            TopicSubscriptionState::Unsubscribed
        );
    }

    #[test]
    fn topic_list_clears_stale_unsubscribe_errors_once_desired_state_is_off() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));
        state.topic_selected = 1;

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/rosout".to_string()]
            })
        );
        state.mark_subscription_error(&["/rosout".to_string()], "boom");

        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));

        assert_eq!(
            state.topics["/rosout"].subscription,
            TopicSubscriptionState::Unsubscribed
        );
        assert_eq!(state.topics["/rosout"].subscription_error, None);
    }

    #[test]
    fn stale_subscribed_ack_does_not_override_pending_unsubscribe() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::Subscribed {
            topics: vec![TopicSub {
                topic: "/camera".into(),
                type_name: "sensor_msgs/msg/Image".into(),
            }],
        });

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::PendingUnsubscribe
        );
        assert!(!state.desired_subscriptions.contains("/camera"));
    }

    #[test]
    fn stale_unsubscribed_ack_does_not_override_pending_subscribe() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );
        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Subscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::Unsubscribed {
            topics: vec!["/camera".to_string()],
        });

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::PendingSubscribe
        );
        assert!(state.desired_subscriptions.contains("/camera"));
    }

    #[test]
    fn topic_data_clears_error_once_desired_subscription_is_healthy() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));
        state.mark_subscription_error(&["/camera".to_string()], "boom");

        state.handle_response(Response::TopicData {
            topic: "/camera".into(),
            type_name: "sensor_msgs/msg/Image".into(),
            stamp: Timestamp { sec: 0, nanosec: 0 },
            data: DynValue::String("frame".into()),
        });

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::Subscribed
        );
        assert_eq!(state.topics["/camera"].subscription_error, None);
    }

    #[test]
    fn topic_data_while_unsubscribe_is_pending_still_updates_latest_sample() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::TopicData {
            topic: "/camera".into(),
            type_name: "sensor_msgs/msg/Image".into(),
            stamp: Timestamp { sec: 0, nanosec: 0 },
            data: DynValue::String("during-pending".into()),
        });

        let topic = &state.topics["/camera"];
        assert_eq!(
            topic.subscription,
            TopicSubscriptionState::PendingUnsubscribe
        );
        assert_eq!(
            topic.latest,
            Some(DynValue::String("during-pending".into()))
        );
        assert_eq!(topic.msg_count, 1);
    }

    #[test]
    fn topic_data_is_ignored_once_topic_is_unsubscribed() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        {
            let topic = state.topics.get_mut("/camera").unwrap();
            topic.latest = Some(DynValue::String("before".into()));
            topic.last_received = Some(Instant::now());
            topic.msg_count = 41;
            topic.hz = 12.5;
        }

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );
        state.handle_response(Response::Unsubscribed {
            topics: vec!["/camera".to_string()],
        });

        let before_last_received = state.topics["/camera"].last_received;
        state.handle_response(Response::TopicData {
            topic: "/camera".into(),
            type_name: "sensor_msgs/msg/Image".into(),
            stamp: Timestamp { sec: 1, nanosec: 0 },
            data: DynValue::String("after".into()),
        });

        let topic = &state.topics["/camera"];
        assert_eq!(topic.subscription, TopicSubscriptionState::Unsubscribed);
        assert_eq!(topic.latest, Some(DynValue::String("before".into())));
        assert_eq!(topic.last_received, before_last_received);
        assert_eq!(topic.msg_count, 41);
        assert_eq!(topic.hz, 12.5);
    }
}
