use std::sync::Arc;

use rclrs::{CreateBasicExecutor, IntoPrimitiveOptions};
use talos_common::config::{AgentConfig, QosProfile};
use talos_common::protocol::messages::Response;
use talos_common::protocol::types::Timestamp;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::conversions::*;
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

        match sub_config.msg_type.as_str() {
            "nav_msgs/msg/Odometry" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<nav_msgs::msg::Odometry, _>(
                    opts,
                    move |msg: nav_msgs::msg::Odometry| {
                        let stamp = timestamp_from_builtin(&msg.header.stamp);
                        let data = odometry_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "geometry_msgs/msg/Twist" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<geometry_msgs::msg::Twist, _>(
                    opts,
                    move |msg: geometry_msgs::msg::Twist| {
                        let stamp = Timestamp { sec: 0, nanosec: 0 };
                        let data = twist_msg_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "std_msgs/msg/String" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<std_msgs::msg::String, _>(
                    opts,
                    move |msg: std_msgs::msg::String| {
                        let stamp = Timestamp { sec: 0, nanosec: 0 };
                        let data = string_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "sensor_msgs/msg/JointState" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<sensor_msgs::msg::JointState, _>(
                    opts,
                    move |msg: sensor_msgs::msg::JointState| {
                        let stamp = timestamp_from_builtin(&msg.header.stamp);
                        let data = joint_state_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "sensor_msgs/msg/LaserScan" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<sensor_msgs::msg::LaserScan, _>(
                    opts,
                    move |msg: sensor_msgs::msg::LaserScan| {
                        let stamp = timestamp_from_builtin(&msg.header.stamp);
                        let data = laser_scan_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "sensor_msgs/msg/Imu" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<sensor_msgs::msg::Imu, _>(
                    opts,
                    move |msg: sensor_msgs::msg::Imu| {
                        let stamp = timestamp_from_builtin(&msg.header.stamp);
                        let data = imu_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "geometry_msgs/msg/PoseStamped" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<geometry_msgs::msg::PoseStamped, _>(
                    opts,
                    move |msg: geometry_msgs::msg::PoseStamped| {
                        let stamp = timestamp_from_builtin(&msg.header.stamp);
                        let data = pose_stamped_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            "rcl_interfaces/msg/Log" => {
                let topic_clone = topic.clone();
                let type_clone = type_name.clone();
                node.create_subscription::<rcl_interfaces::msg::Log, _>(
                    opts,
                    move |msg: rcl_interfaces::msg::Log| {
                        let stamp = timestamp_from_builtin(&msg.stamp);
                        let data = log_to_dynvalue(&msg);
                        let response = Response::TopicData {
                            topic: topic_clone.clone(),
                            type_name: type_clone.clone(),
                            stamp,
                            data,
                        };
                        let _ = tx.send(response);
                    },
                )?;
                info!(topic = %topic, msg_type = %type_name, "subscribed");
            }
            other => {
                warn!(topic = %topic, msg_type = %other, "unsupported message type, skipping");
            }
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
