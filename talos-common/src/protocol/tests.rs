use super::messages::{Request, Response};
use super::types::{
    DynValue, JointInfo, JointLimits, JointType, NodeInfo, PoseInfo, Timestamp, TopicInfo,
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
