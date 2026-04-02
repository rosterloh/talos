use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{error, info, warn};

use talos_common::protocol::codec::BincodeCodec;
use talos_common::protocol::messages::{Request, Response};

use crate::state::AppState;

pub async fn run(
    socket_path: String,
    state: Arc<Mutex<AppState>>,
    mut cmd_rx: mpsc::UnboundedReceiver<Request>,
) {
    loop {
        match connect_and_run(&socket_path, &state, &mut cmd_rx).await {
            Ok(()) => {
                info!("connection closed");
            }
            Err(e) => {
                warn!("connection error: {e}");
            }
        }

        {
            let mut s = state.lock().unwrap();
            s.connected = false;
        }

        // Reconnect after a delay
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        info!("reconnecting...");
    }
}

async fn connect_and_run(
    socket_path: &str,
    state: &Arc<Mutex<AppState>>,
    cmd_rx: &mut mpsc::UnboundedReceiver<Request>,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = UnixStream::connect(socket_path).await?;
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, BincodeCodec::<Response>::new());
    let mut writer = FramedWrite::new(write_half, BincodeCodec::<Request>::new());

    {
        let mut s = state.lock().unwrap();
        s.connected = true;
    }

    info!(path = %socket_path, "connected to agent");

    // Request initial data
    if let Err(e) = writer.send(Request::ListTopics).await {
        error!("failed to send ListTopics: {e}");
    }
    if let Err(e) = writer.send(Request::ListNodes).await {
        error!("failed to send ListNodes: {e}");
    }

    loop {
        tokio::select! {
            result = reader.next() => {
                match result {
                    Some(Ok(response)) => {
                        let mut s = state.lock().unwrap();
                        s.handle_response(response);
                    }
                    Some(Err(e)) => {
                        error!("read error: {e}");
                        break;
                    }
                    None => break,
                }
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(request) => {
                        if let Err(e) = writer.send(request).await {
                            error!("failed to send request: {e}");
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}
