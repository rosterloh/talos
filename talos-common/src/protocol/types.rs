use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicSub {
    pub topic: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicFrame {
    pub stamp: Timestamp,
    pub data: DynValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamHeader {
    pub topic: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DynValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<DynValue>),
    Struct {
        type_name: String,
        fields: Vec<(String, DynValue)>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timestamp {
    pub sec: i32,
    pub nanosec: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicInfo {
    pub name: String,
    pub type_name: String,
    pub publisher_count: usize,
    pub subscriber_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub namespace: String,
    pub publishers: Vec<String>,
    pub subscribers: Vec<String>,
    pub services: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointInfo {
    pub name: String,
    pub joint_type: JointType,
    pub parent_link: String,
    pub child_link: String,
    pub limits: Option<JointLimits>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JointType {
    Revolute,
    Prismatic,
    Continuous,
    Fixed,
    Floating,
    Planar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointLimits {
    pub lower: f64,
    pub upper: f64,
    pub effort: f64,
    pub velocity: f64,
}

/// Per-topic traffic measured by the agent before any per-client frame
/// dropping, so it stays accurate for slow clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicStats {
    pub topic: String,
    /// Smoothed messages per second.
    pub rate_hz: f64,
    /// Smoothed payload bytes per second, from the size of the decoded
    /// message (an approximation of the size on the wire).
    pub bandwidth_bps: f64,
    /// Smoothed agent receive time minus `header.stamp`, in milliseconds.
    /// `None` for types without a stamp. Uses the agent's wall clock, so it
    /// is meaningless under simulated time.
    pub latency_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseInfo {
    pub name: String,
    pub positions: Vec<(String, f64)>,
}

pub use super::params::{ParamInfo, ParamValue};
