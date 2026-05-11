use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::TopicInfo;
use talos_common::session::ProtocolClient;
use talos_common::session::uds::UdsProtocolClient;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::state::{AppState, TransportType};

/// Which transport to use for this session.
pub enum ClientConfig {
    Uds {
        socket_path: String,
    },
    #[cfg(feature = "quic")]
    Quic {
        addr: String,
    },
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
/// 2. Subscribes to the state-owned desired topic set.
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
    if let Response::TopicList(ref topics) = list_resp {
        for t in topics {
            type_map.insert(t.name.clone(), t.type_name.clone());
        }
    }

    let desired_topics = {
        let mut s = state.lock().unwrap();
        s.handle_response(list_resp);
        s.desired_topics_for_connection()
    };

    // ── list nodes ────────────────────────────────────────────────────────────
    if let Ok(resp) = client.request(Request::ListNodes).await {
        let mut s = state.lock().unwrap();
        s.handle_response(resp);
    }

    // ── list configured poses ────────────────────────────────────────────────
    if let Ok(resp) = client.request(Request::ListPoses).await {
        let mut s = state.lock().unwrap();
        s.handle_response(resp);
    }

    // ── subscribe to the desired topics for this session ─────────────────────
    if !desired_topics.is_empty() {
        {
            let mut s = state.lock().unwrap();
            s.mark_topics_pending_subscribe(&desired_topics);
        }

        match client.subscribe(&desired_topics).await {
            Ok(subs) => {
                // Update type_map with confirmed subscriptions (may include type info)
                for s in &subs {
                    type_map.insert(s.topic.clone(), s.type_name.clone());
                }
                {
                    let mut s = state.lock().unwrap();
                    s.handle_response(Response::Subscribed {
                        topics: subs.clone(),
                    });
                }
                info!("subscribed to {} topics", subs.len());
            }
            Err(e) => {
                let error = e.to_string();
                {
                    let mut s = state.lock().unwrap();
                    s.mark_subscription_error(&desired_topics, &error);
                }
                warn!("initial subscribe failed: {error}");
            }
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
                        match request {
                            Request::Subscribe { topics } => {
                                {
                                    let mut s = state.lock().unwrap();
                                    s.mark_topics_pending_subscribe(&topics);
                                }
                                match client.subscribe(&topics).await {
                                    Ok(subs) => {
                                        let mut s = state.lock().unwrap();
                                        s.handle_response(Response::Subscribed { topics: subs });
                                    }
                                    Err(e) => {
                                        let error = e.to_string();
                                        {
                                            let mut s = state.lock().unwrap();
                                            s.mark_subscription_error(&topics, &error);
                                        }
                                        return Err(error);
                                    }
                                }
                            }
                            Request::Unsubscribe { topics } => {
                                {
                                    let mut s = state.lock().unwrap();
                                    s.mark_topics_pending_unsubscribe(&topics);
                                }
                                match client.unsubscribe(&topics).await {
                                    Ok(unsubscribed) => {
                                        let mut s = state.lock().unwrap();
                                        s.handle_response(Response::Unsubscribed {
                                            topics: unsubscribed,
                                        });
                                    }
                                    Err(e) => {
                                        let error = e.to_string();
                                        {
                                            let mut s = state.lock().unwrap();
                                            s.mark_subscription_error(&topics, &error);
                                        }
                                        return Err(error);
                                    }
                                }
                            }
                            other => match client.request(other).await {
                                Ok(response) => {
                                let mut s = state.lock().unwrap();
                                s.handle_response(response);
                            }
                                Err(e) => return Err(e.to_string()),
                            },
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

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use talos_common::error::Error;
    use talos_common::protocol::types::TopicSub;

    use super::*;

    #[derive(Clone)]
    struct FakeClient {
        topics: Vec<TopicInfo>,
        subscribe_calls: Arc<Mutex<Vec<Vec<String>>>>,
        unsubscribe_calls: Arc<Mutex<Vec<Vec<String>>>>,
        request_calls: Arc<Mutex<Vec<Request>>>,
        request_responses: Arc<Mutex<VecDeque<Response>>>,
    }

    impl FakeClient {
        fn new(topics: Vec<TopicInfo>) -> Self {
            let mut request_responses = VecDeque::new();
            request_responses.push_back(Response::NodeList(vec![]));
            request_responses.push_back(Response::PoseList(vec![]));

            Self {
                topics,
                subscribe_calls: Arc::new(Mutex::new(Vec::new())),
                unsubscribe_calls: Arc::new(Mutex::new(Vec::new())),
                request_calls: Arc::new(Mutex::new(Vec::new())),
                request_responses: Arc::new(Mutex::new(request_responses)),
            }
        }

        fn subscribe_calls(&self) -> Vec<Vec<String>> {
            self.subscribe_calls.lock().unwrap().clone()
        }

        fn unsubscribe_calls(&self) -> Vec<Vec<String>> {
            self.unsubscribe_calls.lock().unwrap().clone()
        }

        fn request_calls(&self) -> Vec<Request> {
            self.request_calls.lock().unwrap().clone()
        }
    }

    impl ProtocolClient for FakeClient {
        async fn request(&mut self, req: Request) -> Result<Response, Error> {
            self.request_calls.lock().unwrap().push(req.clone());

            match req {
                Request::ListTopics => Ok(Response::TopicList(self.topics.clone())),
                Request::Subscribe { topics } => Ok(Response::Subscribed {
                    topics: topics
                        .into_iter()
                        .map(|topic| TopicSub {
                            type_name: self
                                .topics
                                .iter()
                                .find(|info| info.name == topic)
                                .map(|info| info.type_name.clone())
                                .unwrap_or_default(),
                            topic,
                        })
                        .collect(),
                }),
                Request::Unsubscribe { topics } => Ok(Response::Unsubscribed { topics }),
                _ => self
                    .request_responses
                    .lock()
                    .unwrap()
                    .pop_front()
                    .ok_or_else(|| Error::Config("missing fake response".into())),
            }
        }

        async fn subscribe(&mut self, topics: &[String]) -> Result<Vec<TopicSub>, Error> {
            self.subscribe_calls.lock().unwrap().push(topics.to_vec());
            Ok(topics
                .iter()
                .cloned()
                .map(|topic| TopicSub {
                    type_name: self
                        .topics
                        .iter()
                        .find(|info| info.name == topic)
                        .map(|info| info.type_name.clone())
                        .unwrap_or_default(),
                    topic,
                })
                .collect())
        }

        async fn unsubscribe(&mut self, topics: &[String]) -> Result<Vec<String>, Error> {
            self.unsubscribe_calls.lock().unwrap().push(topics.to_vec());
            Ok(topics.to_vec())
        }

        async fn recv_data(
            &mut self,
        ) -> Result<(String, talos_common::protocol::types::TopicFrame), Error> {
            std::future::pending().await
        }
    }

    fn sample_topics() -> Vec<TopicInfo> {
        vec![
            TopicInfo {
                name: "/camera".into(),
                type_name: "sensor_msgs/msg/Image".into(),
                publisher_count: 1,
                subscriber_count: 0,
            },
            TopicInfo {
                name: "/rosout".into(),
                type_name: "rcl_interfaces/msg/Log".into(),
                publisher_count: 1,
                subscriber_count: 0,
            },
        ]
    }

    #[tokio::test]
    async fn reconnect_respects_previous_unsubscribe_choice() {
        let state = Arc::new(Mutex::new(AppState::default()));
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        drop(cmd_tx);

        let first_client = FakeClient::new(sample_topics());
        connect_and_run(first_client.clone(), &state, &mut cmd_rx)
            .await
            .unwrap();

        {
            let mut app = state.lock().unwrap();
            app.topic_selected = 1;
            assert_eq!(
                app.toggle_selected_topic_subscription(),
                Some(Request::Unsubscribe {
                    topics: vec!["/rosout".to_string()]
                })
            );
        }

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        drop(cmd_tx);
        let second_client = FakeClient::new(sample_topics());
        connect_and_run(second_client.clone(), &state, &mut cmd_rx)
            .await
            .unwrap();

        assert_eq!(
            first_client.subscribe_calls(),
            vec![vec!["/camera".to_string(), "/rosout".to_string()]]
        );
        assert_eq!(
            second_client.subscribe_calls(),
            vec![vec!["/camera".to_string()]]
        );
    }

    #[tokio::test]
    async fn manual_unsubscribe_uses_protocol_helper() {
        let state = Arc::new(Mutex::new(AppState::default()));
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        cmd_tx
            .send(Request::Unsubscribe {
                topics: vec!["/rosout".into()],
            })
            .unwrap();
        drop(cmd_tx);

        let client = FakeClient::new(sample_topics());
        connect_and_run(client.clone(), &state, &mut cmd_rx)
            .await
            .unwrap();

        assert_eq!(
            client.unsubscribe_calls(),
            vec![vec!["/rosout".to_string()]]
        );
        assert!(
            !client
                .request_calls()
                .iter()
                .any(|request| matches!(request, Request::Unsubscribe { .. }))
        );
    }
}
