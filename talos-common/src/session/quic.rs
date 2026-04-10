//! [`ProtocolClient`] implementation over a QUIC connection.
//!
//! - Bidirectional stream 0 carries control messages (Request / Response).
//! - The agent opens one unidirectional stream per subscribed topic.  Each
//!   stream begins with a `StreamHeader` frame followed by `TopicFrame` frames.
//! - A background task accepts incoming unidirectional streams and routes their
//!   frames to the internal `data_rx` channel.

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use quinn::{RecvStream, SendStream};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::error::Error;
use crate::protocol::codec::BincodeCodec;
use crate::protocol::messages::{Request, Response};
use crate::protocol::types::{StreamHeader, TopicFrame, TopicSub};
use crate::transport::quic::QuicTransport;

use super::ProtocolClient;

/// A [`ProtocolClient`] that communicates over QUIC.
pub struct QuicProtocolClient {
    /// Framed write-half of the bidirectional control stream (stream 0).
    control_tx: FramedWrite<SendStream, BincodeCodec<Request>>,
    /// Framed read-half of the bidirectional control stream (stream 0).
    control_rx: FramedRead<RecvStream, BincodeCodec<Response>>,
    /// Data frames delivered by the background accept task.
    data_rx: mpsc::UnboundedReceiver<(String, TopicFrame)>,
    /// Kept alive so the accept/read tasks don't drop their sender handles.
    _accept_task: tokio::task::JoinHandle<()>,
}

impl QuicProtocolClient {
    /// Connect to the Talos agent at `addr` (e.g. `"192.168.1.50:4433"`).
    pub async fn connect(addr: &str) -> Result<Self, Error> {
        let addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| Error::Config(format!("invalid QUIC address '{addr}': {e}")))?;

        let connection = QuicTransport::connect(addr).await?;

        // Open the bidirectional control stream (stream 0)
        let (send, recv) = connection.open_bi().await.map_err(|e| {
            Error::Config(format!("failed to open QUIC control stream: {e}"))
        })?;

        let control_tx = FramedWrite::new(send, BincodeCodec::new());
        let control_rx = FramedRead::new(recv, BincodeCodec::new());

        // Spawn a task that accepts server-initiated unidirectional data streams
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let conn_clone = connection.clone();
        let accept_task = tokio::spawn(async move {
            accept_data_streams(conn_clone, data_tx).await;
        });

        Ok(Self {
            control_tx,
            control_rx,
            data_rx,
            _accept_task: accept_task,
        })
    }
}

impl ProtocolClient for QuicProtocolClient {
    async fn request(&mut self, req: Request) -> Result<Response, Error> {
        self.control_tx.send(req).await?;
        match self.control_rx.next().await {
            Some(Ok(resp)) => Ok(resp),
            Some(Err(e)) => Err(e),
            None => Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "QUIC control stream closed",
            ))),
        }
    }

    async fn subscribe(&mut self, topics: &[String]) -> Result<Vec<TopicSub>, Error> {
        let resp = self
            .request(Request::Subscribe {
                topics: topics.to_vec(),
            })
            .await?;
        match resp {
            Response::Subscribed { topics } => Ok(topics),
            Response::Error(e) => Err(Error::Config(format!("subscribe error: {e}"))),
            other => Err(Error::Config(format!(
                "unexpected response to Subscribe: {other:?}"
            ))),
        }
    }

    async fn unsubscribe(&mut self, topics: &[String]) -> Result<Vec<String>, Error> {
        let resp = self
            .request(Request::Unsubscribe {
                topics: topics.to_vec(),
            })
            .await?;
        match resp {
            Response::Unsubscribed { topics } => Ok(topics),
            Response::Error(e) => Err(Error::Config(format!("unsubscribe error: {e}"))),
            other => Err(Error::Config(format!(
                "unexpected response to Unsubscribe: {other:?}"
            ))),
        }
    }

    /// Returns the next data frame from any subscribed topic.
    ///
    /// Blocks until a `TopicFrame` is delivered by the background accept task.
    /// This method is cancel-safe because it only awaits on an `mpsc::Receiver`.
    async fn recv_data(&mut self) -> Result<(String, TopicFrame), Error> {
        self.data_rx
            .recv()
            .await
            .ok_or_else(|| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "QUIC data channel closed",
                ))
            })
    }
}

/// Background task: accept server-initiated unidirectional streams and spawn a
/// reader task per stream.
async fn accept_data_streams(
    connection: quinn::Connection,
    data_tx: mpsc::UnboundedSender<(String, TopicFrame)>,
) {
    loop {
        match connection.accept_uni().await {
            Ok(stream) => {
                let tx = data_tx.clone();
                tokio::spawn(async move {
                    read_data_stream(stream, tx).await;
                });
            }
            Err(_) => break,
        }
    }
}

/// Read a single server-initiated data stream:
/// 1. First frame → `StreamHeader` (topic name + type)
/// 2. Subsequent frames → `TopicFrame` (stamp + data)
///
/// Uses `LengthDelimitedCodec` so the same codec handles both frame types
/// without any buffering ambiguity when switching between header and data.
async fn read_data_stream(
    stream: RecvStream,
    tx: mpsc::UnboundedSender<(String, TopicFrame)>,
) {
    let codec = LengthDelimitedCodec::builder()
        .length_field_length(4)
        .big_endian()
        .new_codec();
    let mut framed = FramedRead::new(stream, codec);

    // ── header frame ──────────────────────────────────────────────────────────
    let header_bytes: Bytes = match framed.next().await {
        Some(Ok(b)) => b.freeze(),
        _ => return,
    };
    let header: StreamHeader = match bincode::deserialize(&header_bytes) {
        Ok(h) => h,
        Err(_) => return,
    };

    // ── data frames ───────────────────────────────────────────────────────────
    while let Some(Ok(frame_bytes)) = framed.next().await {
        let frame: TopicFrame = match bincode::deserialize(&frame_bytes) {
            Ok(f) => f,
            Err(_) => break,
        };
        if tx.send((header.topic.clone(), frame)).is_err() {
            break;
        }
    }
}
