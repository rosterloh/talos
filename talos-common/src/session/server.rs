//! Server-side session abstraction.
//!
//! `ProtocolSession` tracks per-client subscriptions. Transport-specific data
//! push (UDS: `Response::TopicData`, QUIC: `TopicFrame` on uni streams) is
//! handled by the agent's server code which owns the write handles.

use std::collections::HashSet;

/// Per-client session state managed by the agent.
///
/// Tracks which topics this client has subscribed to so that the [`TopicRouter`]
/// can route data correctly. Transport-specific send logic lives in the agent.
///
/// [`TopicRouter`]: crate::session::server::ProtocolSession
pub struct ProtocolSession {
    subscriptions: HashSet<String>,
}

impl ProtocolSession {
    pub fn new() -> Self {
        Self {
            subscriptions: HashSet::new(),
        }
    }

    /// Record a subscription for a set of topics.
    pub fn subscribe(&mut self, topics: impl IntoIterator<Item = String>) {
        for t in topics {
            self.subscriptions.insert(t);
        }
    }

    /// Remove a subscription for the given topics.
    pub fn unsubscribe(&mut self, topics: &[String]) {
        for t in topics {
            self.subscriptions.remove(t);
        }
    }

    /// Returns `true` if the client is subscribed to `topic`.
    pub fn is_subscribed(&self, topic: &str) -> bool {
        self.subscriptions.contains(topic)
    }

    /// Iterate over all currently subscribed topics.
    pub fn subscriptions(&self) -> impl Iterator<Item = &str> {
        self.subscriptions.iter().map(String::as_str)
    }
}

impl Default for ProtocolSession {
    fn default() -> Self {
        Self::new()
    }
}
