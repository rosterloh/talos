use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub transport: TransportSettings,
    pub subscriptions: Vec<SubscriptionConfig>,
    pub control: Option<ControlConfig>,
    pub poses: HashMap<String, HashMap<String, f64>>,
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

/// QoS profile for a subscription.
///
/// `Default` uses the rclrs default QoS profile (Reliable reliability, Volatile durability,
/// KeepLast history). The exact depth is inherited from the rclrs/rmw default and is not
/// enforced explicitly here. Suitable for infrequent or control topics.
///
/// `SensorData` maps to `QOS_PROFILE_SENSOR_DATA`: BestEffort reliability, Volatile durability,
/// KeepLast history with depth 5. Suitable for high-rate sensor topics (laser, IMU, etc.)
/// where occasional message loss is acceptable and low latency matters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QosProfile {
    #[default]
    Default,
    SensorData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionConfig {
    pub topic: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub qos: QosProfile,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_without_qos_defaults_to_default_profile() {
        let toml = r#"
            topic = "/scan"
            type  = "sensor_msgs/msg/LaserScan"
        "#;
        let sub: SubscriptionConfig = toml::from_str(toml).unwrap();
        assert!(matches!(sub.qos, QosProfile::Default));
    }

    #[test]
    fn subscription_with_sensor_data_qos_parses_correctly() {
        let toml = r#"
            topic = "/scan"
            type  = "sensor_msgs/msg/LaserScan"
            qos   = "sensor_data"
        "#;
        let sub: SubscriptionConfig = toml::from_str(toml).unwrap();
        assert!(matches!(sub.qos, QosProfile::SensorData));
    }

    #[test]
    fn subscription_with_unknown_qos_returns_error() {
        let toml = r#"
            topic = "/scan"
            type  = "sensor_msgs/msg/LaserScan"
            qos   = "best_effort"
        "#;
        assert!(toml::from_str::<SubscriptionConfig>(toml).is_err());
    }
}
