use std::collections::{HashMap, HashSet, VecDeque};

use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{NodeInfo, ParamInfo, PoseInfo};

mod joints;
mod logs;
mod params;
mod topics;

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

    // Params tab
    pub param_node_selected: usize,
    pub param_node: Option<String>,
    pub parameters: Vec<ParamInfo>,
    pub param_selected: usize,
    pub editing_param: bool,
    pub param_input: String,
    pub param_status: Option<String>,
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
            param_node_selected: 0,
            param_node: None,
            parameters: Vec::new(),
            param_selected: 0,
            editing_param: false,
            param_input: String::new(),
            param_status: None,
        }
    }
}

impl AppState {
    pub fn handle_response(&mut self, response: Response) {
        match response {
            Response::TopicList(topics) => self.handle_topic_list(topics),
            Response::NodeList(nodes) => {
                self.nodes = nodes;
            }
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
            Response::Error(_) => {}
        }
    }
}
