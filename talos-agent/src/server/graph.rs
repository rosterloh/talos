use std::collections::HashMap;

use talos_common::config::AgentConfig;
use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{NodeInfo, TopicInfo};
use tracing::warn;

use crate::GraphHandle;

pub(super) async fn list_topics(config: &AgentConfig, graph_handle: &GraphHandle) -> Response {
    let graph_node = graph_handle.lock().await.clone();

    if let Some(node) = graph_node {
        match node.get_topic_names_and_types() {
            Ok(names_and_types) => {
                let mut topics: Vec<TopicInfo> = names_and_types
                    .into_iter()
                    .map(|(name, types)| {
                        let publisher_count = node.count_publishers(&name).unwrap_or(0);
                        let subscriber_count = node.count_subscriptions(&name).unwrap_or(0);
                        TopicInfo {
                            name,
                            type_name: format_type_names(types),
                            publisher_count,
                            subscriber_count,
                        }
                    })
                    .collect();
                topics.sort_by(|a, b| a.name.cmp(&b.name));
                return Response::TopicList(topics);
            }
            Err(e) => {
                warn!("failed to query ROS graph topics, using configured subscriptions: {e}");
            }
        }
    }

    Response::TopicList(configured_topics(config))
}

pub(super) async fn list_nodes(graph_handle: &GraphHandle) -> Response {
    let graph_node = graph_handle.lock().await.clone();
    let Some(node) = graph_node else {
        return Response::NodeList(vec![]);
    };

    let names = match node.get_node_names() {
        Ok(names) => names,
        Err(e) => return Response::Error(format!("failed to query ROS graph nodes: {e}")),
    };

    let mut nodes: Vec<NodeInfo> = names
        .into_iter()
        .map(|n| {
            let publishers = graph_names_for_node(
                node.get_publisher_names_and_types_by_node(&n.name, &n.namespace),
            );
            let subscribers = graph_names_for_node(
                node.get_subscription_names_and_types_by_node(&n.name, &n.namespace),
            );
            let services = graph_names_for_node(
                node.get_service_names_and_types_by_node(&n.name, &n.namespace),
            );

            NodeInfo {
                name: n.name,
                namespace: n.namespace,
                publishers,
                subscribers,
                services,
            }
        })
        .collect();

    nodes.sort_by(|a, b| {
        a.namespace
            .cmp(&b.namespace)
            .then_with(|| a.name.cmp(&b.name))
    });
    Response::NodeList(nodes)
}

pub(super) fn configured_topics(config: &AgentConfig) -> Vec<TopicInfo> {
    let mut topics: Vec<TopicInfo> = config
        .subscriptions
        .iter()
        .map(|s| TopicInfo {
            name: s.topic.clone(),
            type_name: s.msg_type.clone(),
            publisher_count: 0,
            subscriber_count: 0,
        })
        .collect();
    topics.sort_by(|a, b| a.name.cmp(&b.name));
    topics
}

pub(super) fn graph_names_for_node(
    result: Result<HashMap<String, Vec<String>>, rclrs::RclrsError>,
) -> Vec<String> {
    match result {
        Ok(names_and_types) => {
            let mut names: Vec<String> = names_and_types.into_keys().collect();
            names.sort();
            names
        }
        Err(e) => {
            warn!("failed to query ROS graph node endpoints: {e}");
            vec![]
        }
    }
}

pub(super) fn format_type_names(types: Vec<String>) -> String {
    if types.is_empty() {
        String::new()
    } else {
        types.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use talos_common::config::{AgentConfig, QosProfile, SubscriptionConfig};

    use super::*;

    #[test]
    fn configured_topics_are_sorted_by_name() {
        let mut config = AgentConfig::default();
        config.subscriptions = vec![
            SubscriptionConfig {
                topic: "/z".into(),
                msg_type: "z_msgs/msg/Z".into(),
                qos: QosProfile::Default,
            },
            SubscriptionConfig {
                topic: "/a".into(),
                msg_type: "a_msgs/msg/A".into(),
                qos: QosProfile::SensorData,
            },
        ];

        let topics = configured_topics(&config);

        assert_eq!(
            topics
                .iter()
                .map(|topic| topic.name.as_str())
                .collect::<Vec<_>>(),
            vec!["/a", "/z"]
        );
        assert_eq!(topics[0].type_name, "a_msgs/msg/A");
        assert_eq!(topics[0].publisher_count, 0);
        assert_eq!(topics[0].subscriber_count, 0);
    }

    #[test]
    fn format_type_names_joins_multiple_types() {
        assert_eq!(
            format_type_names(vec!["type/A".into(), "type/B".into()]),
            "type/A, type/B"
        );
        assert_eq!(format_type_names(vec![]), "");
    }
}
