use std::fmt;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseInfo {
    pub name: String,
    pub positions: Vec<(String, f64)>,
}

/// A ROS 2 parameter value. Mirrors the variant set of
/// `rcl_interfaces/msg/ParameterValue` without depending on any ROS 2 types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    NotSet,
    Bool(bool),
    Integer(i64),
    Double(f64),
    String(String),
    ByteArray(Vec<u8>),
    BoolArray(Vec<bool>),
    IntegerArray(Vec<i64>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
}

impl ParamValue {
    /// Human-readable type tag, e.g. for column headings.
    pub fn type_name(&self) -> &'static str {
        match self {
            ParamValue::NotSet => "not set",
            ParamValue::Bool(_) => "bool",
            ParamValue::Integer(_) => "integer",
            ParamValue::Double(_) => "double",
            ParamValue::String(_) => "string",
            ParamValue::ByteArray(_) => "byte_array",
            ParamValue::BoolArray(_) => "bool_array",
            ParamValue::IntegerArray(_) => "integer_array",
            ParamValue::DoubleArray(_) => "double_array",
            ParamValue::StringArray(_) => "string_array",
        }
    }

    /// Infer a `ParamValue` from user input (CLI argument or TUI edit field).
    ///
    /// Recognises `true`/`false`, integers, floats, `[..]` arrays, and falls
    /// back to a string (quotes are stripped). This is inference for new values;
    /// use [`parse_preserving_type`](Self::parse_preserving_type) when editing an
    /// existing parameter value.
    pub fn parse(input: &str) -> ParamValue {
        let s = input.trim();
        if let Some(inner) = s.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            return Self::parse_array(inner);
        }
        Self::parse_scalar(s)
    }

    /// Parse user input while preserving the type of an existing parameter where
    /// possible. This prevents TUI edit buffers from changing `"42"` strings
    /// into integers, and keeps display-only values such as byte arrays intact
    /// when the user submits them unchanged.
    pub fn parse_preserving_type(input: &str, current: &ParamValue) -> ParamValue {
        let s = input.trim();
        if s == current.to_string() {
            return current.clone();
        }

        match current {
            ParamValue::NotSet => Self::parse(s),
            ParamValue::Bool(_) => parse_bool(s)
                .map(ParamValue::Bool)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::Integer(_) => s
                .parse::<i64>()
                .map(ParamValue::Integer)
                .unwrap_or_else(|_| Self::parse(s)),
            ParamValue::Double(_) => s
                .parse::<f64>()
                .map(ParamValue::Double)
                .unwrap_or_else(|_| Self::parse(s)),
            ParamValue::String(_) => ParamValue::String(strip_quotes(s).to_string()),
            ParamValue::ByteArray(_) => parse_typed_array(s, |e| e.parse::<u8>().ok())
                .map(ParamValue::ByteArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::BoolArray(_) => parse_typed_array(s, parse_bool)
                .map(ParamValue::BoolArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::IntegerArray(_) => parse_typed_array(s, |e| e.parse::<i64>().ok())
                .map(ParamValue::IntegerArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::DoubleArray(_) => parse_typed_array(s, |e| e.parse::<f64>().ok())
                .map(ParamValue::DoubleArray)
                .unwrap_or_else(|| Self::parse(s)),
            ParamValue::StringArray(_) => parse_array_elements(s)
                .map(|elems| {
                    ParamValue::StringArray(
                        elems
                            .into_iter()
                            .map(|e| strip_quotes(e).to_string())
                            .collect(),
                    )
                })
                .unwrap_or_else(|| Self::parse(s)),
        }
    }

    fn parse_scalar(s: &str) -> ParamValue {
        if let Some(b) = parse_bool(s) {
            return ParamValue::Bool(b);
        }
        if let Ok(i) = s.parse::<i64>() {
            return ParamValue::Integer(i);
        }
        if let Ok(f) = s.parse::<f64>() {
            return ParamValue::Double(f);
        }
        ParamValue::String(strip_quotes(s).to_string())
    }

    fn parse_array(inner: &str) -> ParamValue {
        let inner = inner.trim();
        if inner.is_empty() {
            return ParamValue::StringArray(Vec::new());
        }
        let elems: Vec<&str> = inner.split(',').map(str::trim).collect();
        if let Some(bools) = elems
            .iter()
            .map(|e| parse_bool(e))
            .collect::<Option<Vec<_>>>()
        {
            return ParamValue::BoolArray(bools);
        }
        if let Some(ints) = elems
            .iter()
            .map(|e| e.parse::<i64>().ok())
            .collect::<Option<Vec<_>>>()
        {
            return ParamValue::IntegerArray(ints);
        }
        if let Some(floats) = elems
            .iter()
            .map(|e| e.parse::<f64>().ok())
            .collect::<Option<Vec<_>>>()
        {
            return ParamValue::DoubleArray(floats);
        }
        ParamValue::StringArray(elems.iter().map(|e| strip_quotes(e).to_string()).collect())
    }
}

fn parse_array_elements(s: &str) -> Option<Vec<&str>> {
    let inner = s.strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    Some(inner.split(',').map(str::trim).collect())
}

fn parse_typed_array<T>(s: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    parse_array_elements(s)?
        .into_iter()
        .map(parse)
        .collect::<Option<Vec<_>>>()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
        .unwrap_or(s)
}

/// Format an `f64` so it always reads as a float (e.g. `1.0`, never `1`),
/// keeping [`ParamValue::Double`] distinct from [`ParamValue::Integer`] when
/// displayed and re-parsed.
fn write_f64(f: &mut fmt::Formatter<'_>, v: f64) -> fmt::Result {
    if v.is_finite() && v == v.trunc() {
        write!(f, "{v:.1}")
    } else {
        write!(f, "{v}")
    }
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::NotSet => write!(f, "<not set>"),
            ParamValue::Bool(b) => write!(f, "{b}"),
            ParamValue::Integer(i) => write!(f, "{i}"),
            ParamValue::Double(d) => write_f64(f, *d),
            ParamValue::String(s) => write!(f, "{s}"),
            ParamValue::ByteArray(a) => write!(f, "[{} bytes]", a.len()),
            ParamValue::BoolArray(a) => write_seq(f, a),
            ParamValue::IntegerArray(a) => write_seq(f, a),
            ParamValue::DoubleArray(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_f64(f, *v)?;
                }
                write!(f, "]")
            }
            ParamValue::StringArray(a) => {
                write!(f, "[")?;
                for (i, s) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{s}\"")?;
                }
                write!(f, "]")
            }
        }
    }
}

fn write_seq<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    write!(f, "[")?;
    for (i, v) in items.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{v}")?;
    }
    write!(f, "]")
}

/// A named parameter with its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    pub value: ParamValue,
}
