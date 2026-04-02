use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use talos_common::config::AgentConfig;
use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::PoseInfo;
use talos_common::transport::uds::UdsTransport;
use talos_common::transport::{TransportServer, TransportConfig};
use tokio::sync::broadcast;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info, warn};

use crate::JointPublisher;

pub async fn run(
    config: Arc<AgentConfig>,
    broadcast_tx: broadcast::Sender<Response>,
    joint_publisher: JointPublisher,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let transport_config = TransportConfig {
        socket_path: config.transport.socket_path.clone(),
    };

    let listener = UdsTransport::bind(&transport_config).await?;
    info!(path = %transport_config.socket_path, "listening for clients");

    loop {
        let conn = UdsTransport::accept(&listener).await?;
        let config = Arc::clone(&config);
        let joint_pub = Arc::clone(&joint_publisher);
        let mut broadcast_rx = broadcast_tx.subscribe();

        info!("client connected");

        tokio::spawn(async move {
            let mut reader = FramedRead::new(conn.reader, BincodeCodec::<Request>::new());
            let mut writer = FramedWrite::new(conn.writer, BincodeCodec::<Response>::new());

            loop {
                tokio::select! {
                    req = reader.next() => {
                        match req {
                            Some(Ok(request)) => {
                                let response = handle_request(&request, &config, &joint_pub).await;
                                if let Err(e) = writer.send(response).await {
                                    error!("failed to send response: {e}");
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                error!("failed to read request: {e}");
                                break;
                            }
                            None => {
                                info!("client disconnected");
                                break;
                            }
                        }
                    }
                    msg = broadcast_rx.recv() => {
                        match msg {
                            Ok(response) => {
                                if let Err(e) = writer.send(response).await {
                                    error!("failed to broadcast to client: {e}");
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!("client lagged, dropped {n} messages");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}

async fn handle_request(
    request: &Request,
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
) -> Response {
    match request {
        Request::ListTopics => {
            let topics = config
                .subscriptions
                .iter()
                .map(|s| talos_common::protocol::types::TopicInfo {
                    name: s.topic.clone(),
                    type_name: s.msg_type.clone(),
                    publisher_count: 0,
                    subscriber_count: 0,
                })
                .collect();
            Response::TopicList(topics)
        }
        Request::ListNodes => {
            Response::NodeList(vec![])
        }
        Request::SetJointPosition { joint, position } => {
            if config.control.is_none() {
                return Response::Error("control not configured".into());
            }
            let guard = joint_publisher.lock().await;
            match guard.as_ref() {
                Some(publisher) => {
                    let mut msg = sensor_msgs::msg::JointState::default();
                    msg.name = vec![joint.clone()];
                    msg.position = vec![*position];
                    match publisher.publish(msg) {
                        Ok(()) => {
                            info!(joint = %joint, position = %position, "published joint command");
                            Response::Error("ok".into())
                        }
                        Err(e) => {
                            error!("failed to publish joint command: {e}");
                            Response::Error(format!("publish failed: {e}"))
                        }
                    }
                }
                None => Response::Error("joint publisher not ready".into()),
            }
        }
        Request::ExecutePose { name } => {
            if config.control.is_none() {
                return Response::Error("control not configured".into());
            }
            match config.poses.get(name) {
                Some(positions) => {
                    let guard = joint_publisher.lock().await;
                    match guard.as_ref() {
                        Some(publisher) => {
                            let mut msg = sensor_msgs::msg::JointState::default();
                            let (names, pos): (Vec<_>, Vec<_>) =
                                positions.iter().map(|(k, v)| (k.clone(), *v)).unzip();
                            msg.name = names;
                            msg.position = pos;
                            match publisher.publish(msg) {
                                Ok(()) => {
                                    info!(
                                        pose = %name,
                                        joints = positions.len(),
                                        "published pose command"
                                    );
                                    let pose_info = PoseInfo {
                                        name: name.clone(),
                                        positions: positions
                                            .iter()
                                            .map(|(k, v)| (k.clone(), *v))
                                            .collect(),
                                    };
                                    Response::PoseList(vec![pose_info])
                                }
                                Err(e) => {
                                    error!("failed to publish pose: {e}");
                                    Response::Error(format!("publish failed: {e}"))
                                }
                            }
                        }
                        None => Response::Error("joint publisher not ready".into()),
                    }
                }
                None => Response::Error(format!("unknown pose: {name}")),
            }
        }
    }
}
