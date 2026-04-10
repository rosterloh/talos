use serde::{Deserialize, Serialize};

use super::types::{DynValue, NodeInfo, PoseInfo, Timestamp, TopicInfo, TopicSub};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Request {
    ListTopics,
    ListNodes,
    SetJointPosition { joint: String, position: f64 },
    ExecutePose { name: String },
    Subscribe { topics: Vec<String> },
    Unsubscribe { topics: Vec<String> },
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
    Subscribed { topics: Vec<TopicSub> },
    Unsubscribed { topics: Vec<String> },
    Error(String),
}
