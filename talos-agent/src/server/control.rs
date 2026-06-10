use talos_common::config::AgentConfig;
use talos_common::protocol::messages::Response;
use talos_common::protocol::types::PoseInfo;
use tracing::{error, info};

use crate::JointPublisher;

pub(super) async fn set_joint_position(
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
    joint: &str,
    position: f64,
) -> Response {
    if config.control.is_none() {
        return Response::Error("control not configured".into());
    }

    let guard = joint_publisher.lock().await;
    match guard.as_ref() {
        Some(publisher) => {
            let mut msg = sensor_msgs::msg::JointState::default();
            msg.name = vec![joint.to_string()];
            msg.position = vec![position];
            match publisher.publish(msg) {
                Ok(()) => {
                    info!(joint = %joint, position = %position, "published joint command");
                    Response::Ok("joint command published".into())
                }
                Err(e) => {
                    error!("failed to publish joint command: {e}");
                    Response::Error(format!("publish failed: {e}"))
                }
            }
        }
        None => Response::Error("joint publisher not ready".into()),
    }
}

pub(super) async fn execute_pose(
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
    name: &str,
) -> Response {
    if config.control.is_none() {
        return Response::Error("control not configured".into());
    }

    match config.poses.get(name) {
        Some(positions) => {
            let guard = joint_publisher.lock().await;
            match guard.as_ref() {
                Some(publisher) => {
                    let mut msg = sensor_msgs::msg::JointState::default();
                    let (names, pos): (Vec<_>, Vec<_>) =
                        positions.iter().map(|(k, v)| (k.clone(), *v)).unzip();
                    msg.name = names;
                    msg.position = pos;
                    match publisher.publish(msg) {
                        Ok(()) => {
                            info!(pose = %name, joints = positions.len(), "published pose");
                            Response::Ok(format!("pose '{name}' published"))
                        }
                        Err(e) => {
                            error!("failed to publish pose: {e}");
                            Response::Error(format!("publish failed: {e}"))
                        }
                    }
                }
                None => Response::Error("joint publisher not ready".into()),
            }
        }
        None => Response::Error(format!("unknown pose: {name}")),
    }
}

pub(super) fn configured_poses(config: &AgentConfig) -> Vec<PoseInfo> {
    let mut poses: Vec<PoseInfo> = config
        .poses
        .iter()
        .map(|(name, positions)| {
            let mut positions: Vec<(String, f64)> =
                positions.iter().map(|(k, v)| (k.clone(), *v)).collect();
            positions.sort_by(|a, b| a.0.cmp(&b.0));
            PoseInfo {
                name: name.clone(),
                positions,
            }
        })
        .collect();
    poses.sort_by(|a, b| a.name.cmp(&b.name));
    poses
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use talos_common::config::AgentConfig;

    use super::*;

    #[test]
    fn configured_poses_are_sorted_by_pose_and_joint_name() {
        let mut config = AgentConfig::default();
        config.poses = HashMap::from([
            (
                "stand".into(),
                HashMap::from([("knee".into(), 0.5), ("ankle".into(), -0.25)]),
            ),
            (
                "home".into(),
                HashMap::from([("shoulder".into(), 1.0), ("elbow".into(), 0.0)]),
            ),
        ]);

        let poses = configured_poses(&config);

        assert_eq!(
            poses
                .iter()
                .map(|pose| pose.name.as_str())
                .collect::<Vec<_>>(),
            vec!["home", "stand"]
        );
        assert_eq!(
            poses[0]
                .positions
                .iter()
                .map(|(joint, position)| (joint.as_str(), *position))
                .collect::<Vec<_>>(),
            vec![("elbow", 0.0), ("shoulder", 1.0)]
        );
        assert_eq!(
            poses[1]
                .positions
                .iter()
                .map(|(joint, position)| (joint.as_str(), *position))
                .collect::<Vec<_>>(),
            vec![("ankle", -0.25), ("knee", 0.5)]
        );
    }
}
