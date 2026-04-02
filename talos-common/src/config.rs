use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub transport: TransportConfig,
    pub subscriptions: Vec<SubscriptionConfig>,
    pub control: Option<ControlConfig>,
    pub poses: HashMap<String, HashMap<String, f64>>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            subscriptions: Vec::new(),
            control: None,
            poses: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn into_transport_config(&self) -> crate::transport::TransportConfig {
        crate::transport::TransportConfig {
            socket_path: self.transport.socket_path.clone(),
        }
    }
}
