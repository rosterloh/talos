use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub transport: TransportSettings,
    pub subscriptions: Vec<SubscriptionConfig>,
    pub control: Option<ControlConfig>,
    pub poses: HashMap<String, HashMap<String, f64>>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            transport: TransportSettings::default(),
            subscriptions: Vec::new(),
            control: None,
            poses: HashMap::new(),
        }
    }
}

/// Top-level transport settings. Both `uds` and `quic` are optional; at least one should be
/// configured for the agent to accept connections. Defaults to UDS-only on the default socket
/// path when the config file is absent (via `AgentConfig::default()`).
///
/// Note: **no** `#[serde(default)]` here — individual `Option<T>` fields already default to
/// `None` when absent from the TOML.  Adding `#[serde(default)]` on the struct would cause
/// absent fields to use `TransportSettings::default()` (which enables UDS), overriding the
/// intent of a `[transport.quic]`-only config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportSettings {
    pub uds: Option<UdsTransportConfig>,
    pub quic: Option<QuicTransportConfig>,
}

impl Default for TransportSettings {
    fn default() -> Self {
        Self {
            uds: Some(UdsTransportConfig::default()),
            quic: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdsTransportConfig {
    pub socket_path: String,
}

impl Default for UdsTransportConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/talos.sock".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicTransportConfig {
    pub bind_addr: String,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

impl Default for QuicTransportConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:4433".into(),
            cert_path: None,
            key_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionConfig {
    pub topic: String,
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlConfig {
    pub method: ControlMethod,
    pub topic: String,
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlMethod {
    Topic,
    Action,
}

impl AgentConfig {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("failed to read config file: {e}")))?;
        let config: AgentConfig = toml::from_str(&contents)
            .map_err(|e| Error::Config(format!("failed to parse config: {e}")))?;
        Ok(config)
    }

    pub fn load_or_default(path: Option<&Path>) -> Result<Self, Error> {
        match path {
            Some(p) => Self::load(p),
            None => {
                let default_path = Path::new("talos-agent.toml");
                if default_path.exists() {
                    Self::load(default_path)
                } else {
                    Ok(Self::default())
                }
            }
        }
    }

    /// Returns the UDS socket path if UDS transport is configured.
    pub fn uds_socket_path(&self) -> Option<&str> {
        self.transport.uds.as_ref().map(|u| u.socket_path.as_str())
    }
}
