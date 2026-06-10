//! Shared Talos types and helpers that do not depend on ROS 2.
//!
//! `talos-common` is the boundary crate used by the robot-side agent and the
//! workstation-side CLI/TUI clients. Keep ROS 2 bindings out of this crate.
//!
//! Module responsibilities:
//!
//! - [`protocol`]: wire schema, dynamic message values, and length-prefixed
//!   framing types.
//! - [`session`]: application-facing client API. CLI and TUI code should use
//!   [`session::ProtocolClient`] instead of opening transports directly.
//! - [`transport`]: endpoint, listener, and connection plumbing for concrete
//!   transports such as Unix domain sockets and QUIC.
//! - [`config`]: shared TOML configuration model.
//! - [`urdf`]: shared URDF parsing helpers.
//! - [`error`]: common error type used across shared code.

pub mod config;
pub mod error;
pub mod protocol;
pub mod session;
pub mod transport;
pub mod urdf;

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod urdf_tests;
