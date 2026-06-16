use std::sync::Arc;

use rclrs::{CreateBasicExecutor, IntoPrimitiveOptions};
use talos_common::config::{AgentConfig, QosProfile};
use talos_common::protocol::messages::Response;
use tokio::sync::mpsc;
use tracing::info;

use crate::conversions::message_type_entry;
use crate::{GraphHandle, JointPublisher};

fn subscription_options<'a>(topic: &'a str, qos: &QosProfile) -> rclrs::PrimitiveOptions<'a> {
    match qos {
        QosProfile::Default => topic.into_primitive_options(),
        QosProfile::SensorData => topic.sensor_data_qos(),
    }
}

/// Sender half for ROS 2 callbacks to forward topic data to the router task
/// without blocking on the router lock.
pub type BridgeSender = mpsc::UnboundedSender<Response>;

pub async fn run(
    config: Arc<AgentConfig>,
    bridge_tx: BridgeSender,
    joint_publisher: JointPublisher,
    graph_handle: GraphHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let context = rclrs::Context::default_from_env()?;
    let mut executor = context.create_basic_executor();
    let node = executor.create_node("talos_agent")?;

    for sub_config in &config.subscriptions {
        let topic = sub_config.topic.clone();
        let type_name = sub_config.msg_type.clone();
        let opts = subscription_options(&topic, &sub_config.qos);
        let tx = bridge_tx.clone();

        if let Some(entry) = message_type_entry(&type_name) {
            entry.subscribe(&node, opts, topic.clone(), type_name.clone(), tx)?;
            info!(topic = %topic, msg_type = %type_name, "subscribed");
        } else {
            // No compiled-in converter: fall back to a runtime dynamic-message
            // subscription that resolves the type via introspection typesupport.
            crate::dynamic::log_dynamic_fallback(&topic, &type_name);
            crate::dynamic::subscribe_dynamic(&node, opts, topic.clone(), type_name.clone(), tx)?;
            info!(topic = %topic, msg_type = %type_name, "subscribed (dynamic)");
        }
    }

    if let Some(control) = &config.control {
        let publisher = node.create_publisher::<sensor_msgs::msg::JointState>(&control.topic)?;
        info!(topic = %control.topic, "joint command publisher created");
        *joint_publisher.lock().await = Some(publisher);
    }

    *graph_handle.lock().await = Some(Arc::clone(&node));

    info!("rclrs node spinning");
    executor.spin(rclrs::SpinOptions::default());

    *graph_handle.lock().await = None;

    Ok(())
}
