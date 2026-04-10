//! Public interface for the talos-agent crate.
//!
//! This `lib.rs` exists to enable integration tests under `tests/` and to
//! expose stable types.  The `bridge` module (ROS 2 subscriptions) and
//! `conversions` module (message adapters) are pub only to allow `main.rs` to
//! reference them — they are not part of the public integration-test surface.

pub mod bridge;
pub mod conversions;
pub mod router;
pub mod server;

use std::sync::Arc;
use tokio::sync::Mutex;

/// Handle to an optional `rclrs` joint-state publisher shared between the
/// bridge task (writer) and the request handler (reader).
pub type JointPublisher = Arc<Mutex<Option<rclrs::Publisher<sensor_msgs::msg::JointState>>>>;
