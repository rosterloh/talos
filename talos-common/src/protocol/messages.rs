use serde::{Deserialize, Serialize};

use super::params::{ParamInfo, ParamValue};
use super::types::{DynValue, NodeInfo, PoseInfo, Timestamp, TopicInfo, TopicSub};

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
}
