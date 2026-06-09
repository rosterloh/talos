use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use talos_common::config::AgentConfig;
use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::{StreamHeader, TopicFrame, TopicSub};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info, warn};

use super::RouterHandle;
use super::requests::handle_control_request;
use crate::{GraphHandle, JointPublisher};

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

pub async fn handle_quic_client(
    connection: quinn::Connection,
    config: Arc<AgentConfig>,
    router: RouterHandle,
    joint_publisher: JointPublisher,
    graph_handle: GraphHandle,
) {
    let (client_id, mut data_rx) = router.lock().await.register();

    // Client opens the first bidirectional stream for control messages.
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
