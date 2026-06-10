use std::sync::Arc;

use tokio::sync::Mutex as TokioMutex;

use crate::router::TopicRouter;

mod control;
mod graph;
mod parameters;
#[cfg(feature = "quic")]
mod quic;
mod requests;
mod uds;

pub use uds::run;

#[cfg(feature = "quic")]
pub use quic::{handle_quic_client, run_quic};

pub type RouterHandle = Arc<TokioMutex<TopicRouter>>;
