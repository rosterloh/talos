use talos_common::protocol::types::{DynValue, Timestamp};

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
                                msg.pose.covariance.iter().map(|&v| DynValue::F64(v)).collect(),
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
