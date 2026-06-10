//! Integration tests for Talos agent subscription routing.
//!
//! Run UDS-only tests:
//!   cargo test -p talos-agent --test integration
//!
//! Run with QUIC:
//!   cargo test -p talos-agent --test integration --features quic

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use talos_common::config::{
    AgentConfig, QosProfile, SubscriptionConfig, TransportSettings, UdsTransportConfig,
};
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::{DynValue, ParamValue, Timestamp};
use talos_common::session::ProtocolClient;
use talos_common::session::uds::UdsProtocolClient;
use tempfile::TempDir;
use tokio::sync::Mutex;

use talos_agent::router::TopicRouter;
use talos_agent::server::RouterHandle;
use talos_agent::{GraphHandle, JointPublisher};

/// Minimal `AgentConfig` with two configured subscriptions.
fn test_config_uds(socket_path: &str) -> Arc<AgentConfig> {
    Arc::new(AgentConfig {
        transport: TransportSettings {
            uds: Some(UdsTransportConfig {
                socket_path: socket_path.to_string(),
            }),
            quic: None,
        },
        subscriptions: vec![
            SubscriptionConfig {
                topic: "/odom".to_string(),
                msg_type: "nav_msgs/msg/Odometry".to_string(),
                qos: QosProfile::Default,
            },
            SubscriptionConfig {
                topic: "/joint_states".to_string(),
                msg_type: "sensor_msgs/msg/JointState".to_string(),
                qos: QosProfile::Default,
            },
        ],
        control: None,
        poses: {
            let mut poses = HashMap::new();
            poses.insert(
                "home".to_string(),
                HashMap::from([("joint_a".to_string(), 0.0), ("joint_b".to_string(), 1.0)]),
            );
            poses
        },
    })
}

fn inject(router: &RouterHandle, topic: &str) {
    let response = Response::TopicData {
        topic: topic.to_string(),
        type_name: "test/Type".to_string(),
        stamp: Timestamp { sec: 1, nanosec: 0 },
        data: DynValue::Bool(true),
    };
    // Use try_lock to avoid blocking in test helpers
    if let Ok(r) = router.try_lock() {
        r.route(&response);
    }
}

fn make_router() -> RouterHandle {
    Arc::new(Mutex::new(TopicRouter::new()))
}

fn make_joint_publisher() -> JointPublisher {
    Arc::new(Mutex::new(None))
}

fn make_graph_handle() -> GraphHandle {
    Arc::new(Mutex::new(None))
}

/// Spawn the UDS server in the background and return its router handle.
async fn spawn_uds_server(config: Arc<AgentConfig>) -> RouterHandle {
    let router = make_router();
    let jp = make_joint_publisher();
    let graph = make_graph_handle();
    let r = Arc::clone(&router);
    let jp2 = Arc::clone(&jp);
    tokio::spawn(async move {
        let _ = talos_agent::server::run(config, r, jp2, graph).await;
    });
    // Give the listener time to bind
    tokio::time::sleep(Duration::from_millis(80)).await;
    router
}

// ── 8.1: UDS client subscribes to a subset of topics ────────────────────────

/// A UDS client that subscribes only to /odom should receive /odom frames but
/// NOT /joint_states frames injected directly into the router.
#[tokio::test]
async fn uds_subscriber_receives_only_subscribed_topics() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("s8_1.sock").to_string_lossy().into_owned();

    let config = test_config_uds(&path);
    let router = spawn_uds_server(config).await;

    let mut client = UdsProtocolClient::connect(&path).await.unwrap();
    client.subscribe(&["/odom".to_string()]).await.unwrap();

    // Inject /odom × 1, /joint_states × 1, /odom × 1
    inject(&router, "/odom");
    inject(&router, "/joint_states"); // should be silently dropped
    inject(&router, "/odom");

    let timeout = Duration::from_secs(2);

    let (t1, _) = tokio::time::timeout(timeout, client.recv_data())
        .await
        .expect("timed out on frame 1")
        .expect("recv_data error");
    assert_eq!(t1, "/odom", "expected /odom on frame 1, got {t1}");

    let (t2, _) = tokio::time::timeout(timeout, client.recv_data())
        .await
        .expect("timed out on frame 2")
        .expect("recv_data error");
    assert_eq!(t2, "/odom", "expected /odom on frame 2, got {t2}");
}

#[tokio::test]
async fn uds_list_poses_returns_configured_poses() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("poses.sock").to_string_lossy().into_owned();

    let config = test_config_uds(&path);
    spawn_uds_server(config).await;

    let mut client = UdsProtocolClient::connect(&path).await.unwrap();
    let response = client.request(Request::ListPoses).await.unwrap();

    match response {
        Response::PoseList(poses) => {
            assert_eq!(poses.len(), 1);
            assert_eq!(poses[0].name, "home");
            assert_eq!(poses[0].positions.len(), 2);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

/// Without a live ROS 2 graph (graph handle is `None` in these tests), the
/// parameter handlers must report the node as unavailable rather than hang or
/// panic. This also exercises the new dispatch arms over a real UDS round-trip.
#[tokio::test]
async fn uds_list_parameters_without_graph_returns_error() {
    let dir = TempDir::new().unwrap();
    let path = dir
        .path()
        .join("listparams.sock")
        .to_string_lossy()
        .into_owned();

    let config = test_config_uds(&path);
    spawn_uds_server(config).await;

    let mut client = UdsProtocolClient::connect(&path).await.unwrap();
    let response = client
        .request(Request::ListParameters {
            node: "/some_node".to_string(),
        })
        .await
        .unwrap();

    match response {
        Response::Error(msg) => assert!(
            msg.contains("not available"),
            "expected node-unavailable error, got: {msg}"
        ),
        other => panic!("unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn uds_set_parameter_without_graph_returns_error() {
    let dir = TempDir::new().unwrap();
    let path = dir
        .path()
        .join("setparam.sock")
        .to_string_lossy()
        .into_owned();

    let config = test_config_uds(&path);
    spawn_uds_server(config).await;

    let mut client = UdsProtocolClient::connect(&path).await.unwrap();
    let response = client
        .request(Request::SetParameter {
            node: "/some_node".to_string(),
            name: "rate".to_string(),
            value: ParamValue::Integer(10),
        })
        .await
        .unwrap();

    match response {
        Response::Error(msg) => assert!(
            msg.contains("not available"),
            "expected node-unavailable error, got: {msg}"
        ),
        other => panic!("unexpected response: {other:?}"),
    }
}

/// End-to-end parameter round trip against a live ROS 2 node.
///
/// Spins a real `rclrs` node (mirroring the bridge node) that declares a
/// parameter and, by default, hosts the standard `rcl_interfaces` parameter
/// services. The agent's handlers create service clients on that same node and
/// drive list/get/set through DDS, exactly as in production.
#[tokio::test]
async fn uds_parameter_round_trip_against_live_node() {
    use rclrs::CreateBasicExecutor;

    const NODE: &str = "/talos_param_live_test";

    // The rclrs context, executor, node, and parameter all live on the spin
    // thread; the executor blocks in `spin`, so it must not run on the async
    // runtime. A clone of the node handle is sent back for the graph handle.
    let (node_tx, node_rx) = std::sync::mpsc::channel::<rclrs::Node>();
    std::thread::spawn(move || {
        let context = rclrs::Context::default_from_env().expect("rclrs context");
        let mut executor = context.create_basic_executor();
        let node = executor
            .create_node("talos_param_live_test")
            .expect("create node");
        let _rate = node
            .declare_parameter("rate")
            .default(10_i64)
            .mandatory()
            .expect("declare rate");
        node_tx.send(Arc::clone(&node)).expect("send node handle");
        // Keep `_rate` alive for the lifetime of the spin so the parameter
        // stays declared.
        executor.spin(rclrs::SpinOptions::default());
        drop(_rate);
    });
    let node = node_rx.recv().expect("receive node handle");

    let dir = TempDir::new().unwrap();
    let path = dir
        .path()
        .join("param_live.sock")
        .to_string_lossy()
        .into_owned();
    let config = test_config_uds(&path);

    let graph: GraphHandle = Arc::new(Mutex::new(Some(node)));
    tokio::spawn(async move {
        let _ =
            talos_agent::server::run(config, make_router(), make_joint_publisher(), graph).await;
    });
    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut client = UdsProtocolClient::connect(&path).await.unwrap();

    // List: the declared parameter shows up with its initial value.
    let response = client
        .request(Request::ListParameters { node: NODE.into() })
        .await
        .unwrap();
    let listed = match response {
        Response::Parameters { parameters, .. } => parameters,
        other => panic!("list: unexpected response: {other:?}"),
    };
    let rate = listed
        .iter()
        .find(|p| p.name == "rate")
        .expect("'rate' present in parameter list");
    assert_eq!(rate.value, ParamValue::Integer(10), "initial value");

    // Set the parameter to a new value.
    let response = client
        .request(Request::SetParameter {
            node: NODE.into(),
            name: "rate".into(),
            value: ParamValue::Integer(50),
        })
        .await
        .unwrap();
    match response {
        Response::ParameterSet {
            successful, reason, ..
        } => assert!(successful, "set rejected: {reason}"),
        other => panic!("set: unexpected response: {other:?}"),
    }

    // Get: the new value is reflected back.
    let response = client
        .request(Request::GetParameters {
            node: NODE.into(),
            names: vec!["rate".into()],
        })
        .await
        .unwrap();
    match response {
        Response::Parameters { parameters, .. } => {
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].name, "rate");
            assert_eq!(
                parameters[0].value,
                ParamValue::Integer(50),
                "value after set"
            );
        }
        other => panic!("get: unexpected response: {other:?}"),
    }
}

#[cfg(feature = "quic")]
#[tokio::test]
async fn quic_list_poses_returns_configured_poses() {
    use talos_common::config::QuicTransportConfig;
    use talos_common::session::QuicProtocolClient;
    use talos_common::transport::quic::QuicTransport;

    let dir = TempDir::new().unwrap();
    let path = dir
        .path()
        .join("poses-quic.sock")
        .to_string_lossy()
        .into_owned();

    let mut config = (*test_config_uds(&path)).clone();
    config.transport.quic = Some(QuicTransportConfig {
        bind_addr: "127.0.0.1:0".to_string(),
        cert_path: None,
        key_path: None,
    });
    let config = Arc::new(config);
    let router = make_router();

    let quic_cfg = config.transport.quic.as_ref().unwrap();
    let endpoint = QuicTransport::bind(quic_cfg).await.unwrap();
    let quic_addr = endpoint.local_addr().unwrap();

    {
        let r = Arc::clone(&router);
        let jp = make_joint_publisher();
        let cfg = Arc::clone(&config);
        tokio::spawn(async move {
            while let Some(inc) = endpoint.accept().await {
                if let Ok(conn) = inc.await {
                    let r2 = Arc::clone(&r);
                    let j2 = Arc::clone(&jp);
                    let cfg2 = Arc::clone(&cfg);
                    let graph = make_graph_handle();
                    tokio::spawn(talos_agent::server::handle_quic_client(
                        conn, cfg2, r2, j2, graph,
                    ));
                }
            }
        });
    }

    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut client = QuicProtocolClient::connect(&quic_addr.to_string())
        .await
        .unwrap();
    let response = client.request(Request::ListPoses).await.unwrap();

    match response {
        Response::PoseList(poses) => {
            assert_eq!(poses.len(), 1);
            assert_eq!(poses[0].name, "home");
            assert_eq!(poses[0].positions.len(), 2);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

// ── 8.2: QUIC client subscribes and receives data on uni streams ─────────────

#[cfg(feature = "quic")]
#[tokio::test]
async fn quic_client_subscribes_and_receives_data() {
    use talos_common::config::QuicTransportConfig;
    use talos_common::session::QuicProtocolClient;
    use talos_common::transport::quic::QuicTransport;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("s8_2.sock").to_string_lossy().into_owned();

    let config = Arc::new(AgentConfig {
        transport: TransportSettings {
            uds: Some(UdsTransportConfig { socket_path: path }),
            quic: Some(QuicTransportConfig {
                bind_addr: "127.0.0.1:0".to_string(),
                cert_path: None,
                key_path: None,
            }),
        },
        subscriptions: vec![SubscriptionConfig {
            topic: "/odom".to_string(),
            msg_type: "nav_msgs/msg/Odometry".to_string(),
            qos: QosProfile::Default,
        }],
        control: None,
        poses: Default::default(),
    });

    let router = make_router();

    // Bind the QUIC endpoint separately so we know the port
    let quic_cfg = config.transport.quic.as_ref().unwrap();
    let endpoint = QuicTransport::bind(quic_cfg).await.unwrap();
    let quic_addr = endpoint.local_addr().unwrap();

    // Accept loop
    {
        let r = Arc::clone(&router);
        let jp = make_joint_publisher();
        let cfg = Arc::clone(&config);
        tokio::spawn(async move {
            while let Some(inc) = endpoint.accept().await {
                if let Ok(conn) = inc.await {
                    let r2 = Arc::clone(&r);
                    let j2 = Arc::clone(&jp);
                    let cfg2 = Arc::clone(&cfg);
                    let graph = make_graph_handle();
                    tokio::spawn(talos_agent::server::handle_quic_client(
                        conn, cfg2, r2, j2, graph,
                    ));
                }
            }
        });
    }

    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut client = QuicProtocolClient::connect(&quic_addr.to_string())
        .await
        .unwrap();
    client.subscribe(&["/odom".to_string()]).await.unwrap();

    // Give the server time to open the uni stream
    tokio::time::sleep(Duration::from_millis(50)).await;

    inject(&router, "/odom");

    let (topic, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_data())
        .await
        .expect("timed out waiting for QUIC data frame")
        .expect("recv_data error");

    assert_eq!(topic, "/odom");
}

// ── 8.3: Dual-mode agent serves UDS and QUIC simultaneously ─────────────────

#[cfg(feature = "quic")]
#[tokio::test]
async fn dual_mode_agent_serves_uds_and_quic() {
    use talos_common::config::QuicTransportConfig;
    use talos_common::session::QuicProtocolClient;
    use talos_common::transport::quic::QuicTransport;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("s8_3.sock").to_string_lossy().into_owned();

    let config = Arc::new(AgentConfig {
        transport: TransportSettings {
            uds: Some(UdsTransportConfig {
                socket_path: path.clone(),
            }),
            quic: Some(QuicTransportConfig {
                bind_addr: "127.0.0.1:0".to_string(),
                cert_path: None,
                key_path: None,
            }),
        },
        subscriptions: vec![SubscriptionConfig {
            topic: "/odom".to_string(),
            msg_type: "nav_msgs/msg/Odometry".to_string(),
            qos: QosProfile::Default,
        }],
        control: None,
        poses: Default::default(),
    });

    let router = make_router();

    // UDS server
    {
        let r = Arc::clone(&router);
        let jp = make_joint_publisher();
        let cfg = Arc::clone(&config);
        let graph = make_graph_handle();
        tokio::spawn(async move {
            let _ = talos_agent::server::run(cfg, r, jp, graph).await;
        });
    }

    // QUIC server
    let quic_addr = {
        let quic_cfg = config.transport.quic.as_ref().unwrap();
        let endpoint = QuicTransport::bind(quic_cfg).await.unwrap();
        let addr = endpoint.local_addr().unwrap();
        let r = Arc::clone(&router);
        let jp = make_joint_publisher();
        let cfg = Arc::clone(&config);
        tokio::spawn(async move {
            while let Some(inc) = endpoint.accept().await {
                if let Ok(conn) = inc.await {
                    let r2 = Arc::clone(&r);
                    let j2 = Arc::clone(&jp);
                    let cfg2 = Arc::clone(&cfg);
                    let graph = make_graph_handle();
                    tokio::spawn(talos_agent::server::handle_quic_client(
                        conn, cfg2, r2, j2, graph,
                    ));
                }
            }
        });
        addr
    };

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect both clients
    let mut uds = UdsProtocolClient::connect(&path).await.unwrap();
    uds.subscribe(&["/odom".to_string()]).await.unwrap();

    let mut quic = QuicProtocolClient::connect(&quic_addr.to_string())
        .await
        .unwrap();
    quic.subscribe(&["/odom".to_string()]).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    inject(&router, "/odom");

    let timeout = Duration::from_secs(2);
    let (t_uds, _) = tokio::time::timeout(timeout, uds.recv_data())
        .await
        .unwrap()
        .unwrap();
    let (t_quic, _) = tokio::time::timeout(timeout, quic.recv_data())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(t_uds, "/odom");
    assert_eq!(t_quic, "/odom");
}

// ── 8.4: Unsubscribe closes data delivery ────────────────────────────────────

/// After unsubscribing, the client should not receive further frames for
/// that topic (verified with a short timeout).
#[tokio::test]
async fn uds_unsubscribe_stops_data_delivery() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("s8_4.sock").to_string_lossy().into_owned();

    let config = test_config_uds(&path);
    let router = spawn_uds_server(config).await;

    let mut client = UdsProtocolClient::connect(&path).await.unwrap();
    client.subscribe(&["/odom".to_string()]).await.unwrap();

    // First frame should arrive
    inject(&router, "/odom");
    let (topic, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_data())
        .await
        .expect("timed out on first frame")
        .unwrap();
    assert_eq!(topic, "/odom");

    // Unsubscribe
    client.unsubscribe(&["/odom".to_string()]).await.unwrap();

    // Inject another frame — should not be delivered
    inject(&router, "/odom");
    let result = tokio::time::timeout(Duration::from_millis(200), client.recv_data()).await;
    assert!(
        result.is_err(),
        "recv_data should have timed out after unsubscribe, but got a frame"
    );
}
