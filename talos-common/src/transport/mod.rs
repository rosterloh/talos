pub mod uds;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Error;

pub trait Transport {
    type Read: AsyncRead + Unpin + Send;
    type Write: AsyncWrite + Unpin + Send;
}

pub struct Connection<T: Transport> {
    pub reader: T::Read,
    pub writer: T::Write,
}

pub trait TransportClient: Transport + Sized {
    fn connect(
        config: &TransportConfig,
    ) -> impl Future<Output = Result<Connection<Self>, Error>> + Send;
}

pub trait TransportServer: Transport + Sized {
    type Listener: Send;

    fn bind(
        config: &TransportConfig,
    ) -> impl Future<Output = Result<Self::Listener, Error>> + Send;
    fn accept(
        listener: &Self::Listener,
    ) -> impl Future<Output = Result<Connection<Self>, Error>> + Send;
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub socket_path: String,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/talos.sock".into(),
        }
    }
}
