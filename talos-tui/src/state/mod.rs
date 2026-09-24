use ratatui::widgets::{ListState, TableState};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{EndpointInfo, NodeInfo, ParamInfo, PoseInfo};

mod filter;
mod joints;
mod logs;
mod params;
mod topics;
mod tree;

pub use filter::{FilterPrompt, TextInput};
pub use joints::{JointData, JointFocus};
pub use logs::{LogEntry, LogLevel};
pub(crate) use params::{node_fqn, node_label};
pub use topics::{TopicData, TopicSubscriptionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Topics,
    Nodes,
    Log,
    Joints,
    Params,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Topics, Tab::Nodes, Tab::Log, Tab::Joints, Tab::Params];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Topics => "Topics",
            Tab::Nodes => "Nodes",
            Tab::Log => "Log",
            Tab::Joints => "Joints",
            Tab::Params => "Params",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Topics => 0,
            Tab::Nodes => 1,
            Tab::Log => 2,
            Tab::Joints => 3,
            Tab::Params => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Left,
    Right,
}

/// Which transport the TUI is currently using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportType {
    Uds,
    #[cfg(feature = "quic")]
    Quic,
}

pub struct AppState {
    // Widget viewport state is presentation-only; keep it across redraws/resizes.
    pub lists: RefCell<HashMap<String, ListState>>,
    pub tables: RefCell<HashMap<String, TableState>>,
    pub scroll: RefCell<HashMap<String, u16>>,
    pub tree_selection: HashMap<String, String>,
    pub show_endpoints: bool,
    pub log_live: bool,
    pub log_expanded: bool,
    pub log_inspected: Option<LogEntry>,
    pub param_edit_target: Option<(String, String)>,
    pub joint_targets: HashMap<String, f64>,
    pub joint_pending: bool,
    pub no_color: bool,
    pub active_tab: Tab,
    pub active_pane: Pane,
    pub connected: bool,
    /// Set when connected; drives the transport-type indicator in the status bar.
    pub transport_type: Option<TransportType>,
    pub show_help: bool,
    /// Open `/` filter prompt for the active tab's list.
    pub filter_prompt: Option<FilterPrompt>,

    // Topics tab
    pub topics: HashMap<String, TopicData>,
    pub topic_names: Vec<String>,
    pub topic_selected: usize,
    pub tree_expanded: HashMap<String, bool>,
    pub desired_subscriptions: HashSet<String>,
    // Sticky on purpose: once a user makes any manual choice, later topic
    // catalogs should keep honoring that explicit desired set across reconnects.
    pub subscriptions_customized: bool,
    pub topic_filter: String,

    // Nodes tab
    pub nodes: Vec<NodeInfo>,
    pub node_selected: usize,
    pub node_filter: String,
    /// Node that `logger_level` and `logger_status` belong to.
    pub logger_node: Option<String>,
    pub logger_level: Option<u32>,
    /// Outcome of the last logger-level set, or an agent error.
    pub logger_status: Option<String>,

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
    /// Outcome of the last joint or pose command.
    pub joint_status: Option<String>,
    pub pose_confirming: bool,

    // Params tab
    pub param_node_selected: usize,
    pub param_node: Option<String>,
    pub parameters: Vec<ParamInfo>,
    pub param_selected: usize,
    pub param_filter: String,
    pub editing_param: bool,
    pub param_input: TextInput,
    pub param_status: Option<String>,
    /// A load or set was sent and its first reply should update `param_status`.
    pub param_awaiting_reply: bool,
    /// Endpoints of the selected topic, from the last `GetTopicEndpoints`.
    pub topic_endpoints: Option<TopicEndpoints>,
}

#[derive(Debug, Clone)]
pub struct TopicEndpoints {
    pub topic: String,
    pub publishers: Vec<EndpointInfo>,
    pub subscribers: Vec<EndpointInfo>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            lists: RefCell::default(),
            tables: RefCell::default(),
            scroll: RefCell::default(),
            tree_selection: HashMap::new(),
            show_endpoints: false,
            log_live: true,
            log_expanded: false,
            log_inspected: None,
            param_edit_target: None,
            joint_targets: HashMap::new(),
            joint_pending: false,
            no_color: std::env::var_os("NO_COLOR").is_some_and(|s| !s.is_empty()),
            active_tab: Tab::Topics,
            active_pane: Pane::Left,
            connected: false,
            transport_type: None,
            show_help: false,
            filter_prompt: None,
            topics: HashMap::new(),
            topic_names: Vec::new(),
            topic_selected: 0,
            tree_expanded: HashMap::new(),
            desired_subscriptions: HashSet::new(),
            subscriptions_customized: false,
            topic_filter: String::new(),
            nodes: Vec::new(),
            node_selected: 0,
            node_filter: String::new(),
            logger_node: None,
            logger_level: None,
            logger_status: None,
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
            joint_status: None,
            pose_confirming: false,
            param_node_selected: 0,
            param_node: None,
            parameters: Vec::new(),
            param_selected: 0,
            param_filter: String::new(),
            editing_param: false,
            param_input: TextInput::default(),
            param_status: None,
            param_awaiting_reply: false,
            topic_endpoints: None,
        }
    }
}

impl AppState {
    /// A publication acknowledgement is not evidence that a joint reached its target.
    pub fn handle_joint_command_response(&mut self, response: Response) {
        self.joint_pending = false;
        match response {
            Response::Ok(_) => {
                if let Some(status) = &mut self.joint_status {
                    *status = status.replacen("pending", "published", 1);
                }
            }
            Response::Error(e) => self.joint_status = Some(format!("error: {e}")),
            other => self.handle_response(other),
        }
    }

    pub fn scroll_by(&self, key: &str, delta: i16) {
        let mut scroll = self.scroll.borrow_mut();
        let offset = scroll.entry(key.into()).or_default();
        *offset = offset.saturating_add_signed(delta);
    }

    pub fn node_scroll_key(&self) -> String {
        format!(
            "node:{}",
            self.filtered_nodes()
                .get(self.node_selected)
                .map(|n| node_fqn(n))
                .unwrap_or_default()
        )
    }

    /// Reply to `GetLoggerLevel` / `SetLoggerLevel`. Errors go to
    /// `logger_status` rather than a pending parameter request.
    pub fn handle_logger_response(&mut self, response: Response) {
        match response {
            Response::Error(e) => self.logger_status = Some(format!("error: {e}")),
            other => self.handle_response(other),
        }
    }

    /// Topic whose endpoints should be fetched: the one selected on the
    /// Topics tab.
    pub fn endpoint_query_topic(&self) -> Option<String> {
        (self.active_tab == Tab::Topics)
            .then(|| self.selected_topic_name())
            .flatten()
    }

    /// Reply to `GetTopicEndpoints`. A failed graph query only clears the
    /// endpoint view; it must not reach `handle_response`, where an `Error`
    /// would be taken as the reply to a pending parameter request.
    pub fn handle_endpoints_response(&mut self, response: Response) {
        match response {
            Response::Error(e) => {
                self.topic_endpoints = None;
                tracing::warn!("failed to list topic endpoints: {e}");
            }
            other => self.handle_response(other),
        }
    }

    /// Replace the node list, keeping the Nodes and Params selections on the
    /// same node by name, or clamped if it is gone.
    fn handle_node_list(&mut self, nodes: Vec<NodeInfo>) {
        // `node_selected` indexes the filtered Nodes list; `param_node_selected`
        // indexes the unfiltered list on the Params tab.
        let node_key = self
            .filtered_nodes()
            .get(self.node_selected)
            .map(|n| node_fqn(n));
        let param_key = self.nodes.get(self.param_node_selected).map(node_fqn);
        self.nodes = nodes;
        let reselect = |selected: &mut usize, key: Option<String>, fqns: Vec<String>| match key
            .and_then(|k| fqns.iter().position(|f| *f == k))
        {
            Some(index) => *selected = index,
            None => filter::clamp_selection(selected, fqns.len()),
        };
        let fqns = self.filtered_nodes().into_iter().map(node_fqn).collect();
        reselect(&mut self.node_selected, node_key, fqns);
        let fqns = self.nodes.iter().map(node_fqn).collect();
        reselect(&mut self.param_node_selected, param_key, fqns);
    }

    pub fn handle_response(&mut self, response: Response) {
        match response {
            Response::TopicList(topics) => self.handle_topic_list(topics),
            Response::NodeList(nodes) => self.handle_node_list(nodes),
            Response::TopicData {
                topic,
                type_name,
                stamp: _,
                data,
            } => self.handle_topic_data(topic, type_name, data),
            Response::PoseList(poses) => {
                self.poses = poses;
            }
            Response::Parameters { node, parameters } => self.handle_parameters(node, parameters),
            Response::ParameterSet {
                name,
                successful,
                reason,
                ..
            } => self.handle_parameter_set(name, successful, reason),
            Response::Subscribed { topics } => self.handle_subscribed_topics(topics),
            Response::Unsubscribed { topics } => self.handle_unsubscribed_topics(topics),
            Response::Ok(_) => {}
            Response::TopicStats(stats) => self.handle_topic_stats(stats),
            Response::TopicEndpoints {
                topic,
                publishers,
                subscribers,
            } => {
                self.topic_endpoints = Some(TopicEndpoints {
                    topic,
                    publishers,
                    subscribers,
                });
            }
            Response::LoggerLevel { node, level, .. } => {
                self.logger_node = Some(node);
                self.logger_level = Some(level);
            }
            Response::LoggerLevelSet {
                successful, reason, ..
            } => {
                self.logger_status = (!successful).then(|| format!("rejected: {reason}"));
            }
            Response::Error(e) => {
                if std::mem::take(&mut self.param_awaiting_reply) {
                    self.param_status = Some(format!("error: {e}"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str) -> NodeInfo {
        NodeInfo {
            name: name.into(),
            namespace: "/".into(),
            publishers: vec![],
            subscribers: vec![],
            services: vec![],
        }
    }

    #[test]
    fn node_list_refresh_keeps_selection_by_name_or_clamps() {
        let mut state = AppState::default();
        state.handle_response(Response::NodeList(vec![node("a"), node("b"), node("c")]));
        state.node_selected = 1;
        state.param_node_selected = 2;

        state.handle_response(Response::NodeList(vec![node("new"), node("a"), node("b")]));
        assert_eq!(state.nodes[state.node_selected].name, "b");
        // "c" is gone: clamp to the end of the list.
        assert_eq!(state.param_node_selected, 2);

        state.handle_response(Response::NodeList(vec![]));
        assert_eq!(state.node_selected, 0);
    }

    #[test]
    fn node_list_refresh_keeps_selection_within_filter() {
        let mut state = AppState {
            node_filter: "cam".into(),
            ..AppState::default()
        };
        state.handle_response(Response::NodeList(vec![
            node("cam_left"),
            node("lidar"),
            node("cam_right"),
        ]));
        state.node_selected = 1;
        assert_eq!(
            state.filtered_nodes()[state.node_selected].name,
            "cam_right"
        );

        state.handle_response(Response::NodeList(vec![
            node("cam_front"),
            node("cam_left"),
            node("cam_right"),
        ]));
        assert_eq!(
            state.filtered_nodes()[state.node_selected].name,
            "cam_right"
        );
    }
}
