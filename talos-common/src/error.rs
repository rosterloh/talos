use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("frame too large: {size} bytes (max: {max})")]
    FrameTooLarge { size: usize, max: usize },

    #[error("bincode encode error: {0}")]
    Encode(#[from] bincode::error::EncodeError),

    #[error("bincode decode error: {0}")]
    Decode(#[from] bincode::error::DecodeError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("urdf parse error: {0}")]
    Urdf(String),
}
