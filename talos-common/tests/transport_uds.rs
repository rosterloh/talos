use std::sync::atomic::{AtomicU32, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio_util::codec::{FramedRead, FramedWrite};

use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::TopicInfo;
use talos_common::transport::uds::UdsTransport;
use talos_common::transport::{TransportConfig, TransportServer};

static TEST_ID: AtomicU32 = AtomicU32::new(0);

fn test_config() -> TransportConfig {
    let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
    let path = format!("/tmp/talos-test-{}-{id}.sock", std::process::id());
    TransportConfig { socket_path: path }
}

#[tokio::test]
async fn client_server_round_trip() {
    let config = test_config();
    let listener = UdsTransport::bind(&config).await.unwrap();

    let client_path = config.socket_path.clone();
    let server = tokio::spawn(async move {
        let conn = UdsTransport::accept(&listener).await.unwrap();
        let mut reader = FramedRead::new(conn.reader, BincodeCodec::<Request>::new());
        let mut writer = FramedWrite::new(conn.writer, BincodeCodec::<Response>::new());

        let req = reader.next().await.unwrap().unwrap();
        assert_eq!(req, Request::ListTopics);

        let response = Response::TopicList(vec![TopicInfo {
            name: "/odom".into(),
            type_name: "nav_msgs/msg/Odometry".into(),
            publisher_count: 1,
            subscriber_count: 0,
        }]);
        writer.send(response).await.unwrap();
    });

    // Connect after bind — the OS queues the connection until accept
    let stream = UnixStream::connect(&client_path).await.unwrap();
    let (read_half, write_half) = stream.into_split();
    let mut writer = FramedWrite::new(write_half, BincodeCodec::<Request>::new());
    let mut reader = FramedRead::new(read_half, BincodeCodec::<Response>::new());

    writer.send(Request::ListTopics).await.unwrap();
    let resp = reader.next().await.unwrap().unwrap();

    server.await.unwrap();

    match resp {
        Response::TopicList(topics) => {
            assert_eq!(topics.len(), 1);
            assert_eq!(topics[0].name, "/odom");
        }
        other => panic!("unexpected response: {other:?}"),
    }

    let _ = std::fs::remove_file(&config.socket_path);
}

#[tokio::test]
async fn multiple_requests() {
    let config = test_config();
    let listener = UdsTransport::bind(&config).await.unwrap();

    let client_path = config.socket_path.clone();
    let server = tokio::spawn(async move {
        let conn = UdsTransport::accept(&listener).await.unwrap();
        let mut reader = FramedRead::new(conn.reader, BincodeCodec::<Request>::new());
        let mut writer = FramedWrite::new(conn.writer, BincodeCodec::<Response>::new());

        let req1 = reader.next().await.unwrap().unwrap();
        assert_eq!(req1, Request::ListTopics);
        let req2 = reader.next().await.unwrap().unwrap();
        assert_eq!(req2, Request::ListNodes);

        writer.send(Response::TopicList(vec![])).await.unwrap();
        writer.send(Response::NodeList(vec![])).await.unwrap();
    });

    let stream = UnixStream::connect(&client_path).await.unwrap();
    let (read_half, write_half) = stream.into_split();
    let mut writer = FramedWrite::new(write_half, BincodeCodec::<Request>::new());
    let mut reader = FramedRead::new(read_half, BincodeCodec::<Response>::new());

    writer.send(Request::ListTopics).await.unwrap();
    writer.send(Request::ListNodes).await.unwrap();

    let r1 = reader.next().await.unwrap().unwrap();
    let r2 = reader.next().await.unwrap().unwrap();

    server.await.unwrap();

    assert!(matches!(r1, Response::TopicList(_)));
    assert!(matches!(r2, Response::NodeList(_)));

    let _ = std::fs::remove_file(&config.socket_path);
}
