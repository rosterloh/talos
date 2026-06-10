use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use talos_common::config::AgentConfig;
use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};
use talos_common::transport::uds::UdsTransport;
use talos_common::transport::{TransportConfig, TransportServer};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info};

use super::RouterHandle;
use super::requests::handle_request;
use crate::{GraphHandle, JointPublisher};

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
