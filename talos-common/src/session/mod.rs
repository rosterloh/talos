//! Application-facing protocol session layer.
#![allow(async_fn_in_trait)]
//!
//! This module owns the client API used by `talos-cli` and `talos-tui`.
//! Application code should use the [`ProtocolClient`] trait rather than opening
//! sockets, streams, or listeners from [`crate::transport`] directly.
//!
//! The session layer translates application requests into framed protocol
//! exchanges, buffers topic data that arrives while a control response is
//! pending, and hides UDS versus QUIC stream layout from callers.
//!
//! Concrete implementations:
//! - [`UdsProtocolClient`]: single framed UDS connection
//! - [`QuicProtocolClient`] (feature `quic`): QUIC bidirectional control stream +
//!   server-initiated unidirectional data streams

pub mod uds;

#[cfg(feature = "quic")]
pub mod quic;

pub use uds::UdsProtocolClient;

#[cfg(feature = "quic")]
pub use quic::QuicProtocolClient;

use crate::error::Error;
use crate::protocol::messages::{Request, Response};
use crate::protocol::types::{TopicFrame, TopicSub};

/// Transport-agnostic interface for application code that needs to talk to the
/// Talos agent.
///
/// All methods take `&mut self` — implementations are **not** expected to be
/// called concurrently from different tasks.  They **are** designed to work
/// inside a `tokio::select!` — in particular, `recv_data()` is cancel-safe.
#[allow(async_fn_in_trait)]
pub trait ProtocolClient {
    /// Send a control `Request` and return the corresponding `Response`.
    ///
    /// Any `TopicData` frames that arrive while waiting for the control response
    /// are buffered and returned by subsequent calls to `recv_data()`.
    async fn request(&mut self, req: Request) -> Result<Response, Error>;

    /// Send a `Subscribe` request and update the local subscription set.
    /// Returns the confirmed subscriptions with type information.
    async fn subscribe(&mut self, topics: &[String]) -> Result<Vec<TopicSub>, Error>;

    /// Send an `Unsubscribe` request and update the local subscription set.
    /// Returns the list of topics that were unsubscribed.
    async fn unsubscribe(&mut self, topics: &[String]) -> Result<Vec<String>, Error>;

    /// Return the next data frame from any subscribed topic.
    ///
    /// Returns `(topic_name, frame)`. This method is cancel-safe.
    async fn recv_data(&mut self) -> Result<(String, TopicFrame), Error>;
}
