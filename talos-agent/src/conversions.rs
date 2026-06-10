use std::error::Error;

use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{DynValue, ParamValue, Timestamp};
use tokio::sync::mpsc;

pub type TopicSender = mpsc::UnboundedSender<Response>;
type SubscribeResult = Result<(), Box<dyn Error + Send + Sync>>;
type SubscribeFn = for<'a> fn(
    &rclrs::Node,
    rclrs::PrimitiveOptions<'a>,
    String,
    String,
    TopicSender,
) -> SubscribeResult;

pub struct MessageTypeEntry {
    pub type_name: &'static str,
    subscribe: SubscribeFn,
}

impl MessageTypeEntry {
    pub fn subscribe(
        &self,
        node: &rclrs::Node,
        opts: rclrs::PrimitiveOptions<'_>,
        topic: String,
        type_name: String,
        tx: TopicSender,
    ) -> SubscribeResult {
        (self.subscribe)(node, opts, topic, type_name, tx)
    }
}

fn send_topic_data(
    tx: &TopicSender,
    topic: &str,
    type_name: &str,
    stamp: Timestamp,
    data: DynValue,
) {
    let _ = tx.send(Response::TopicData {
        topic: topic.to_string(),
        type_name: type_name.to_string(),
        stamp,
        data,
    });
}

macro_rules! message_registry {
    (
        $(
            $subscribe_fn:ident: $type_name:literal => $msg_ty:ty,
            stamp = $stamp:expr,
            convert = $convert:path;
        )+
    ) => {
        const SUPPORTED_MESSAGE_TYPES: &[&str] = &[
            $($type_name,)+
        ];

        const MESSAGE_TYPE_ENTRIES: &[MessageTypeEntry] = &[
            $(
                MessageTypeEntry {
                    type_name: $type_name,
                    subscribe: $subscribe_fn,
                },
            )+
        ];

        $(
            fn $subscribe_fn(
                node: &rclrs::Node,
                opts: rclrs::PrimitiveOptions<'_>,
                topic: String,
                type_name: String,
                tx: TopicSender,
            ) -> SubscribeResult {
                node.create_subscription::<$msg_ty, _>(opts, move |msg: $msg_ty| {
                    let stamp = ($stamp)(&msg);
                    let data = $convert(&msg);
                    send_topic_data(&tx, &topic, &type_name, stamp, data);
                })?;
                Ok(())
            }
        )+
    };
}

pub fn timestamp_from_builtin(t: &builtin_interfaces::msg::Time) -> Timestamp {
    Timestamp {
        sec: t.sec,
        nanosec: t.nanosec,
    }
}

fn header_to_dynvalue(h: &std_msgs::msg::Header) -> DynValue {
    DynValue::Struct {
        type_name: "Header".into(),
        fields: vec![
            (
                "stamp".into(),
                DynValue::Struct {
                    type_name: "Time".into(),
                    fields: vec![
                        ("sec".into(), DynValue::I32(h.stamp.sec)),
                        ("nanosec".into(), DynValue::U32(h.stamp.nanosec)),
                    ],
                },
            ),
            ("frame_id".into(), DynValue::String(h.frame_id.clone())),
        ],
    }
}

fn vector3_to_dynvalue(v: &geometry_msgs::msg::Vector3) -> DynValue {
    DynValue::Struct {
        type_name: "Vector3".into(),
        fields: vec![
            ("x".into(), DynValue::F64(v.x)),
            ("y".into(), DynValue::F64(v.y)),
            ("z".into(), DynValue::F64(v.z)),
        ],
    }
}

fn quaternion_to_dynvalue(q: &geometry_msgs::msg::Quaternion) -> DynValue {
    DynValue::Struct {
        type_name: "Quaternion".into(),
        fields: vec![
            ("x".into(), DynValue::F64(q.x)),
            ("y".into(), DynValue::F64(q.y)),
            ("z".into(), DynValue::F64(q.z)),
            ("w".into(), DynValue::F64(q.w)),
        ],
    }
}

fn point_to_dynvalue(p: &geometry_msgs::msg::Point) -> DynValue {
    DynValue::Struct {
        type_name: "Point".into(),
        fields: vec![
            ("x".into(), DynValue::F64(p.x)),
            ("y".into(), DynValue::F64(p.y)),
            ("z".into(), DynValue::F64(p.z)),
        ],
    }
}

fn pose_to_dynvalue(p: &geometry_msgs::msg::Pose) -> DynValue {
    DynValue::Struct {
        type_name: "Pose".into(),
        fields: vec![
            ("position".into(), point_to_dynvalue(&p.position)),
            ("orientation".into(), quaternion_to_dynvalue(&p.orientation)),
        ],
    }
}

fn twist_to_dynvalue(t: &geometry_msgs::msg::Twist) -> DynValue {
    DynValue::Struct {
        type_name: "Twist".into(),
        fields: vec![
            ("linear".into(), vector3_to_dynvalue(&t.linear)),
            ("angular".into(), vector3_to_dynvalue(&t.angular)),
        ],
    }
}

// --- Top-level message conversion functions ---

pub fn odometry_to_dynvalue(msg: &nav_msgs::msg::Odometry) -> DynValue {
    DynValue::Struct {
        type_name: "Odometry".into(),
        fields: vec![
            ("header".into(), header_to_dynvalue(&msg.header)),
            (
                "child_frame_id".into(),
                DynValue::String(msg.child_frame_id.clone()),
            ),
            (
                "pose".into(),
                DynValue::Struct {
                    type_name: "PoseWithCovariance".into(),
                    fields: vec![
                        ("pose".into(), pose_to_dynvalue(&msg.pose.pose)),
                        (
                            "covariance".into(),
                            DynValue::Array(
                                msg.pose
                                    .covariance
                                    .iter()
                                    .map(|&v| DynValue::F64(v))
                                    .collect(),
                            ),
                        ),
                    ],
                },
            ),
            (
                "twist".into(),
                DynValue::Struct {
                    type_name: "TwistWithCovariance".into(),
                    fields: vec![
                        ("twist".into(), twist_to_dynvalue(&msg.twist.twist)),
                        (
                            "covariance".into(),
                            DynValue::Array(
                                msg.twist
                                    .covariance
                                    .iter()
                                    .map(|&v| DynValue::F64(v))
                                    .collect(),
                            ),
                        ),
                    ],
                },
            ),
        ],
    }
}

pub fn twist_msg_to_dynvalue(msg: &geometry_msgs::msg::Twist) -> DynValue {
    twist_to_dynvalue(msg)
}

pub fn string_to_dynvalue(msg: &std_msgs::msg::String) -> DynValue {
    DynValue::String(msg.data.clone())
}

pub fn joint_state_to_dynvalue(msg: &sensor_msgs::msg::JointState) -> DynValue {
    DynValue::Struct {
        type_name: "JointState".into(),
        fields: vec![
            ("header".into(), header_to_dynvalue(&msg.header)),
            (
                "name".into(),
                DynValue::Array(
                    msg.name
                        .iter()
                        .map(|s| DynValue::String(s.clone()))
                        .collect(),
                ),
            ),
            (
                "position".into(),
                DynValue::Array(msg.position.iter().map(|&v| DynValue::F64(v)).collect()),
            ),
            (
                "velocity".into(),
                DynValue::Array(msg.velocity.iter().map(|&v| DynValue::F64(v)).collect()),
            ),
            (
                "effort".into(),
                DynValue::Array(msg.effort.iter().map(|&v| DynValue::F64(v)).collect()),
            ),
        ],
    }
}

pub fn laser_scan_to_dynvalue(msg: &sensor_msgs::msg::LaserScan) -> DynValue {
    DynValue::Struct {
        type_name: "LaserScan".into(),
        fields: vec![
            ("header".into(), header_to_dynvalue(&msg.header)),
            ("angle_min".into(), DynValue::F64(msg.angle_min as f64)),
            ("angle_max".into(), DynValue::F64(msg.angle_max as f64)),
            (
                "angle_increment".into(),
                DynValue::F64(msg.angle_increment as f64),
            ),
            (
                "time_increment".into(),
                DynValue::F64(msg.time_increment as f64),
            ),
            ("scan_time".into(), DynValue::F64(msg.scan_time as f64)),
            ("range_min".into(), DynValue::F64(msg.range_min as f64)),
            ("range_max".into(), DynValue::F64(msg.range_max as f64)),
            (
                "ranges".into(),
                DynValue::Array(
                    msg.ranges
                        .iter()
                        .map(|&v| DynValue::F64(v as f64))
                        .collect(),
                ),
            ),
            (
                "intensities".into(),
                DynValue::Array(
                    msg.intensities
                        .iter()
                        .map(|&v| DynValue::F64(v as f64))
                        .collect(),
                ),
            ),
        ],
    }
}

pub fn imu_to_dynvalue(msg: &sensor_msgs::msg::Imu) -> DynValue {
    DynValue::Struct {
        type_name: "Imu".into(),
        fields: vec![
            ("header".into(), header_to_dynvalue(&msg.header)),
            (
                "orientation".into(),
                quaternion_to_dynvalue(&msg.orientation),
            ),
            (
                "orientation_covariance".into(),
                DynValue::Array(
                    msg.orientation_covariance
                        .iter()
                        .map(|&v| DynValue::F64(v))
                        .collect(),
                ),
            ),
            (
                "angular_velocity".into(),
                vector3_to_dynvalue(&msg.angular_velocity),
            ),
            (
                "angular_velocity_covariance".into(),
                DynValue::Array(
                    msg.angular_velocity_covariance
                        .iter()
                        .map(|&v| DynValue::F64(v))
                        .collect(),
                ),
            ),
            (
                "linear_acceleration".into(),
                vector3_to_dynvalue(&msg.linear_acceleration),
            ),
            (
                "linear_acceleration_covariance".into(),
                DynValue::Array(
                    msg.linear_acceleration_covariance
                        .iter()
                        .map(|&v| DynValue::F64(v))
                        .collect(),
                ),
            ),
        ],
    }
}

pub fn pose_stamped_to_dynvalue(msg: &geometry_msgs::msg::PoseStamped) -> DynValue {
    DynValue::Struct {
        type_name: "PoseStamped".into(),
        fields: vec![
            ("header".into(), header_to_dynvalue(&msg.header)),
            ("pose".into(), pose_to_dynvalue(&msg.pose)),
        ],
    }
}

message_registry! {
    subscribe_odometry: "nav_msgs/msg/Odometry" => nav_msgs::msg::Odometry,
        stamp = |msg: &nav_msgs::msg::Odometry| timestamp_from_builtin(&msg.header.stamp),
        convert = odometry_to_dynvalue;
    subscribe_twist: "geometry_msgs/msg/Twist" => geometry_msgs::msg::Twist,
        stamp = |_msg: &geometry_msgs::msg::Twist| Timestamp { sec: 0, nanosec: 0 },
        convert = twist_msg_to_dynvalue;
    subscribe_string: "std_msgs/msg/String" => std_msgs::msg::String,
        stamp = |_msg: &std_msgs::msg::String| Timestamp { sec: 0, nanosec: 0 },
        convert = string_to_dynvalue;
    subscribe_joint_state: "sensor_msgs/msg/JointState" => sensor_msgs::msg::JointState,
        stamp = |msg: &sensor_msgs::msg::JointState| timestamp_from_builtin(&msg.header.stamp),
        convert = joint_state_to_dynvalue;
    subscribe_laser_scan: "sensor_msgs/msg/LaserScan" => sensor_msgs::msg::LaserScan,
        stamp = |msg: &sensor_msgs::msg::LaserScan| timestamp_from_builtin(&msg.header.stamp),
        convert = laser_scan_to_dynvalue;
    subscribe_imu: "sensor_msgs/msg/Imu" => sensor_msgs::msg::Imu,
        stamp = |msg: &sensor_msgs::msg::Imu| timestamp_from_builtin(&msg.header.stamp),
        convert = imu_to_dynvalue;
    subscribe_pose_stamped: "geometry_msgs/msg/PoseStamped" => geometry_msgs::msg::PoseStamped,
        stamp = |msg: &geometry_msgs::msg::PoseStamped| timestamp_from_builtin(&msg.header.stamp),
        convert = pose_stamped_to_dynvalue;
    subscribe_log: "rcl_interfaces/msg/Log" => rcl_interfaces::msg::Log,
        stamp = |msg: &rcl_interfaces::msg::Log| timestamp_from_builtin(&msg.stamp),
        convert = log_to_dynvalue;
}

pub fn supported_message_types() -> &'static [&'static str] {
    SUPPORTED_MESSAGE_TYPES
}

pub fn message_type_entry(type_name: &str) -> Option<&'static MessageTypeEntry> {
    MESSAGE_TYPE_ENTRIES
        .iter()
        .find(|entry| entry.type_name == type_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_names(dv: &DynValue) -> Vec<&str> {
        match dv {
            DynValue::Struct { fields, .. } => fields.iter().map(|(k, _)| k.as_str()).collect(),
            _ => panic!("expected DynValue::Struct"),
        }
    }

    fn array_len(dv: &DynValue, field: &str) -> usize {
        match dv {
            DynValue::Struct { fields, .. } => {
                let val = fields.iter().find(|(k, _)| k == field).map(|(_, v)| v);
                match val.expect("field not found") {
                    DynValue::Array(a) => a.len(),
                    _ => panic!("field {field} is not an Array"),
                }
            }
            _ => panic!("expected DynValue::Struct"),
        }
    }

    #[test]
    fn laser_scan_field_names_and_range_length() {
        let mut msg = sensor_msgs::msg::LaserScan::default();
        msg.ranges = vec![1.0, 2.0, 3.0];
        msg.intensities = vec![0.1, 0.2, 0.3];
        let dv = laser_scan_to_dynvalue(&msg);
        let names = field_names(&dv);
        assert_eq!(
            names,
            &[
                "header",
                "angle_min",
                "angle_max",
                "angle_increment",
                "time_increment",
                "scan_time",
                "range_min",
                "range_max",
                "ranges",
                "intensities"
            ]
        );
        assert_eq!(array_len(&dv, "ranges"), 3);
        assert_eq!(array_len(&dv, "intensities"), 3);
    }

    #[test]
    fn imu_field_names_and_covariance_lengths() {
        let msg = sensor_msgs::msg::Imu::default();
        let dv = imu_to_dynvalue(&msg);
        let names = field_names(&dv);
        assert_eq!(
            names,
            &[
                "header",
                "orientation",
                "orientation_covariance",
                "angular_velocity",
                "angular_velocity_covariance",
                "linear_acceleration",
                "linear_acceleration_covariance"
            ]
        );
        assert_eq!(array_len(&dv, "orientation_covariance"), 9);
        assert_eq!(array_len(&dv, "angular_velocity_covariance"), 9);
        assert_eq!(array_len(&dv, "linear_acceleration_covariance"), 9);
    }

    #[test]
    fn pose_stamped_field_names() {
        let msg = geometry_msgs::msg::PoseStamped::default();
        let dv = pose_stamped_to_dynvalue(&msg);
        let names = field_names(&dv);
        assert_eq!(names, &["header", "pose"]);
    }

    #[test]
    fn supported_message_registry_lists_current_types_once() {
        let supported = supported_message_types();
        assert_eq!(
            supported,
            &[
                "nav_msgs/msg/Odometry",
                "geometry_msgs/msg/Twist",
                "std_msgs/msg/String",
                "sensor_msgs/msg/JointState",
                "sensor_msgs/msg/LaserScan",
                "sensor_msgs/msg/Imu",
                "geometry_msgs/msg/PoseStamped",
                "rcl_interfaces/msg/Log",
            ]
        );

        let mut sorted = supported.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), supported.len());

        for type_name in supported {
            assert!(message_type_entry(type_name).is_some());
        }
        assert!(message_type_entry("geometry_msgs/msg/Pose").is_none());
    }
}

pub fn log_to_dynvalue(msg: &rcl_interfaces::msg::Log) -> DynValue {
    let level = match msg.level {
        10 => "DEBUG",
        20 => "INFO",
        30 => "WARN",
        40 => "ERROR",
        50 => "FATAL",
        _ => "UNKNOWN",
    };
    DynValue::Struct {
        type_name: "Log".into(),
        fields: vec![
            (
                "stamp".into(),
                DynValue::Struct {
                    type_name: "Time".into(),
                    fields: vec![
                        ("sec".into(), DynValue::I32(msg.stamp.sec)),
                        ("nanosec".into(), DynValue::U32(msg.stamp.nanosec)),
                    ],
                },
            ),
            ("level".into(), DynValue::String(level.into())),
            ("name".into(), DynValue::String(msg.name.clone())),
            ("msg".into(), DynValue::String(msg.msg.clone())),
            ("file".into(), DynValue::String(msg.file.clone())),
            ("function".into(), DynValue::String(msg.function.clone())),
            ("line".into(), DynValue::U32(msg.line)),
        ],
    }
}

// --- ROS 2 parameter value conversion ---
//
// Tags mirror `rcl_interfaces/msg/ParameterType`. We match on the raw `u8`
// rather than the generated constants so the mapping is explicit and stable.
const PARAMETER_NOT_SET: u8 = 0;
const PARAMETER_BOOL: u8 = 1;
const PARAMETER_INTEGER: u8 = 2;
const PARAMETER_DOUBLE: u8 = 3;
const PARAMETER_STRING: u8 = 4;
const PARAMETER_BYTE_ARRAY: u8 = 5;
const PARAMETER_BOOL_ARRAY: u8 = 6;
const PARAMETER_INTEGER_ARRAY: u8 = 7;
const PARAMETER_DOUBLE_ARRAY: u8 = 8;
const PARAMETER_STRING_ARRAY: u8 = 9;

/// Convert a ROS 2 `ParameterValue` into the transport-agnostic [`ParamValue`].
pub fn param_value_from_ros(v: &rcl_interfaces::msg::ParameterValue) -> ParamValue {
    match v.type_ {
        PARAMETER_BOOL => ParamValue::Bool(v.bool_value),
        PARAMETER_INTEGER => ParamValue::Integer(v.integer_value),
        PARAMETER_DOUBLE => ParamValue::Double(v.double_value),
        PARAMETER_STRING => ParamValue::String(v.string_value.clone()),
        PARAMETER_BYTE_ARRAY => ParamValue::ByteArray(v.byte_array_value.clone()),
        PARAMETER_BOOL_ARRAY => ParamValue::BoolArray(v.bool_array_value.clone()),
        PARAMETER_INTEGER_ARRAY => ParamValue::IntegerArray(v.integer_array_value.clone()),
        PARAMETER_DOUBLE_ARRAY => ParamValue::DoubleArray(v.double_array_value.clone()),
        PARAMETER_STRING_ARRAY => ParamValue::StringArray(v.string_array_value.clone()),
        _ => ParamValue::NotSet,
    }
}

/// Convert a [`ParamValue`] into a ROS 2 `ParameterValue` with the matching
/// `type` tag and populated field.
pub fn param_value_to_ros(v: &ParamValue) -> rcl_interfaces::msg::ParameterValue {
    let mut out = rcl_interfaces::msg::ParameterValue::default();
    match v {
        ParamValue::NotSet => out.type_ = PARAMETER_NOT_SET,
        ParamValue::Bool(b) => {
            out.type_ = PARAMETER_BOOL;
            out.bool_value = *b;
        }
        ParamValue::Integer(i) => {
            out.type_ = PARAMETER_INTEGER;
            out.integer_value = *i;
        }
        ParamValue::Double(d) => {
            out.type_ = PARAMETER_DOUBLE;
            out.double_value = *d;
        }
        ParamValue::String(s) => {
            out.type_ = PARAMETER_STRING;
            out.string_value = s.clone();
        }
        ParamValue::ByteArray(a) => {
            out.type_ = PARAMETER_BYTE_ARRAY;
            out.byte_array_value = a.clone();
        }
        ParamValue::BoolArray(a) => {
            out.type_ = PARAMETER_BOOL_ARRAY;
            out.bool_array_value = a.clone();
        }
        ParamValue::IntegerArray(a) => {
            out.type_ = PARAMETER_INTEGER_ARRAY;
            out.integer_array_value = a.clone();
        }
        ParamValue::DoubleArray(a) => {
            out.type_ = PARAMETER_DOUBLE_ARRAY;
            out.double_array_value = a.clone();
        }
        ParamValue::StringArray(a) => {
            out.type_ = PARAMETER_STRING_ARRAY;
            out.string_array_value = a.clone();
        }
    }
    out
}
