use serde::{Deserialize, Serialize};

use super::params::{ParamInfo, ParamValue};
use super::types::{
    DynValue, EndpointInfo, NodeInfo, PoseInfo, Timestamp, TopicInfo, TopicStats, TopicSub,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Request {
    ListTopics,
    ListNodes,
    ListPoses,
    SetJointPosition {
        joint: String,
        position: f64,
    },
    ExecutePose {
        name: String,
    },
    Subscribe {
        topics: Vec<String>,
    },
    Unsubscribe {
        topics: Vec<String>,
    },
    /// List all parameters of a node (names with their current values).
    ListParameters {
        node: String,
    },
    /// Get the current values of specific parameters of a node.
    GetParameters {
        node: String,
        names: Vec<String>,
    },
    /// Set a single parameter value on a node.
    SetParameter {
        node: String,
        name: String,
        value: ParamValue,
    },
    /// Agent-side rate/bandwidth/latency for every bridged topic.
    GetTopicStats,
    /// Publishers and subscriptions on `topic`, with their QoS.
    GetTopicEndpoints {
        topic: String,
    },
    /// Level of a node's logger; an empty `logger` means the node's own logger.
    GetLoggerLevel {
        node: String,
        logger: String,
    },
    /// Set the level of a node's logger; an empty `logger` means the node's own logger.
    SetLoggerLevel {
        node: String,
        logger: String,
        level: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Response {
    TopicList(Vec<TopicInfo>),
    NodeList(Vec<NodeInfo>),
    TopicData {
        topic: String,
        type_name: String,
        stamp: Timestamp,
        data: DynValue,
    },
    PoseList(Vec<PoseInfo>),
    Subscribed {
        topics: Vec<TopicSub>,
    },
    Unsubscribed {
        topics: Vec<String>,
    },
    /// Parameter names with values, for both `ListParameters` and `GetParameters`.
    Parameters {
        node: String,
        parameters: Vec<ParamInfo>,
    },
    /// Result of a `SetParameter` request.
    ParameterSet {
        node: String,
        name: String,
        successful: bool,
        reason: String,
    },
    Ok(String),
    Error(String),
    /// Reply to `GetTopicStats`.
    TopicStats(Vec<TopicStats>),
    /// Reply to `GetTopicEndpoints`.
    TopicEndpoints {
        topic: String,
        publishers: Vec<EndpointInfo>,
        subscribers: Vec<EndpointInfo>,
    },
    /// Reply to `GetLoggerLevel`, with the resolved logger name.
    LoggerLevel {
        node: String,
        logger: String,
        level: u32,
    },
    /// Reply to `SetLoggerLevel`.
    LoggerLevelSet {
        node: String,
        logger: String,
        successful: bool,
        reason: String,
    },
}
