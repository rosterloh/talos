use tokio::net::{UnixListener, UnixStream};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

use super::{Connection, Transport, TransportClient, TransportConfig, TransportServer};
use crate::error::Error;

pub struct UdsTransport;

impl Transport for UdsTransport {
    type Read = OwnedReadHalf;
    type Write = OwnedWriteHalf;
}

impl TransportClient for UdsTransport {
    async fn connect(config: &TransportConfig) -> Result<Connection<Self>, Error> {
        let stream = UnixStream::connect(&config.socket_path).await?;
        let (reader, writer) = stream.into_split();
        Ok(Connection { reader, writer })
    }
}

impl TransportServer for UdsTransport {
    type Listener = UnixListener;

    async fn bind(config: &TransportConfig) -> Result<Self::Listener, Error> {
        // Remove stale socket file if it exists
        let _ = std::fs::remove_file(&config.socket_path);
        let listener = UnixListener::bind(&config.socket_path)?;
        Ok(listener)
    }

    async fn accept(listener: &Self::Listener) -> Result<Connection<Self>, Error> {
        let (stream, _addr) = listener.accept().await?;
        let (reader, writer) = stream.into_split();
        Ok(Connection { reader, writer })
    }
}
