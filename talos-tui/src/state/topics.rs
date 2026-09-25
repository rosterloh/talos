use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use talos_common::protocol::messages::Request;
use talos_common::protocol::types::{DynValue, TopicInfo, TopicStats, TopicSub};

use super::AppState;
use super::filter::{clamp_selection, matches_filter};

#[derive(Debug, Clone)]
pub struct TopicData {
    pub info: TopicInfo,
    pub latest: Option<DynValue>,
    pub last_received: Option<Instant>,
    pub msg_count: u64,
    pub subscription: TopicSubscriptionState,
    pub subscription_error: Option<String>,
    /// Latest agent-side stats and when they arrived.
    pub stats: Option<(TopicStats, Instant)>,
    /// Agent-reported rate, one sample per stats poll, oldest first.
    pub rate_history: VecDeque<u64>,
}

/// Rate samples kept for the sparkline (one per second).
pub const RATE_HISTORY_LEN: usize = 60;

/// Agent stats older than this (e.g. after a disconnect) are ignored.
const STATS_STALE_AFTER: Duration = Duration::from_secs(3);

impl TopicData {
    /// Agent-side stats if they are current.
    pub fn current_stats(&self, now: Instant) -> Option<&TopicStats> {
        self.stats
            .as_ref()
            .filter(|(_, at)| now.duration_since(*at) < STATS_STALE_AFTER)
            .map(|(stats, _)| stats)
    }

    fn placeholder(name: &str) -> Self {
        Self {
            info: TopicInfo {
                name: name.to_string(),
                type_name: String::new(),
                publisher_count: 0,
                subscriber_count: 0,
            },
            latest: None,
            last_received: None,
            msg_count: 0,
            subscription: TopicSubscriptionState::Unsubscribed,
            subscription_error: None,
            stats: None,
            rate_history: VecDeque::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopicSubscriptionState {
    Subscribed,
    Unsubscribed,
    PendingSubscribe,
    PendingUnsubscribe,
    Error,
}

impl TopicSubscriptionState {
    pub fn label(self) -> &'static str {
        match self {
            TopicSubscriptionState::Subscribed => "subscribed",
            TopicSubscriptionState::Unsubscribed => "unsubscribed",
            TopicSubscriptionState::PendingSubscribe => "pending subscribe",
            TopicSubscriptionState::PendingUnsubscribe => "pending unsubscribe",
            TopicSubscriptionState::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PendingTopicSubscriptionToggle {
    pub(crate) request: Request,
    topic: String,
    previous_subscription: TopicSubscriptionState,
    previous_subscription_error: Option<String>,
}

impl AppState {
    pub(crate) fn handle_topic_stats(&mut self, stats: Vec<TopicStats>) {
        let now = Instant::now();
        for stat in stats {
            if let Some(topic) = self.topics.get_mut(&stat.topic) {
                if topic.rate_history.len() == RATE_HISTORY_LEN {
                    topic.rate_history.pop_front();
                }
                topic.rate_history.push_back(stat.rate_hz.round() as u64);
                topic.stats = Some((stat, now));
            }
        }
    }

    pub(crate) fn handle_topic_list(&mut self, topics: Vec<TopicInfo>) {
        let auto_subscribe_all = !self.subscriptions_customized;
        let mut current_topics = std::mem::take(&mut self.topics);
        let mut next_topics = HashMap::with_capacity(topics.len());
        let mut next_topic_names = Vec::with_capacity(topics.len());
        // Missing topics are intentionally dropped from the visible
        // catalog and cached samples until the agent advertises them again.

        for info in topics {
            let name = info.name.clone();
            next_topic_names.push(name.clone());

            let should_be_subscribed =
                auto_subscribe_all || self.desired_subscriptions.contains(&name);
            // `ListTopics` is re-polled during a connection, so existing
            // topics keep their subscription state and cached samples; new
            // ones start unsubscribed until a subscribe ack or live data
            // arrives. On reconnect, every desired topic is marked pending
            // before it is subscribed again.
            let mut topic = current_topics
                .remove(&name)
                .unwrap_or_else(|| TopicData::placeholder(&name));

            topic.info = info;
            if topic.subscription == TopicSubscriptionState::Error && !should_be_subscribed {
                topic.subscription = TopicSubscriptionState::Unsubscribed;
                topic.subscription_error = None;
            }

            if auto_subscribe_all {
                self.desired_subscriptions.insert(name.clone());
            }
            next_topics.insert(name, topic);
        }

        if auto_subscribe_all {
            self.desired_subscriptions = next_topic_names.iter().cloned().collect();
        }
        self.topics = next_topics;
        self.tree_expanded.retain(|path, _| {
            self.topics.keys().any(|topic| {
                path == topic
                    || path.starts_with(&format!("{topic}."))
                    || path.starts_with(&format!("{topic}["))
            })
        });
        self.tree_selection
            .retain(|topic, _| self.topics.contains_key(topic));
        self.replace_topic_names(next_topic_names);
    }

    pub(crate) fn handle_topic_data(&mut self, topic: String, type_name: String, data: DynValue) {
        if !self.subscriptions_customized {
            self.desired_subscriptions.insert(topic.clone());
        }
        let should_be_subscribed = self.desired_subscriptions.contains(&topic);
        let keep_pending_unsubscribe_data = self
            .topics
            .get(&topic)
            .is_some_and(|entry| entry.subscription == TopicSubscriptionState::PendingUnsubscribe);
        if !should_be_subscribed && !keep_pending_unsubscribe_data {
            return;
        }

        let now = Instant::now();
        let entry = self
            .topics
            .entry(topic.clone())
            .or_insert_with(|| TopicData {
                info: TopicInfo {
                    name: topic.clone(),
                    type_name: type_name.clone(),
                    publisher_count: 0,
                    subscriber_count: 0,
                },
                latest: None,
                last_received: None,
                msg_count: 0,
                subscription: if should_be_subscribed {
                    TopicSubscriptionState::Subscribed
                } else {
                    TopicSubscriptionState::Unsubscribed
                },
                subscription_error: None,
                stats: None,
                rate_history: VecDeque::new(),
            });

        entry.latest = Some(data.clone());
        entry.last_received = Some(now);
        entry.msg_count += 1;
        if should_be_subscribed
            && !matches!(
                entry.subscription,
                TopicSubscriptionState::PendingUnsubscribe
            )
        {
            entry.subscription = TopicSubscriptionState::Subscribed;
            entry.subscription_error = None;
        }

        self.ensure_topic_name(&topic);

        // Extract log entries from /rosout
        if topic == "/rosout" {
            self.push_log_entry_from_data(&data);
        }

        // Update joint data from /joint_states
        if topic == "/joint_states" {
            self.update_joints_from_data(&data);
        }

        // Parse URDF from /robot_description
        if topic == "/robot_description"
            && let DynValue::String(urdf_xml) = &data
        {
            self.update_joints_from_urdf(urdf_xml);
        }
    }

    pub(crate) fn handle_subscribed_topics(&mut self, topics: Vec<TopicSub>) {
        for sub in topics {
            let topic_name = sub.topic.clone();
            if !self.desired_subscriptions.contains(&topic_name) {
                continue;
            }
            let entry = self
                .topics
                .entry(topic_name.clone())
                // Defensive fallback: the normal path sees TopicList first.
                .or_insert_with(|| TopicData {
                    info: TopicInfo {
                        name: topic_name.clone(),
                        type_name: sub.type_name.clone(),
                        publisher_count: 0,
                        subscriber_count: 0,
                    },
                    latest: None,
                    last_received: None,
                    msg_count: 0,
                    subscription: TopicSubscriptionState::Unsubscribed,
                    subscription_error: None,
                    stats: None,
                    rate_history: VecDeque::new(),
                });
            entry.info.type_name = sub.type_name;
            entry.subscription = TopicSubscriptionState::Subscribed;
            entry.subscription_error = None;
            self.ensure_topic_name(&topic_name);
        }
    }

    pub(crate) fn handle_unsubscribed_topics(&mut self, topics: Vec<String>) {
        for topic_name in topics {
            if self.desired_subscriptions.contains(&topic_name) {
                continue;
            }
            let entry = self
                .topics
                .entry(topic_name.clone())
                .or_insert_with(|| TopicData::placeholder(&topic_name));
            entry.subscription = TopicSubscriptionState::Unsubscribed;
            entry.subscription_error = None;
        }
    }

    /// Returns the desired topic set for the next connection, limited to topics
    /// that still exist in the latest server-advertised catalog.
    pub fn desired_topics_for_connection(&self) -> Vec<String> {
        self.topic_names
            .iter()
            .filter(|name| {
                self.desired_subscriptions.contains(*name) && self.topics.contains_key(*name)
            })
            .cloned()
            .collect()
    }

    /// Desired topics not yet subscribed or in flight, e.g. topics that
    /// appeared in a refreshed topic list. Errored topics wait for reconnect.
    pub fn desired_topics_to_subscribe(&self) -> Vec<String> {
        self.desired_topics_for_connection()
            .into_iter()
            .filter(|name| self.topics[name].subscription == TopicSubscriptionState::Unsubscribed)
            .collect()
    }

    /// Optimistically updates desired subscription intent so a failed manual
    /// toggle is retried automatically after reconnect.
    pub fn toggle_selected_topic_subscription(&mut self) -> Option<Request> {
        let topic = self.selected_topic_name()?;
        self.subscriptions_customized = true;

        if self.desired_subscriptions.remove(&topic) {
            self.set_topic_subscription_state(&topic, TopicSubscriptionState::PendingUnsubscribe);
            Some(Request::Unsubscribe {
                topics: vec![topic],
            })
        } else {
            self.desired_subscriptions.insert(topic.clone());
            self.set_topic_subscription_state(&topic, TopicSubscriptionState::PendingSubscribe);
            Some(Request::Subscribe {
                topics: vec![topic],
            })
        }
    }

    pub(crate) fn prepare_selected_topic_subscription_toggle(
        &mut self,
    ) -> Option<PendingTopicSubscriptionToggle> {
        let topic = self.selected_topic_name()?;
        let previous_subscription = self
            .topics
            .get(&topic)
            .map(|entry| entry.subscription)
            .unwrap_or(TopicSubscriptionState::Unsubscribed);
        let previous_subscription_error = self
            .topics
            .get(&topic)
            .and_then(|entry| entry.subscription_error.clone());
        let request = self.toggle_selected_topic_subscription()?;

        Some(PendingTopicSubscriptionToggle {
            request,
            topic,
            previous_subscription,
            previous_subscription_error,
        })
    }

    pub(crate) fn revert_topic_subscription_toggle(
        &mut self,
        toggle: PendingTopicSubscriptionToggle,
    ) {
        match &toggle.request {
            Request::Subscribe { topics } => {
                for topic_name in topics {
                    self.desired_subscriptions.remove(topic_name);
                }
            }
            Request::Unsubscribe { topics } => {
                for topic_name in topics {
                    self.desired_subscriptions.insert(topic_name.clone());
                }
            }
            _ => return,
        }

        let entry = self
            .topics
            .entry(toggle.topic.clone())
            .or_insert_with(|| TopicData::placeholder(&toggle.topic));
        entry.subscription = toggle.previous_subscription;
        entry.subscription_error = toggle.previous_subscription_error;
    }

    pub fn mark_topics_pending_subscribe(&mut self, topics: &[String]) {
        self.set_topics_subscription_state(topics, TopicSubscriptionState::PendingSubscribe);
    }

    pub fn mark_topics_pending_unsubscribe(&mut self, topics: &[String]) {
        self.set_topics_subscription_state(topics, TopicSubscriptionState::PendingUnsubscribe);
    }

    pub fn mark_subscription_error(&mut self, topics: &[String], error: &str) {
        for topic_name in topics {
            let entry = self
                .topics
                .entry(topic_name.clone())
                .or_insert_with(|| TopicData::placeholder(topic_name));
            entry.subscription = TopicSubscriptionState::Error;
            entry.subscription_error = Some(error.to_string());
        }
    }

    fn set_topics_subscription_state(&mut self, topics: &[String], state: TopicSubscriptionState) {
        for topic_name in topics {
            self.set_topic_subscription_state(topic_name, state);
        }
    }

    fn set_topic_subscription_state(&mut self, topic_name: &str, state: TopicSubscriptionState) {
        let entry = self
            .topics
            .entry(topic_name.to_string())
            .or_insert_with(|| TopicData::placeholder(topic_name));
        entry.subscription = state;
        if state != TopicSubscriptionState::Error {
            entry.subscription_error = None;
        }
    }

    pub fn filtered_topic_names(&self) -> Vec<&String> {
        self.topic_names
            .iter()
            .filter(|name| matches_filter(name, &self.topic_filter))
            .collect()
    }

    /// Selected topic; `topic_selected` indexes the filtered list.
    pub(crate) fn selected_topic_name(&self) -> Option<String> {
        self.filtered_topic_names()
            .get(self.topic_selected)
            .map(|name| name.to_string())
    }

    fn ensure_topic_name(&mut self, topic_name: &str) {
        if self.topic_names.iter().any(|name| name == topic_name) {
            return;
        }

        let selected_topic = self.selected_topic_name();
        self.topic_names.push(topic_name.to_string());
        self.topic_names.sort();
        self.restore_topic_selection(selected_topic.as_deref());
    }

    fn replace_topic_names(&mut self, topic_names: Vec<String>) {
        let selected_topic = self.selected_topic_name();
        self.topic_names = topic_names;
        self.topic_names.sort();
        self.topic_names.dedup();
        self.restore_topic_selection(selected_topic.as_deref());
    }

    fn restore_topic_selection(&mut self, selected_topic: Option<&str>) {
        let visible = self.filtered_topic_names();
        if let Some(selected_topic) = selected_topic
            && let Some(index) = visible
                .iter()
                .position(|topic_name| *topic_name == selected_topic)
        {
            self.topic_selected = index;
            return;
        }

        let len = visible.len();
        clamp_selection(&mut self.topic_selected, len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_queried_for_selected_topic_on_topics_tab() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![TopicInfo {
            name: "/scan".into(),
            type_name: "sensor_msgs/msg/LaserScan".into(),
            publisher_count: 1,
            subscriber_count: 0,
        }]));
        assert_eq!(state.endpoint_query_topic().as_deref(), Some("/scan"));

        state.active_tab = crate::state::Tab::Nodes;
        assert_eq!(state.endpoint_query_topic(), None);
    }

    #[test]
    fn endpoint_query_error_clears_endpoints_without_touching_params() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicEndpoints {
            topic: "/scan".into(),
            publishers: vec![],
            subscribers: vec![],
        });
        state.param_awaiting_reply = true;
        state.handle_endpoints_response(Response::Error("graph query failed".into()));
        assert!(state.topic_endpoints.is_none());
        // Not mistaken for the reply to a pending parameter request.
        assert!(state.param_awaiting_reply);
    }

    #[test]
    fn agent_stats_expire_when_they_stop_arriving() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![TopicInfo {
            name: "/scan".into(),
            type_name: "sensor_msgs/msg/LaserScan".into(),
            publisher_count: 1,
            subscriber_count: 0,
        }]));
        state.handle_topic_stats(vec![TopicStats {
            topic: "/scan".into(),
            rate_hz: 9.6,
            bandwidth_bps: 0.0,
            latency_ms: None,
        }]);
        let topic = &state.topics["/scan"];
        let (_, at) = topic.stats.as_ref().unwrap();
        assert_eq!(topic.current_stats(*at).map(|s| s.rate_hz), Some(9.6));
        assert_eq!(topic.rate_history, [10]);
        // After a disconnect the stats stop updating and are no longer shown.
        assert!(topic.current_stats(*at + Duration::from_secs(5)).is_none());
    }

    use crate::state::AppState;
    use talos_common::protocol::messages::Response;
    use talos_common::protocol::types::{DynValue, Timestamp, TopicSub};

    fn topic(name: &str, type_name: &str) -> TopicInfo {
        TopicInfo {
            name: name.into(),
            type_name: type_name.into(),
            publisher_count: 1,
            subscriber_count: 0,
        }
    }

    #[test]
    fn topic_list_defaults_reconnect_intent_to_all_topics_until_user_customizes() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));

        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string(), "/rosout".to_string()]
        );
    }

    #[test]
    fn topic_list_does_not_speculatively_mark_desired_topics_subscribed() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::Unsubscribed
        );
    }

    #[test]
    fn toggle_selected_topic_preserves_manual_subscription_choice() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));
        state.topic_selected = 1;

        let request = state.toggle_selected_topic_subscription();

        assert_eq!(
            request,
            Some(Request::Unsubscribe {
                topics: vec!["/rosout".to_string()]
            })
        );
        assert!(state.subscriptions_customized);
        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string()]
        );
        assert_eq!(
            state.topics["/rosout"].subscription,
            TopicSubscriptionState::PendingUnsubscribe
        );
    }

    #[test]
    fn ignored_subscribed_ack_keeps_selected_topic_stable() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/beta", "std_msgs/msg/String"),
            topic("/delta", "std_msgs/msg/String"),
        ]));
        state.topic_selected = 1;

        state.handle_response(Response::Subscribed {
            topics: vec![TopicSub {
                topic: "/alpha".into(),
                type_name: "std_msgs/msg/String".into(),
            }],
        });

        assert_eq!(
            state.topic_names,
            vec!["/beta".to_string(), "/delta".to_string()]
        );
        assert_eq!(state.topic_names[state.topic_selected], "/delta");
    }

    #[test]
    fn latest_topic_list_removes_missing_topics_from_reconnect_intent() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));
        state.topic_selected = 1;

        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(state.topic_names, vec!["/camera".to_string()]);
        assert_eq!(state.topic_selected, 0);
        assert_eq!(
            state.desired_topics_for_connection(),
            vec!["/camera".to_string()]
        );
    }

    #[test]
    fn refreshed_topic_list_keeps_state_and_selection_by_name() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/beta", "std_msgs/msg/String"),
            topic("/delta", "std_msgs/msg/String"),
        ]));
        state.handle_response(Response::Subscribed {
            topics: vec![TopicSub {
                topic: "/delta".into(),
                type_name: "std_msgs/msg/String".into(),
            }],
        });
        state.topic_selected = 1;
        assert_eq!(state.desired_topics_to_subscribe(), ["/beta"]);

        // `/alpha` appears before the selection, `/beta` vanishes.
        state.handle_response(Response::TopicList(vec![
            topic("/alpha", "std_msgs/msg/String"),
            topic("/delta", "std_msgs/msg/String"),
            topic("/gamma", "std_msgs/msg/String"),
        ]));

        assert_eq!(state.topic_names, ["/alpha", "/delta", "/gamma"]);
        assert_eq!(state.topic_names[state.topic_selected], "/delta");
        assert!(!state.topics.contains_key("/beta"));
        assert_eq!(
            state.topics["/delta"].subscription,
            TopicSubscriptionState::Subscribed
        );
        // Only the newly advertised topics still need a subscribe.
        assert_eq!(state.desired_topics_to_subscribe(), ["/alpha", "/gamma"]);

        // The selected topic vanishing clamps the selection.
        state.topic_selected = 2;
        state.handle_response(Response::TopicList(vec![topic(
            "/alpha",
            "std_msgs/msg/String",
        )]));
        assert_eq!(state.topic_selected, 0);
    }

    #[test]
    fn new_topics_after_customization_default_to_unsubscribed() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));
        state.topic_selected = 0;

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/lidar", "sensor_msgs/msg/LaserScan"),
        ]));

        assert!(!state.desired_subscriptions.contains("/lidar"));
        assert_eq!(
            state.topics["/lidar"].subscription,
            TopicSubscriptionState::Unsubscribed
        );
    }

    #[test]
    fn topic_list_clears_stale_unsubscribe_errors_once_desired_state_is_off() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));
        state.topic_selected = 1;

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/rosout".to_string()]
            })
        );
        state.mark_subscription_error(&["/rosout".to_string()], "boom");

        state.handle_response(Response::TopicList(vec![
            topic("/camera", "sensor_msgs/msg/Image"),
            topic("/rosout", "rcl_interfaces/msg/Log"),
        ]));

        assert_eq!(
            state.topics["/rosout"].subscription,
            TopicSubscriptionState::Unsubscribed
        );
        assert_eq!(state.topics["/rosout"].subscription_error, None);
    }

    #[test]
    fn stale_subscribed_ack_does_not_override_pending_unsubscribe() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::Subscribed {
            topics: vec![TopicSub {
                topic: "/camera".into(),
                type_name: "sensor_msgs/msg/Image".into(),
            }],
        });

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::PendingUnsubscribe
        );
        assert!(!state.desired_subscriptions.contains("/camera"));
    }

    #[test]
    fn stale_unsubscribed_ack_does_not_override_pending_subscribe() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );
        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Subscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::Unsubscribed {
            topics: vec!["/camera".to_string()],
        });

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::PendingSubscribe
        );
        assert!(state.desired_subscriptions.contains("/camera"));
    }

    #[test]
    fn topic_data_clears_error_once_desired_subscription_is_healthy() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));
        state.mark_subscription_error(&["/camera".to_string()], "boom");

        state.handle_response(Response::TopicData {
            topic: "/camera".into(),
            type_name: "sensor_msgs/msg/Image".into(),
            stamp: Timestamp { sec: 0, nanosec: 0 },
            data: DynValue::String("frame".into()),
        });

        assert_eq!(
            state.topics["/camera"].subscription,
            TopicSubscriptionState::Subscribed
        );
        assert_eq!(state.topics["/camera"].subscription_error, None);
    }

    #[test]
    fn topic_data_while_unsubscribe_is_pending_still_updates_latest_sample() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );

        state.handle_response(Response::TopicData {
            topic: "/camera".into(),
            type_name: "sensor_msgs/msg/Image".into(),
            stamp: Timestamp { sec: 0, nanosec: 0 },
            data: DynValue::String("during-pending".into()),
        });

        let topic = &state.topics["/camera"];
        assert_eq!(
            topic.subscription,
            TopicSubscriptionState::PendingUnsubscribe
        );
        assert_eq!(
            topic.latest,
            Some(DynValue::String("during-pending".into()))
        );
        assert_eq!(topic.msg_count, 1);
    }

    #[test]
    fn topic_data_is_ignored_once_topic_is_unsubscribed() {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![topic(
            "/camera",
            "sensor_msgs/msg/Image",
        )]));

        {
            let topic = state.topics.get_mut("/camera").unwrap();
            topic.latest = Some(DynValue::String("before".into()));
            topic.last_received = Some(Instant::now());
            topic.msg_count = 41;
        }

        assert_eq!(
            state.toggle_selected_topic_subscription(),
            Some(Request::Unsubscribe {
                topics: vec!["/camera".to_string()]
            })
        );
        state.handle_response(Response::Unsubscribed {
            topics: vec!["/camera".to_string()],
        });

        let before_last_received = state.topics["/camera"].last_received;
        state.handle_response(Response::TopicData {
            topic: "/camera".into(),
            type_name: "sensor_msgs/msg/Image".into(),
            stamp: Timestamp { sec: 1, nanosec: 0 },
            data: DynValue::String("after".into()),
        });

        let topic = &state.topics["/camera"];
        assert_eq!(topic.subscription, TopicSubscriptionState::Unsubscribed);
        assert_eq!(topic.latest, Some(DynValue::String("before".into())));
        assert_eq!(topic.last_received, before_last_received);
        assert_eq!(topic.msg_count, 41);
    }
}
