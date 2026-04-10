use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::TopicInfo;
use talos_common::session::uds::UdsProtocolClient;
use talos_common::session::ProtocolClient;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::state::{AppState, TransportType};

/// Which transport to use for this session.
pub enum ClientConfig {
    Uds { socket_path: String },
    #[cfg(feature = "quic")]
    Quic { addr: String },
}

/// Connect (and reconnect on error) using the given transport configuration.
pub async fn run(
    config: ClientConfig,
    state: Arc<Mutex<AppState>>,
    mut cmd_rx: mpsc::UnboundedReceiver<Request>,
) {
    loop {
        let result = match &config {
            ClientConfig::Uds { socket_path } => {
                match UdsProtocolClient::connect(socket_path).await {
                    Ok(client) => {
                        {
                            let mut s = state.lock().unwrap();
                            s.connected = true;
                            s.transport_type = Some(TransportType::Uds);
                        }
                        info!(path = %socket_path, "connected to agent via UDS");
                        connect_and_run(client, &state, &mut cmd_rx).await
                    }
                    Err(e) => Err(format!("UDS connect failed: {e}")),
                }
            }
            #[cfg(feature = "quic")]
            ClientConfig::Quic { addr } => {
                match talos_common::session::QuicProtocolClient::connect(addr).await {
                    Ok(client) => {
                        {
                            let mut s = state.lock().unwrap();
                            s.connected = true;
                            s.transport_type = Some(TransportType::Quic);
                        }
                        info!(addr = %addr, "connected to agent via QUIC");
                        connect_and_run(client, &state, &mut cmd_rx).await
                    }
                    Err(e) => Err(format!("QUIC connect failed: {e}")),
                }
            }
        };

        {
            let mut s = state.lock().unwrap();
            s.connected = false;
            s.transport_type = None;
        }

        match result {
            Ok(()) => info!("connection closed"),
            Err(e) => warn!("connection error: {e}"),
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
        info!("reconnecting...");
    }
}

/// Run the protocol session for one connection lifetime.
///
/// 1. Sends `ListTopics` and `ListNodes` to populate the UI.
/// 2. Subscribes to all discovered topics.
/// 3. Enters a select loop that concurrently handles incoming data frames
///    and outgoing commands from the UI.
///
/// On reconnect, this function is called again with a fresh client.
async fn connect_and_run<C: ProtocolClient>(
    mut client: C,
    state: &Arc<Mutex<AppState>>,
    cmd_rx: &mut mpsc::UnboundedReceiver<Request>,
) -> Result<(), String> {
    // ── discover topics ───────────────────────────────────────────────────────
    let list_resp = client
        .request(Request::ListTopics)
        .await
        .map_err(|e| e.to_string())?;

    // Collect type info so we can reconstruct it in TopicData frames later
    let mut type_map: HashMap<String, String> = HashMap::new();
    let topic_names: Vec<String>;

    if let Response::TopicList(ref topics) = list_resp {
        for t in topics {
            type_map.insert(t.name.clone(), t.type_name.clone());
        }
        topic_names = topics.iter().map(|t| t.name.clone()).collect();
    } else {
        topic_names = Vec::new();
    }

    {
        let mut s = state.lock().unwrap();
        s.handle_response(list_resp);
    }

    // ── list nodes ────────────────────────────────────────────────────────────
    if let Ok(resp) = client.request(Request::ListNodes).await {
        let mut s = state.lock().unwrap();
        s.handle_response(resp);
    }

    // ── subscribe to all discovered topics ────────────────────────────────────
    if !topic_names.is_empty() {
        match client.subscribe(&topic_names).await {
            Ok(subs) => {
                // Update type_map with confirmed subscriptions (may include type info)
                for s in &subs {
                    type_map.insert(s.topic.clone(), s.type_name.clone());
                }
                info!("subscribed to {} topics", subs.len());
            }
            Err(e) => warn!("initial subscribe failed: {e}"),
        }
    }

    // ── main loop ─────────────────────────────────────────────────────────────
    loop {
        tokio::select! {
            data_result = client.recv_data() => {
                match data_result {
                    Ok((topic, frame)) => {
                        let type_name = type_map
                            .get(&topic)
                            .cloned()
                            .unwrap_or_default();
                        let response = Response::TopicData {
                            topic,
                            type_name,
                            stamp: frame.stamp,
                            data: frame.data,
                        };
                        let mut s = state.lock().unwrap();
                        s.handle_response(response);
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(request) => {
                        match client.request(request).await {
                            Ok(response) => {
                                let mut s = state.lock().unwrap();
                                s.handle_response(response);
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                    None => return Ok(()),
                }
            }
        }
    }
}

/// Extract TopicInfo list from a `TopicList` response.
#[allow(dead_code)]
fn extract_topics(resp: &Response) -> Vec<TopicInfo> {
    if let Response::TopicList(topics) = resp {
        topics.clone()
    } else {
        vec![]
    }
}
