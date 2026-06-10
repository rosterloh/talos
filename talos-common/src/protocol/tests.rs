use super::messages::{Request, Response};
use super::types::{
    DynValue, JointInfo, JointLimits, JointType, NodeInfo, ParamInfo, ParamValue, PoseInfo,
    StreamHeader, Timestamp, TopicFrame, TopicInfo, TopicSub,
};

fn round_trip_request(req: &Request) {
    let bytes = bincode::serialize(req).expect("serialize request");
    let decoded: Request = bincode::deserialize(&bytes).expect("deserialize request");
    assert_eq!(req, &decoded);
}

fn round_trip_response(resp: &Response) {
    let bytes = bincode::serialize(resp).expect("serialize response");
    let decoded: Response = bincode::deserialize(&bytes).expect("deserialize response");
    assert_eq!(resp, &decoded);
}

#[test]
fn request_list_topics() {
    round_trip_request(&Request::ListTopics);
}

#[test]
fn request_list_nodes() {
    round_trip_request(&Request::ListNodes);
}

#[test]
fn request_list_poses() {
    round_trip_request(&Request::ListPoses);
}

#[test]
fn request_set_joint_position() {
    round_trip_request(&Request::SetJointPosition {
        joint: "shoulder_pan".into(),
        position: 1.57,
    });
}

#[test]
fn request_execute_pose() {
    round_trip_request(&Request::ExecutePose {
        name: "home".into(),
    });
}

#[test]
fn response_topic_list() {
    round_trip_response(&Response::TopicList(vec![TopicInfo {
        name: "/odom".into(),
        type_name: "nav_msgs/msg/Odometry".into(),
        publisher_count: 1,
        subscriber_count: 2,
    }]));
}

#[test]
fn response_node_list() {
    round_trip_response(&Response::NodeList(vec![NodeInfo {
        name: "robot_state_publisher".into(),
        namespace: "/".into(),
        publishers: vec!["/tf".into()],
        subscribers: vec!["/joint_states".into()],
        services: vec!["/get_parameters".into()],
    }]));
}

#[test]
fn response_topic_data_with_nested_struct() {
    let data = DynValue::Struct {
        type_name: "Twist".into(),
        fields: vec![
            (
                "linear".into(),
                DynValue::Struct {
                    type_name: "Vector3".into(),
                    fields: vec![
                        ("x".into(), DynValue::F64(1.0)),
                        ("y".into(), DynValue::F64(0.0)),
                        ("z".into(), DynValue::F64(0.0)),
                    ],
                },
            ),
            (
                "angular".into(),
                DynValue::Struct {
                    type_name: "Vector3".into(),
                    fields: vec![
                        ("x".into(), DynValue::F64(0.0)),
                        ("y".into(), DynValue::F64(0.0)),
                        ("z".into(), DynValue::F64(0.1)),
                    ],
                },
            ),
        ],
    };
    round_trip_response(&Response::TopicData {
        topic: "/cmd_vel".into(),
        type_name: "geometry_msgs/msg/Twist".into(),
        stamp: Timestamp {
            sec: 1000,
            nanosec: 500,
        },
        data,
    });
}

#[test]
fn response_topic_data_with_arrays() {
    let data = DynValue::Struct {
        type_name: "JointState".into(),
        fields: vec![
            (
                "name".into(),
                DynValue::Array(vec![
                    DynValue::String("joint1".into()),
                    DynValue::String("joint2".into()),
                ]),
            ),
            (
                "position".into(),
                DynValue::Array(vec![DynValue::F64(1.0), DynValue::F64(2.0)]),
            ),
        ],
    };
    round_trip_response(&Response::TopicData {
        topic: "/joint_states".into(),
        type_name: "sensor_msgs/msg/JointState".into(),
        stamp: Timestamp {
            sec: 42,
            nanosec: 0,
        },
        data,
    });
}

#[test]
fn response_pose_list() {
    round_trip_response(&Response::PoseList(vec![PoseInfo {
        name: "home".into(),
        positions: vec![("shoulder_pan".into(), 0.0), ("elbow".into(), 1.57)],
    }]));
}

#[test]
fn response_error() {
    round_trip_response(&Response::Error("something went wrong".into()));
}

#[test]
fn response_ok() {
    round_trip_response(&Response::Ok("done".into()));
}

#[test]
fn request_subscribe() {
    round_trip_request(&Request::Subscribe {
        topics: vec!["/odom".into(), "/joint_states".into()],
    });
}

#[test]
fn request_unsubscribe() {
    round_trip_request(&Request::Unsubscribe {
        topics: vec!["/odom".into()],
    });
}

#[test]
fn response_subscribed() {
    round_trip_response(&Response::Subscribed {
        topics: vec![
            TopicSub {
                topic: "/odom".into(),
                type_name: "nav_msgs/msg/Odometry".into(),
            },
            TopicSub {
                topic: "/joint_states".into(),
                type_name: "sensor_msgs/msg/JointState".into(),
            },
        ],
    });
}

#[test]
fn response_unsubscribed() {
    round_trip_response(&Response::Unsubscribed {
        topics: vec!["/odom".into()],
    });
}

#[test]
fn topic_frame_round_trip() {
    let frame = TopicFrame {
        stamp: Timestamp {
            sec: 100,
            nanosec: 500_000,
        },
        data: DynValue::Struct {
            type_name: "Point".into(),
            fields: vec![
                ("x".into(), DynValue::F64(1.0)),
                ("y".into(), DynValue::F64(2.0)),
            ],
        },
    };
    let bytes = bincode::serialize(&frame).expect("serialize");
    let decoded: TopicFrame = bincode::deserialize(&bytes).expect("deserialize");
    assert_eq!(frame, decoded);
}

#[test]
fn stream_header_round_trip() {
    let header = StreamHeader {
        topic: "/odom".into(),
        type_name: "nav_msgs/msg/Odometry".into(),
    };
    let bytes = bincode::serialize(&header).expect("serialize");
    let decoded: StreamHeader = bincode::deserialize(&bytes).expect("deserialize");
    assert_eq!(header, decoded);
}

#[test]
fn topic_sub_round_trip() {
    let ts = TopicSub {
        topic: "/cmd_vel".into(),
        type_name: "geometry_msgs/msg/Twist".into(),
    };
    let bytes = bincode::serialize(&ts).expect("serialize");
    let decoded: TopicSub = bincode::deserialize(&bytes).expect("deserialize");
    assert_eq!(ts, decoded);
}

#[test]
fn dynvalue_all_primitives() {
    let values = vec![
        DynValue::Bool(true),
        DynValue::I8(-1),
        DynValue::I16(-256),
        DynValue::I32(-65536),
        DynValue::I64(-1_000_000),
        DynValue::U8(255),
        DynValue::U16(65535),
        DynValue::U32(4_294_967_295),
        DynValue::U64(u64::MAX),
        DynValue::F32(3.14),
        DynValue::F64(std::f64::consts::PI),
        DynValue::String("hello".into()),
        DynValue::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]),
    ];
    for val in &values {
        let bytes = bincode::serialize(val).expect("serialize");
        let decoded: DynValue = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(val, &decoded);
    }
}

#[test]
fn joint_info_round_trip() {
    let info = JointInfo {
        name: "shoulder_pan".into(),
        joint_type: JointType::Revolute,
        parent_link: "base_link".into(),
        child_link: "shoulder_link".into(),
        limits: Some(JointLimits {
            lower: -3.14,
            upper: 3.14,
            effort: 100.0,
            velocity: 1.0,
        }),
    };
    let bytes = bincode::serialize(&info).expect("serialize");
    let decoded: JointInfo = bincode::deserialize(&bytes).expect("deserialize");
    assert_eq!(info, decoded);
}

#[test]
fn request_list_parameters() {
    round_trip_request(&Request::ListParameters {
        node: "/talos_agent".into(),
    });
}

#[test]
fn request_get_parameters() {
    round_trip_request(&Request::GetParameters {
        node: "/talos_agent".into(),
        names: vec!["use_sim_time".into(), "rate".into()],
    });
}

#[test]
fn request_set_parameter() {
    round_trip_request(&Request::SetParameter {
        node: "/talos_agent".into(),
        name: "rate".into(),
        value: ParamValue::Double(50.0),
    });
}

#[test]
fn response_parameters() {
    round_trip_response(&Response::Parameters {
        node: "/talos_agent".into(),
        parameters: vec![
            ParamInfo {
                name: "use_sim_time".into(),
                value: ParamValue::Bool(false),
            },
            ParamInfo {
                name: "rate".into(),
                value: ParamValue::Double(50.0),
            },
            ParamInfo {
                name: "frames".into(),
                value: ParamValue::StringArray(vec!["base".into(), "tool".into()]),
            },
        ],
    });
}

#[test]
fn response_parameter_set() {
    round_trip_response(&Response::ParameterSet {
        node: "/talos_agent".into(),
        name: "rate".into(),
        successful: true,
        reason: String::new(),
    });
}

#[test]
fn param_value_all_variants_round_trip() {
    for value in [
        ParamValue::NotSet,
        ParamValue::Bool(true),
        ParamValue::Integer(-42),
        ParamValue::Double(3.5),
        ParamValue::String("hello".into()),
        ParamValue::ByteArray(vec![1, 2, 3]),
        ParamValue::BoolArray(vec![true, false]),
        ParamValue::IntegerArray(vec![1, 2, 3]),
        ParamValue::DoubleArray(vec![1.0, 2.5]),
        ParamValue::StringArray(vec!["a".into(), "b".into()]),
    ] {
        let info = ParamInfo {
            name: "p".into(),
            value,
        };
        let bytes = bincode::serialize(&info).expect("serialize param");
        let decoded: ParamInfo = bincode::deserialize(&bytes).expect("deserialize param");
        assert_eq!(info, decoded);
    }
}

#[test]
fn param_value_parse_infers_scalar_types() {
    assert_eq!(ParamValue::parse("true"), ParamValue::Bool(true));
    assert_eq!(ParamValue::parse("False"), ParamValue::Bool(false));
    assert_eq!(ParamValue::parse("42"), ParamValue::Integer(42));
    assert_eq!(ParamValue::parse("-7"), ParamValue::Integer(-7));
    assert_eq!(ParamValue::parse("3.14"), ParamValue::Double(3.14));
    assert_eq!(ParamValue::parse("1.0"), ParamValue::Double(1.0));
    assert_eq!(
        ParamValue::parse("hello"),
        ParamValue::String("hello".into())
    );
    // Quoted digits stay a string rather than becoming a number.
    assert_eq!(
        ParamValue::parse("\"123\""),
        ParamValue::String("123".into())
    );
}

#[test]
fn param_value_parse_infers_arrays() {
    assert_eq!(
        ParamValue::parse("[1, 2, 3]"),
        ParamValue::IntegerArray(vec![1, 2, 3])
    );
    assert_eq!(
        ParamValue::parse("[1.5, 2.0]"),
        ParamValue::DoubleArray(vec![1.5, 2.0])
    );
    assert_eq!(
        ParamValue::parse("[true, false]"),
        ParamValue::BoolArray(vec![true, false])
    );
    assert_eq!(
        ParamValue::parse("[a, b]"),
        ParamValue::StringArray(vec!["a".into(), "b".into()])
    );
    assert_eq!(ParamValue::parse("[]"), ParamValue::StringArray(vec![]));
}

#[test]
fn param_value_parse_preserving_type_keeps_existing_string_scalars() {
    assert_eq!(
        ParamValue::parse_preserving_type("42", &ParamValue::String("old".into())),
        ParamValue::String("42".into())
    );
    assert_eq!(
        ParamValue::parse_preserving_type("false", &ParamValue::String("old".into())),
        ParamValue::String("false".into())
    );
    assert_eq!(
        ParamValue::parse_preserving_type("[1, 2]", &ParamValue::String("old".into())),
        ParamValue::String("[1, 2]".into())
    );
}

#[test]
fn param_value_parse_preserving_type_keeps_unchanged_display_only_values() {
    let bytes = ParamValue::ByteArray(vec![1, 2, 3]);
    assert_eq!(
        ParamValue::parse_preserving_type(&bytes.to_string(), &bytes),
        bytes
    );

    let strings = ParamValue::StringArray(vec!["a,b".into()]);
    assert_eq!(
        ParamValue::parse_preserving_type(&strings.to_string(), &strings),
        strings
    );
}

#[test]
fn param_value_display_round_trips_through_parse() {
    for value in [
        ParamValue::Bool(true),
        ParamValue::Integer(42),
        ParamValue::Double(1.0),
        ParamValue::Double(3.14),
        ParamValue::String("hello".into()),
        ParamValue::IntegerArray(vec![1, 2, 3]),
        ParamValue::DoubleArray(vec![1.0, 2.5]),
        ParamValue::BoolArray(vec![true, false]),
    ] {
        let shown = value.to_string();
        assert_eq!(ParamValue::parse(&shown), value, "round trip via '{shown}'");
    }
}
