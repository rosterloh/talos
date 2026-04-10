//! Per-client subscription tracking and topic routing.
//!
//! [`TopicRouter`] replaces the `broadcast::channel` used in v0.1.  Instead of
//! sending every message to every client, it keeps a per-client subscription
//! set and routes `TopicData` responses only to clients that have subscribed to
//! the relevant topic.

use std::collections::{HashMap, HashSet};

use talos_common::protocol::messages::Response;
use tokio::sync::mpsc;

pub type ClientId = u64;

/// Manages per-client subscriptions and routes topic data.
///
/// Wrap in `Arc<tokio::sync::Mutex<TopicRouter>>` and share between the bridge
/// task (caller of [`route`]) and all client handler tasks.
pub struct TopicRouter {
    clients: HashMap<ClientId, ClientEntry>,
    next_id: ClientId,
}

struct ClientEntry {
    subscriptions: HashSet<String>,
    tx: mpsc::UnboundedSender<Response>,
}

impl TopicRouter {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            next_id: 0,
        }
    }

    /// Register a new client and return its ID and the channel it should receive
    /// data on.
    pub fn register(&mut self) -> (ClientId, mpsc::UnboundedReceiver<Response>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = self.next_id;
        self.next_id += 1;
        self.clients.insert(
            id,
            ClientEntry {
                subscriptions: HashSet::new(),
                tx,
            },
        );
        (id, rx)
    }

    /// Remove a client's subscription state.
    pub fn deregister(&mut self, id: ClientId) {
        self.clients.remove(&id);
    }

    /// Add topics to a client's subscription set.
    pub fn subscribe(&mut self, id: ClientId, topics: impl IntoIterator<Item = String>) {
        if let Some(entry) = self.clients.get_mut(&id) {
            for t in topics {
                entry.subscriptions.insert(t);
            }
        }
    }

    /// Remove topics from a client's subscription set.
    pub fn unsubscribe(&mut self, id: ClientId, topics: &[String]) {
        if let Some(entry) = self.clients.get_mut(&id) {
            for t in topics {
                entry.subscriptions.remove(t);
            }
        }
    }

    /// Route a `TopicData` response to all subscribed clients.
    ///
    /// Non-`TopicData` responses are silently ignored.
    pub fn route(&self, response: &Response) {
        if let Response::TopicData { topic, .. } = response {
            for entry in self.clients.values() {
                if entry.subscriptions.contains(topic.as_str()) {
                    let _ = entry.tx.send(response.clone());
                }
            }
        }
    }
}

impl Default for TopicRouter {
    fn default() -> Self {
        Self::new()
    }
}
