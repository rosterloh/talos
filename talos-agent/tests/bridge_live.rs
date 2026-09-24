//! End-to-end check that the bridge actually delivers ROS 2 messages.
//!
//! Requires the ROS 2 Lyrical environment:
//!   pixi run -- cargo test -p talos-agent --test bridge_live
//!
//! The rest of the agent's tests drive the IPC protocol with synthetic
//! `Response` values, so they stay green even when no ROS 2 message ever
//! reaches the router. This test publishes on a real topic and asserts the
//! bridge forwards it, which is the only way to catch the two failure modes
//! below.
//!
//! Regression coverage:
//!
//! 1. Dropped subscription handles. Under rclrs 0.8 a `Subscription` owns its
//!    place in the executor wait set, so discarding the handle returned by
//!    `create_subscription` tears the subscription down immediately — the
//!    agent logs `subscribed` and the ROS graph reports zero subscribers.
//!
//! 2. Executor starvation. `executor.spin()` blocks its thread for the process
//!    lifetime and runs every ROS 2 callback there. Called directly inside a
//!    `tokio::spawn`ed task, the callback's `tx.send` wakes the forwarder onto
//!    that worker's non-stealable LIFO slot, which is never polled again — the
//!    send reports success and the data is never seen.
//!
//! The runtime topology below deliberately mirrors `main.rs`: the bridge task
//! and the bridge→router forwarder share one multi-threaded runtime, because
//! that co-location is precisely what failure mode 2 depends on. Receiving in
//! a separate runtime would make the test pass with the bug present.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rclrs::CreateBasicExecutor;
use ros_env::std_msgs;
use talos_common::config::{
    AgentConfig, QosProfile, SubscriptionConfig, TransportSettings, UdsTransportConfig,
};
use talos_common::protocol::messages::Response;
use talos_common::protocol::types::DynValue;
use tokio::sync::Mutex;

use talos_agent::{GraphHandle, JointPublisher};

const TOPIC: &str = "/talos_bridge_live_test";
const PAYLOAD: &str = "live";
/// Generous enough to absorb DDS discovery on a loaded CI runner.
const DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);

fn test_config() -> Arc<AgentConfig> {
    Arc::new(AgentConfig {
        transport: TransportSettings {
            uds: Some(UdsTransportConfig {
                socket_path: "/tmp/talos-bridge-live-test.sock".to_string(),
            }),
            quic: None,
        },
        subscriptions: vec![SubscriptionConfig {
            topic: TOPIC.to_string(),
            msg_type: "std_msgs/msg/String".to_string(),
            qos: QosProfile::Default,
        }],
        control: None,
        poses: HashMap::new(),
    })
}

/// Publish until `stop` is set. Repeating covers the window before DDS
/// discovery matches the two endpoints; a single publish could legitimately be
/// lost.
fn publish_until_stopped(stop: Arc<AtomicBool>) {
    let context = rclrs::Context::default_from_env().expect("publisher context");
    // Never spun: publishing does not require the executor to run.
    let executor = context.create_basic_executor();
    let node = executor
        .create_node("talos_bridge_live_test_publisher")
        .expect("publisher node");
    let publisher = node
        .create_publisher::<std_msgs::msg::String>(TOPIC)
        .expect("publisher");

    while !stop.load(Ordering::Relaxed) {
        let message = std_msgs::msg::String {
            data: PAYLOAD.to_string(),
        };
        if publisher.publish(message).is_err() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `executor.spin()` only returns on context shutdown, which the bridge owns
/// internally, so the bridge runtime is parked on a detached thread and torn
/// down by process exit rather than joined.
fn spawn_bridge(out_tx: std::sync::mpsc::Sender<Response>) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("bridge runtime");

        runtime.block_on(async move {
            let (bridge_tx, mut bridge_rx) = tokio::sync::mpsc::unbounded_channel();

            // Mirrors the bridge→router forwarder in main.rs.
            tokio::spawn(async move {
                while let Some(response) = bridge_rx.recv().await {
                    if out_tx.send(response).is_err() {
                        return;
                    }
                }
            });

            let joint_publisher: JointPublisher = Arc::new(Mutex::new(None));
            let graph_handle: GraphHandle = Arc::new(Mutex::new(None));
            tokio::spawn(talos_agent::bridge::run(
                test_config(),
                bridge_tx,
                joint_publisher,
                graph_handle,
            ));

            std::future::pending::<()>().await
        })
    });
}

#[test]
fn bridge_forwards_live_topic_data() {
    let (out_tx, out_rx) = std::sync::mpsc::channel();
    spawn_bridge(out_tx);

    let stop = Arc::new(AtomicBool::new(false));
    let publisher = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || publish_until_stopped(stop))
    };

    let deadline = std::time::Instant::now() + DELIVERY_TIMEOUT;
    let data = loop {
        let remaining = deadline
            .checked_duration_since(std::time::Instant::now())
            .expect("bridge delivered no data for a live topic within the timeout");

        match out_rx.recv_timeout(remaining) {
            Ok(Response::TopicData { topic, data, .. }) if topic == TOPIC => break data,
            Ok(_) => continue,
            Err(e) => panic!("bridge delivered no data for a live topic: {e}"),
        }
    };

    stop.store(true, Ordering::Relaxed);
    let _ = publisher.join();

    assert_eq!(data, DynValue::String(PAYLOAD.to_string()));
}
