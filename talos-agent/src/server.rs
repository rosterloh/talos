use std::sync::Arc;

#[cfg(feature = "quic")]
use bytes::{BufMut, BytesMut};
use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "quic")]
use serde::Serialize;
use talos_common::config::AgentConfig;
use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::{NodeInfo, PoseInfo, TopicInfo, TopicSub};
#[cfg(feature = "quic")]
use talos_common::protocol::types::{StreamHeader, TopicFrame};
use talos_common::transport::uds::UdsTransport;
use talos_common::transport::{TransportConfig, TransportServer};
use tokio::sync::Mutex as TokioMutex;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info, warn};

use crate::router::{ClientId, TopicRouter};
use crate::{GraphHandle, JointPublisher};

pub type RouterHandle = Arc<TokioMutex<TopicRouter>>;

// ── UDS listener ─────────────────────────────────────────────────────────────

/// Accept UDS connections and spawn a handler task for each client.
/// This is the primary entry point for the UDS server loop.
pub async fn run(
    config: Arc<AgentConfig>,
    router: RouterHandle,
    joint_publisher: JointPublisher,
    graph_handle: GraphHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket_path = config
        .transport
        .uds
        .as_ref()
        .map(|u| u.socket_path.clone())
        .unwrap_or_else(|| "/tmp/talos.sock".to_string());

    let transport_config = TransportConfig {
        socket_path: socket_path.clone(),
    };

    let listener = UdsTransport::bind(&transport_config).await?;
    info!(path = %socket_path, "listening for UDS clients");

    loop {
        let conn = UdsTransport::accept(&listener).await?;
        let config = Arc::clone(&config);
        let router = Arc::clone(&router);
        let joint_pub = Arc::clone(&joint_publisher);
        let graph = Arc::clone(&graph_handle);

        info!("UDS client connected");
        tokio::spawn(async move {
            handle_uds_connection(conn, config, router, joint_pub, graph).await;
        });
    }
}

async fn handle_uds_connection(
    conn: talos_common::transport::Connection<UdsTransport>,
    config: Arc<AgentConfig>,
    router: RouterHandle,
    joint_publisher: JointPublisher,
    graph_handle: GraphHandle,
) {
    let (client_id, mut data_rx) = router.lock().await.register();

    let mut reader = FramedRead::new(conn.reader, BincodeCodec::<Request>::new());
    let mut writer = FramedWrite::new(conn.writer, BincodeCodec::<Response>::new());

    loop {
        tokio::select! {
            req = reader.next() => {
                match req {
                    Some(Ok(request)) => {
                        if let Some(response) =
                            handle_request(&request, &config, &joint_publisher, &graph_handle, &router, client_id).await
                        {
                            if let Err(e) = writer.send(response).await {
                                error!("failed to send response: {e}");
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("failed to read request: {e}");
                        break;
                    }
                    None => {
                        info!("UDS client disconnected");
                        break;
                    }
                }
            }
            data = data_rx.recv() => {
                match data {
                    Some(response) => {
                        if let Err(e) = writer.send(response).await {
                            error!("failed to push topic data: {e}");
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    router.lock().await.deregister(client_id);
    info!("UDS client session ended");
}

// ── QUIC listener ─────────────────────────────────────────────────────────────

#[cfg(feature = "quic")]
pub async fn run_quic(
    config: Arc<AgentConfig>,
    router: RouterHandle,
    joint_publisher: JointPublisher,
    graph_handle: GraphHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use talos_common::transport::quic::QuicTransport;

    let quic_cfg = config
        .transport
        .quic
        .as_ref()
        .ok_or("QUIC transport not configured")?;

    let endpoint = QuicTransport::bind(quic_cfg).await?;
    info!(addr = %quic_cfg.bind_addr, "listening for QUIC clients");

    while let Some(incoming) = endpoint.accept().await {
        let connection = match incoming.await {
            Ok(c) => c,
            Err(e) => {
                warn!("QUIC incoming connection failed: {e}");
                continue;
            }
        };

        let config = Arc::clone(&config);
        let router = Arc::clone(&router);
        let joint_pub = Arc::clone(&joint_publisher);
        let graph = Arc::clone(&graph_handle);

        info!("QUIC client connected from {}", connection.remote_address());
        tokio::spawn(async move {
            handle_quic_client(connection, config, router, joint_pub, graph).await;
        });
    }

    Ok(())
}

#[cfg(feature = "quic")]
pub async fn handle_quic_client(
    connection: quinn::Connection,
    config: Arc<AgentConfig>,
    router: RouterHandle,
    joint_publisher: JointPublisher,
    graph_handle: GraphHandle,
) {
    let (client_id, mut data_rx) = router.lock().await.register();

    // Client opens the first bidirectional stream for control messages
    let (send, recv) = match connection.accept_bi().await {
        Ok(s) => s,
        Err(e) => {
            error!("QUIC: failed to accept control stream: {e}");
            router.lock().await.deregister(client_id);
            return;
        }
    };

    let mut control_tx = FramedWrite::new(send, BincodeCodec::<Response>::new());
    let mut control_rx = FramedRead::new(recv, BincodeCodec::<Request>::new());

    // Server-initiated unidirectional streams, one per subscribed topic
    let mut topic_streams: std::collections::HashMap<String, quinn::SendStream> =
        std::collections::HashMap::new();

    loop {
        tokio::select! {
            req = control_rx.next() => {
                match req {
                    Some(Ok(Request::Subscribe { topics })) => {
                        let topic_subs: Vec<TopicSub> = topics.iter()
                            .filter_map(|t| {
                                config.subscriptions.iter().find(|s| &s.topic == t)
                                    .map(|s| TopicSub {
                                        topic: s.topic.clone(),
                                        type_name: s.msg_type.clone(),
                                    })
                            })
                            .collect();

                        router.lock().await.subscribe(client_id, topics.iter().cloned());

                        // Open server-initiated uni streams for new subscriptions
                        for ts in &topic_subs {
                            if !topic_streams.contains_key(&ts.topic) {
                                match connection.open_uni().await {
                                    Ok(mut send) => {
                                        let header = StreamHeader {
                                            topic: ts.topic.clone(),
                                            type_name: ts.type_name.clone(),
                                        };
                                        if write_quic_frame(&mut send, &header).await.is_ok() {
                                            topic_streams.insert(ts.topic.clone(), send);
                                        } else {
                                            warn!(topic = %ts.topic, "failed to write stream header");
                                        }
                                    }
                                    Err(e) => error!("QUIC open_uni failed: {e}"),
                                }
                            }
                        }

                        let _ = control_tx.send(Response::Subscribed { topics: topic_subs }).await;
                    }
                    Some(Ok(Request::Unsubscribe { topics })) => {
                        router.lock().await.unsubscribe(client_id, &topics);
                        for topic in &topics {
                            if let Some(mut send) = topic_streams.remove(topic) {
                                let _ = send.finish();
                            }
                        }
                        let _ = control_tx.send(Response::Unsubscribed { topics }).await;
                    }
                    Some(Ok(other)) => {
                        let response =
                            handle_control_request(&other, &config, &joint_publisher, &graph_handle).await;
                        let _ = control_tx.send(response).await;
                    }
                    Some(Err(e)) => {
                        error!("QUIC control stream error: {e}");
                        break;
                    }
                    None => {
                        info!("QUIC client disconnected");
                        break;
                    }
                }
            }
            data = data_rx.recv() => {
                if let Some(Response::TopicData { topic, stamp, data, .. }) = data {
                    if let Some(send) = topic_streams.get_mut(&topic) {
                        let frame = TopicFrame { stamp, data };
                        if write_quic_frame(send, &frame).await.is_err() {
                            topic_streams.remove(&topic);
                        }
                    }
                }
            }
        }
    }

    for (_, mut send) in topic_streams {
        let _ = send.finish();
    }
    router.lock().await.deregister(client_id);
    info!("QUIC client session ended");
}

/// Write a single length-prefixed bincode frame to a QUIC SendStream.
#[cfg(feature = "quic")]
async fn write_quic_frame<T: Serialize>(
    send: &mut quinn::SendStream,
    value: &T,
) -> Result<(), String> {
    let payload = bincode::serialize(value).map_err(|e| e.to_string())?;
    let len: u32 = u32::try_from(payload.len())
        .map_err(|_| format!("frame too large: {} bytes exceeds u32::MAX", payload.len()))?;
    let mut buf = BytesMut::with_capacity(4 + payload.len());
    buf.put_u32(len);
    buf.put_slice(&payload);
    send.write_all(&buf).await.map_err(|e| e.to_string())
}

// ── Request dispatching ───────────────────────────────────────────────────────

async fn handle_request(
    request: &Request,
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
    graph_handle: &GraphHandle,
    router: &RouterHandle,
    client_id: ClientId,
) -> Option<Response> {
    match request {
        Request::Subscribe { topics } => {
            let topic_subs: Vec<TopicSub> = topics
                .iter()
                .filter_map(|t| {
                    config
                        .subscriptions
                        .iter()
                        .find(|s| &s.topic == t)
                        .map(|s| TopicSub {
                            topic: s.topic.clone(),
                            type_name: s.msg_type.clone(),
                        })
                })
                .collect();
            router
                .lock()
                .await
                .subscribe(client_id, topics.iter().cloned());
            Some(Response::Subscribed { topics: topic_subs })
        }
        Request::Unsubscribe { topics } => {
            router.lock().await.unsubscribe(client_id, topics);
            Some(Response::Unsubscribed {
                topics: topics.clone(),
            })
        }
        other => Some(handle_control_request(other, config, joint_publisher, graph_handle).await),
    }
}

async fn handle_control_request(
    request: &Request,
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
    graph_handle: &GraphHandle,
) -> Response {
    match request {
        Request::ListTopics => list_topics(config, graph_handle).await,
        Request::ListNodes => list_nodes(graph_handle).await,
        Request::ListPoses => Response::PoseList(configured_poses(config)),
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
                            Response::Ok("joint command published".into())
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
                                    info!(pose = %name, joints = positions.len(), "published pose");
                                    Response::Ok(format!("pose '{name}' published"))
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
        _ => Response::Error("unexpected request".into()),
    }
}

async fn list_topics(config: &AgentConfig, graph_handle: &GraphHandle) -> Response {
    let graph_node = graph_handle.lock().await.clone();

    if let Some(node) = graph_node {
        match node.get_topic_names_and_types() {
            Ok(names_and_types) => {
                let mut topics: Vec<TopicInfo> = names_and_types
                    .into_iter()
                    .map(|(name, types)| {
                        let publisher_count = node.count_publishers(&name).unwrap_or(0);
                        let subscriber_count = node.count_subscriptions(&name).unwrap_or(0);
                        TopicInfo {
                            name,
                            type_name: format_type_names(types),
                            publisher_count,
                            subscriber_count,
                        }
                    })
                    .collect();
                topics.sort_by(|a, b| a.name.cmp(&b.name));
                return Response::TopicList(topics);
            }
            Err(e) => {
                warn!("failed to query ROS graph topics, using configured subscriptions: {e}");
            }
        }
    }

    Response::TopicList(configured_topics(config))
}

async fn list_nodes(graph_handle: &GraphHandle) -> Response {
    let graph_node = graph_handle.lock().await.clone();
    let Some(node) = graph_node else {
        return Response::NodeList(vec![]);
    };

    let names = match node.get_node_names() {
        Ok(names) => names,
        Err(e) => return Response::Error(format!("failed to query ROS graph nodes: {e}")),
    };

    let mut nodes: Vec<NodeInfo> = names
        .into_iter()
        .map(|n| {
            let publishers = graph_names_for_node(
                node.get_publisher_names_and_types_by_node(&n.name, &n.namespace),
            );
            let subscribers = graph_names_for_node(
                node.get_subscription_names_and_types_by_node(&n.name, &n.namespace),
            );
            let services = graph_names_for_node(
                node.get_service_names_and_types_by_node(&n.name, &n.namespace),
            );

            NodeInfo {
                name: n.name,
                namespace: n.namespace,
                publishers,
                subscribers,
                services,
            }
        })
        .collect();

    nodes.sort_by(|a, b| {
        a.namespace
            .cmp(&b.namespace)
            .then_with(|| a.name.cmp(&b.name))
    });
    Response::NodeList(nodes)
}

fn configured_topics(config: &AgentConfig) -> Vec<TopicInfo> {
    let mut topics: Vec<TopicInfo> = config
        .subscriptions
        .iter()
        .map(|s| TopicInfo {
            name: s.topic.clone(),
            type_name: s.msg_type.clone(),
            publisher_count: 0,
            subscriber_count: 0,
        })
        .collect();
    topics.sort_by(|a, b| a.name.cmp(&b.name));
    topics
}

fn configured_poses(config: &AgentConfig) -> Vec<PoseInfo> {
    let mut poses: Vec<PoseInfo> = config
        .poses
        .iter()
        .map(|(name, positions)| {
            let mut positions: Vec<(String, f64)> =
                positions.iter().map(|(k, v)| (k.clone(), *v)).collect();
            positions.sort_by(|a, b| a.0.cmp(&b.0));
            PoseInfo {
                name: name.clone(),
                positions,
            }
        })
        .collect();
    poses.sort_by(|a, b| a.name.cmp(&b.name));
    poses
}

fn graph_names_for_node(
    result: Result<std::collections::HashMap<String, Vec<String>>, rclrs::RclrsError>,
) -> Vec<String> {
    match result {
        Ok(names_and_types) => {
            let mut names: Vec<String> = names_and_types.into_keys().collect();
            names.sort();
            names
        }
        Err(e) => {
            warn!("failed to query ROS graph node endpoints: {e}");
            vec![]
        }
    }
}

fn format_type_names(types: Vec<String>) -> String {
    if types.is_empty() {
        String::new()
    } else {
        types.join(", ")
    }
}
