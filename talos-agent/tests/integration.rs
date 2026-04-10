//! Integration tests for Talos agent subscription routing.
//!
//! Run UDS-only tests:
//!   cargo test -p talos-agent --test integration
//!
//! Run with QUIC:
//!   cargo test -p talos-agent --test integration --features quic

use std::sync::Arc;
use std::time::Duration;

use talos_common::config::{AgentConfig, SubscriptionConfig, TransportSettings, UdsTransportConfig};
use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{DynValue, Timestamp};
use talos_common::session::uds::UdsProtocolClient;
use talos_common::session::ProtocolClient;
use tempfile::TempDir;
use tokio::sync::Mutex;

use talos_agent::router::TopicRouter;
use talos_agent::server::RouterHandle;
use talos_agent::JointPublisher;

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
            },
            SubscriptionConfig {
                topic: "/joint_states".to_string(),
                msg_type: "sensor_msgs/msg/JointState".to_string(),
            },
        ],
        control: None,
        poses: Default::default(),
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

/// Spawn the UDS server in the background and return its router handle.
async fn spawn_uds_server(config: Arc<AgentConfig>) -> RouterHandle {
    let router = make_router();
    let jp = make_joint_publisher();
    let r = Arc::clone(&router);
    let jp2 = Arc::clone(&jp);
    tokio::spawn(async move {
        let _ = talos_agent::server::run(config, r, jp2).await;
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
                    tokio::spawn(
                        talos_agent::server::handle_quic_client(conn, cfg2, r2, j2),
                    );
                }
            }
        });
    }

    tokio::time::sleep(Duration::from_millis(80)).await;

    let mut client = QuicProtocolClient::connect(&quic_addr.to_string()).await.unwrap();
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
            uds: Some(UdsTransportConfig { socket_path: path.clone() }),
            quic: Some(QuicTransportConfig {
                bind_addr: "127.0.0.1:0".to_string(),
                cert_path: None,
                key_path: None,
            }),
        },
        subscriptions: vec![SubscriptionConfig {
            topic: "/odom".to_string(),
            msg_type: "nav_msgs/msg/Odometry".to_string(),
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
        tokio::spawn(async move { let _ = talos_agent::server::run(cfg, r, jp).await; });
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
                    tokio::spawn(talos_agent::server::handle_quic_client(conn, cfg2, r2, j2));
                }
            }
        });
        addr
    };

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect both clients
    let mut uds = UdsProtocolClient::connect(&path).await.unwrap();
    uds.subscribe(&["/odom".to_string()]).await.unwrap();

    let mut quic = QuicProtocolClient::connect(&quic_addr.to_string()).await.unwrap();
    quic.subscribe(&["/odom".to_string()]).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;
    inject(&router, "/odom");

    let timeout = Duration::from_secs(2);
    let (t_uds, _) = tokio::time::timeout(timeout, uds.recv_data()).await.unwrap().unwrap();
    let (t_quic, _) = tokio::time::timeout(timeout, quic.recv_data()).await.unwrap().unwrap();

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
    let result =
        tokio::time::timeout(Duration::from_millis(200), client.recv_data()).await;
    assert!(
        result.is_err(),
        "recv_data should have timed out after unsubscribe, but got a frame"
    );
}
