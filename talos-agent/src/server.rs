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

pub async fn run(
    config: Arc<AgentConfig>,
    broadcast_tx: broadcast::Sender<Response>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let transport_config = TransportConfig {
        socket_path: config.transport.socket_path.clone(),
    };

    let listener = UdsTransport::bind(&transport_config).await?;
    info!(path = %transport_config.socket_path, "listening for clients");

    loop {
        let conn = UdsTransport::accept(&listener).await?;
        let config = Arc::clone(&config);
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
                                let response = handle_request(&request, &config);
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

fn handle_request(request: &Request, config: &AgentConfig) -> Response {
    match request {
        Request::ListTopics => {
            // TODO: query rclrs graph for full topic list
            // For now, return configured subscriptions
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
            // TODO: query rclrs graph for node list
            Response::NodeList(vec![])
        }
        Request::SetJointPosition { joint, position } => {
            if config.control.is_none() {
                return Response::Error("control not configured".into());
            }
            // TODO: publish joint command via rclrs publisher
            info!(joint = %joint, position = %position, "joint position command received");
            Response::Error("joint control not yet implemented".into())
        }
        Request::ExecutePose { name } => {
            if config.control.is_none() {
                return Response::Error("control not configured".into());
            }
            match config.poses.get(name) {
                Some(positions) => {
                    // TODO: publish full JointState via rclrs publisher
                    let pose_info = PoseInfo {
                        name: name.clone(),
                        positions: positions.iter().map(|(k, v)| (k.clone(), *v)).collect(),
                    };
                    info!(pose = %name, joints = pose_info.positions.len(), "executing pose");
                    Response::Error("pose execution not yet implemented".into())
                }
                None => Response::Error(format!("unknown pose: {name}")),
            }
        }
    }
}
