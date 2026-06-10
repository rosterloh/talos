//! [`ProtocolClient`] implementation over a Unix domain socket connection.

use std::collections::{HashSet, VecDeque};

use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::error::Error;
use crate::protocol::codec::BincodeCodec;
use crate::protocol::messages::{Request, Response};
use crate::protocol::types::{TopicFrame, TopicSub};

use super::ProtocolClient;

/// A [`ProtocolClient`] that communicates with the agent over a Unix domain socket.
///
/// Control requests (ListTopics, Subscribe, …) and `TopicData` frames share the
/// same framed byte stream and are demultiplexed internally.
pub struct UdsProtocolClient {
    reader: FramedRead<OwnedReadHalf, BincodeCodec<Response>>,
    writer: FramedWrite<OwnedWriteHalf, BincodeCodec<Request>>,
    /// Buffered data frames received while waiting for a control response.
    data_queue: VecDeque<(String, TopicFrame)>,
    /// Topics the client has subscribed to (used to filter incoming TopicData).
    subscriptions: HashSet<String>,
}

impl UdsProtocolClient {
    /// Connect to the agent at `socket_path` and return a ready client.
    pub async fn connect(socket_path: &str) -> Result<Self, Error> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = stream.into_split();
        Ok(Self {
            reader: FramedRead::new(read_half, BincodeCodec::new()),
            writer: FramedWrite::new(write_half, BincodeCodec::new()),
            data_queue: VecDeque::new(),
            subscriptions: HashSet::new(),
        })
    }

    /// Read frames from the stream until a control response arrives.
    ///
    /// Any `TopicData` frames encountered along the way are pushed onto the
    /// internal `data_queue` so they can be returned by `recv_data()` later.
    async fn read_control_response(&mut self) -> Result<Response, Error> {
        loop {
            match self.reader.next().await {
                Some(Ok(Response::TopicData {
                    topic, stamp, data, ..
                })) => {
                    if self.subscriptions.contains(&topic) {
                        self.data_queue
                            .push_back((topic, TopicFrame { stamp, data }));
                    }
                }
                Some(Ok(response)) => return Ok(response),
                Some(Err(e)) => return Err(e),
                None => {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "UDS connection closed",
                    )));
                }
            }
        }
    }
}

impl ProtocolClient for UdsProtocolClient {
    async fn request(&mut self, req: Request) -> Result<Response, Error> {
        self.writer.send(req).await?;
        self.read_control_response().await
    }

    async fn subscribe(&mut self, topics: &[String]) -> Result<Vec<TopicSub>, Error> {
        // Pre-register the requested topics so that any `TopicData` frames arriving
        // between the Subscribe request and the Subscribed response are buffered
        // rather than silently dropped.
        for t in topics {
            self.subscriptions.insert(t.clone());
        }

        let response = self
            .request(Request::Subscribe {
                topics: topics.to_vec(),
            })
            .await;

        match response {
            Ok(Response::Subscribed { topics: confirmed }) => {
                // Replace pre-registered entries with the server-confirmed set
                for t in topics {
                    self.subscriptions.remove(t);
                }
                for t in &confirmed {
                    self.subscriptions.insert(t.topic.clone());
                }
                Ok(confirmed)
            }
            Ok(Response::Error(e)) => {
                for t in topics {
                    self.subscriptions.remove(t);
                }
                Err(Error::Config(format!("subscribe error: {e}")))
            }
            Ok(other) => {
                for t in topics {
                    self.subscriptions.remove(t);
                }
                Err(Error::Config(format!(
                    "unexpected response to Subscribe: {other:?}"
                )))
            }
            Err(e) => {
                for t in topics {
                    self.subscriptions.remove(t);
                }
                Err(e)
            }
        }
    }

    async fn unsubscribe(&mut self, topics: &[String]) -> Result<Vec<String>, Error> {
        let response = self
            .request(Request::Unsubscribe {
                topics: topics.to_vec(),
            })
            .await?;
        match response {
            Response::Unsubscribed { topics } => {
                for t in &topics {
                    self.subscriptions.remove(t);
                }
                Ok(topics)
            }
            Response::Error(e) => Err(Error::Config(format!("unsubscribe error: {e}"))),
            other => Err(Error::Config(format!(
                "unexpected response to Unsubscribe: {other:?}"
            ))),
        }
    }

    /// Returns the next data frame from any subscribed topic.
    ///
    /// Buffered frames (queued while waiting for control responses) are
    /// returned first.  Otherwise the underlying stream is polled.
    ///
    /// This method is cancel-safe: cancelling it in a `select!` leaves the
    /// `FramedRead` internal buffer intact so the next poll resumes correctly.
    async fn recv_data(&mut self) -> Result<(String, TopicFrame), Error> {
        // Drain the buffer first
        if let Some(frame) = self.data_queue.pop_front() {
            return Ok(frame);
        }

        loop {
            match self.reader.next().await {
                Some(Ok(Response::TopicData {
                    topic, stamp, data, ..
                })) => {
                    if self.subscriptions.contains(&topic) {
                        return Ok((topic, TopicFrame { stamp, data }));
                    }
                    // Not subscribed; discard silently.
                }
                Some(Ok(_)) => {
                    // Unsolicited control response while waiting for data.
                    // Drop it — the agent should not send these unprompted.
                }
                Some(Err(e)) => return Err(e),
                None => {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "UDS connection closed",
                    )));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use tempfile::TempDir;
    use tokio::net::UnixListener;
    use tokio_util::codec::{FramedRead, FramedWrite};

    use crate::protocol::codec::BincodeCodec;
    use crate::protocol::messages::{Request, Response};
    use crate::protocol::types::{DynValue, Timestamp, TopicInfo, TopicSub};

    use super::ProtocolClient;
    use super::UdsProtocolClient;

    /// Spawn a minimal UDS server that handles one client connection.
    async fn spawn_test_server(socket_path: String) {
        let listener = UnixListener::bind(&socket_path).unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (r, w) = stream.into_split();
            let mut reader = FramedRead::new(r, BincodeCodec::<Request>::new());
            let mut writer = FramedWrite::new(w, BincodeCodec::<Response>::new());

            while let Some(Ok(req)) = reader.next().await {
                let resp = match req {
                    Request::ListTopics => Response::TopicList(vec![TopicInfo {
                        name: "/odom".into(),
                        type_name: "nav_msgs/msg/Odometry".into(),
                        publisher_count: 1,
                        subscriber_count: 0,
                    }]),
                    Request::Subscribe { topics } => {
                        let subs: Vec<TopicSub> = topics
                            .iter()
                            .map(|t| TopicSub {
                                topic: t.clone(),
                                type_name: "nav_msgs/msg/Odometry".into(),
                            })
                            .collect();
                        // Send a TopicData frame BEFORE the Subscribed response to test demux
                        let _ = writer
                            .send(Response::TopicData {
                                topic: topics[0].clone(),
                                type_name: "nav_msgs/msg/Odometry".into(),
                                stamp: Timestamp { sec: 1, nanosec: 0 },
                                data: DynValue::Bool(true),
                            })
                            .await;
                        Response::Subscribed { topics: subs }
                    }
                    _ => Response::Error("not handled".into()),
                };
                if writer.send(resp).await.is_err() {
                    break;
                }
            }
        });
    }

    #[tokio::test]
    async fn test_uds_request() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.sock").to_string_lossy().into_owned();
        spawn_test_server(path.clone()).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut client = UdsProtocolClient::connect(&path).await.unwrap();
        let resp = client.request(Request::ListTopics).await.unwrap();
        match resp {
            Response::TopicList(topics) => assert_eq!(topics[0].name, "/odom"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_uds_demux_data_before_control() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test2.sock").to_string_lossy().into_owned();
        spawn_test_server(path.clone()).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut client = UdsProtocolClient::connect(&path).await.unwrap();

        // Subscribe — the test server sends a TopicData before the Subscribed response
        let subs = client.subscribe(&["/odom".into()]).await.unwrap();
        assert_eq!(subs[0].topic, "/odom");

        // The data frame sent before Subscribed should be buffered and available now
        let (topic, _frame) = client.recv_data().await.unwrap();
        assert_eq!(topic, "/odom");
    }
}
