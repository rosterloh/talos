use talos_common::config::AgentConfig;
use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::TopicSub;

use super::RouterHandle;
use super::control::{configured_poses, execute_pose, set_joint_position};
use super::graph::{list_nodes, list_topics};
use crate::router::ClientId;
use crate::{GraphHandle, JointPublisher};

pub(super) async fn handle_request(
    request: &Request,
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
    graph_handle: &GraphHandle,
    router: &RouterHandle,
    client_id: ClientId,
) -> Option<Response> {
    match request {
        Request::Subscribe { topics } => {
            let topic_subs: Vec<TopicSub> = topics
                .iter()
                .filter_map(|t| {
                    config
                        .subscriptions
                        .iter()
                        .find(|s| &s.topic == t)
                        .map(|s| TopicSub {
                            topic: s.topic.clone(),
                            type_name: s.msg_type.clone(),
                        })
                })
                .collect();
            router
                .lock()
                .await
                .subscribe(client_id, topics.iter().cloned());
            Some(Response::Subscribed { topics: topic_subs })
        }
        Request::Unsubscribe { topics } => {
            router.lock().await.unsubscribe(client_id, topics);
            Some(Response::Unsubscribed {
                topics: topics.clone(),
            })
        }
        other => Some(handle_control_request(other, config, joint_publisher, graph_handle).await),
    }
}

pub(super) async fn handle_control_request(
    request: &Request,
    config: &AgentConfig,
    joint_publisher: &JointPublisher,
    graph_handle: &GraphHandle,
) -> Response {
    match request {
        Request::ListTopics => list_topics(config, graph_handle).await,
        Request::ListNodes => list_nodes(graph_handle).await,
        Request::ListPoses => Response::PoseList(configured_poses(config)),
        Request::SetJointPosition { joint, position } => {
            set_joint_position(config, joint_publisher, joint, *position).await
        }
        Request::ExecutePose { name } => execute_pose(config, joint_publisher, name).await,
        _ => Response::Error("unexpected request".into()),
    }
}
