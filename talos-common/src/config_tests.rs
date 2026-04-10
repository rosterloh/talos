use std::io::Write;

use super::config::AgentConfig;

#[test]
fn parse_full_config() {
    let toml = r#"
[transport.uds]
socket_path = "/run/talos.sock"

[[subscriptions]]
topic = "/odom"
type = "nav_msgs/msg/Odometry"

[[subscriptions]]
topic = "/cmd_vel"
type = "geometry_msgs/msg/Twist"

[control]
method = "topic"
topic = "/joint_commands"
type = "sensor_msgs/msg/JointState"

[poses.home]
shoulder_pan = 0.0
elbow = 1.57

[poses.pick_ready]
shoulder_pan = 0.5
elbow = 0.8
"#;

    let config: AgentConfig = toml::from_str(toml).unwrap();
    let uds = config.transport.uds.as_ref().unwrap();
    assert_eq!(uds.socket_path, "/run/talos.sock");
    assert!(config.transport.quic.is_none());
    assert_eq!(config.subscriptions.len(), 2);
    assert_eq!(config.subscriptions[0].topic, "/odom");
    assert_eq!(config.subscriptions[0].msg_type, "nav_msgs/msg/Odometry");

    let control = config.control.unwrap();
    assert_eq!(control.topic, "/joint_commands");

    assert_eq!(config.poses.len(), 2);
    assert_eq!(config.poses["home"]["shoulder_pan"], 0.0);
    assert_eq!(config.poses["pick_ready"]["elbow"], 0.8);
}

#[test]
fn parse_dual_transport_config() {
    let toml = r#"
[transport.uds]
socket_path = "/tmp/talos.sock"

[transport.quic]
bind_addr = "0.0.0.0:4433"
"#;

    let config: AgentConfig = toml::from_str(toml).unwrap();
    let uds = config.transport.uds.as_ref().unwrap();
    assert_eq!(uds.socket_path, "/tmp/talos.sock");
    let quic = config.transport.quic.as_ref().unwrap();
    assert_eq!(quic.bind_addr, "0.0.0.0:4433");
    assert!(quic.cert_path.is_none());
    assert!(quic.key_path.is_none());
}

#[test]
fn parse_quic_only_config() {
    let toml = r#"
[transport.quic]
bind_addr = "0.0.0.0:9999"
cert_path = "/etc/talos/cert.pem"
key_path = "/etc/talos/key.pem"
"#;

    let config: AgentConfig = toml::from_str(toml).unwrap();
    // When [transport.quic] is present but no [transport.uds], uds is None
    assert!(config.transport.uds.is_none());
    let quic = config.transport.quic.as_ref().unwrap();
    assert_eq!(quic.bind_addr, "0.0.0.0:9999");
    assert_eq!(quic.cert_path.as_deref(), Some("/etc/talos/cert.pem"));
    assert_eq!(quic.key_path.as_deref(), Some("/etc/talos/key.pem"));
}

#[test]
fn parse_minimal_config() {
    let toml = "";
    let config: AgentConfig = toml::from_str(toml).unwrap();
    // With no [transport] section at all, uses TransportSettings::default() (UDS on default path)
    let uds = config.transport.uds.as_ref().unwrap();
    assert_eq!(uds.socket_path, "/tmp/talos.sock");
    assert!(config.subscriptions.is_empty());
    assert!(config.control.is_none());
    assert!(config.poses.is_empty());
}

#[test]
fn load_from_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(
        tmp,
        r#"
[transport.uds]
socket_path = "/tmp/test.sock"

[[subscriptions]]
topic = "/test"
type = "std_msgs/msg/String"
"#
    )
    .unwrap();

    let config = AgentConfig::load(tmp.path()).unwrap();
    let uds = config.transport.uds.as_ref().unwrap();
    assert_eq!(uds.socket_path, "/tmp/test.sock");
    assert_eq!(config.subscriptions.len(), 1);
}

#[test]
fn load_or_default_missing_file() {
    let config = AgentConfig::load_or_default(None).unwrap();
    let uds = config.transport.uds.as_ref().unwrap();
    assert_eq!(uds.socket_path, "/tmp/talos.sock");
}

#[test]
fn uds_socket_path_helper() {
    let config = AgentConfig::default();
    assert_eq!(config.uds_socket_path(), Some("/tmp/talos.sock"));
}
