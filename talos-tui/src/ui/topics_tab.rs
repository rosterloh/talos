use super::style;
use crate::state::{AppState, Pane, TopicEndpoints, TopicSubscriptionState};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Cell, List, ListItem, Paragraph, Row, Sparkline, Table},
};
use talos_common::protocol::types::{DynValue, EndpointInfo};

pub fn draw(f: &mut Frame, state: &AppState, area: Rect) {
    let [left, right] = super::panes(area, state.active_pane);
    let focused = state.active_pane == Pane::Left;
    let now = std::time::Instant::now();
    let rows: Vec<_> = state
        .filtered_topic_names()
        .iter()
        .map(|name| {
            let topic = &state.topics[*name];
            let rate = topic
                .current_stats(now)
                .map(|s| format!("{:.0}Hz", s.rate_hz))
                .unwrap_or_else(|| "—".into());
            let (badge, badge_style) = subscription_badge(topic);
            Row::new(vec![
                Cell::from(badge).style(badge_style),
                Cell::from(name.to_string()),
                Cell::from(rate).style(style::SECONDARY),
            ])
        })
        .collect();
    super::table(
        f,
        state,
        "topics",
        Table::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Min(1),
                Constraint::Length(6),
            ],
        )
        .header(Row::new(["Sub", "Topic", "Rate"]).style(style::SECONDARY))
        .block(style::pane(
            super::filter_title("TOPICS".into(), &state.topic_filter),
            focused,
        )),
        left,
        state.topic_selected,
        focused,
    );
    if right.is_empty() {
        return;
    }
    let Some(topic) = state
        .selected_topic_name()
        .and_then(|name| state.topics.get(&name))
    else {
        f.render_widget(
            Paragraph::new("No topic selected").block(style::pane("PAYLOAD", !focused)),
            right,
        );
        return;
    };
    let mut summary = vec![Line::from(vec![
        Span::raw("Subscription: "),
        Span::styled(topic.subscription.label(), subscription_badge(topic).1),
    ])];
    if let Some(error) = &topic.subscription_error {
        summary.push(Line::styled(error.as_str(), style::ERROR));
    }
    if right.height >= 12 {
        summary.push(Line::styled(
            topic.info.type_name.as_str(),
            style::SECONDARY,
        ));
    }
    if let Some(stats) = topic.current_stats(now) {
        let latency = stats
            .latency_ms
            .map_or_else(|| "-".into(), |ms| format!("{ms:.1} ms"));
        summary.push(Line::from(format!(
            "Agent: {:.1} Hz  {}  latency {latency}",
            stats.rate_hz,
            format_bandwidth(stats.bandwidth_bps)
        )));
    }
    let show_spark = right.height >= 16 && topic.rate_history.len() > 1 && !state.show_endpoints;
    let [summary_area, body, spark] = Layout::vertical([
        Constraint::Length((summary.len() as u16 + 1).min(right.height.saturating_sub(3))),
        Constraint::Min(0),
        Constraint::Length(if show_spark { 2 } else { 0 }),
    ])
    .areas(right);
    f.render_widget(
        Paragraph::new(summary).block(style::pane(format!("TOPIC {}", topic.info.name), false)),
        summary_area,
    );
    if state.show_endpoints {
        let mut lines = Vec::new();
        if let Some(endpoints) = state
            .topic_endpoints
            .as_ref()
            .filter(|e| e.topic == topic.info.name)
        {
            push_endpoint_lines(&mut lines, endpoints);
        } else {
            lines.push(Line::from("Waiting for endpoint information"));
        }
        super::scroll_text(
            f,
            state,
            &format!("qos:{}", topic.info.name),
            "ENDPOINTS / QoS · i payload",
            lines,
            body,
            !focused,
        );
    } else {
        let rows = state.tree_rows();
        let selected = state.tree_index(&rows);
        let items: Vec<_> = rows
            .iter()
            .map(|row| {
                let arrow = if row.branch {
                    if row.expanded { "v" } else { ">" }
                } else {
                    " "
                };
                ListItem::new(format!(
                    "{}{} {}: {}",
                    "  ".repeat(row.depth),
                    arrow,
                    row.label,
                    format_value(row.value)
                ))
            })
            .collect();
        let list = if rows.is_empty() {
            List::new(vec![ListItem::new("No data received yet")])
        } else {
            List::new(items)
        };
        super::list(
            f,
            state,
            &format!("tree:{}", topic.info.name),
            list.block(style::pane("PAYLOAD · i endpoints/QoS", !focused)),
            body,
            selected,
            !focused,
        );
    }
    if show_spark {
        let history: Vec<_> = topic.rate_history.iter().copied().collect();
        f.render_widget(
            Sparkline::default()
                .data(&history)
                .style(style::SECONDARY)
                .block(style::pane(format!("rate, last {}s", history.len()), false)),
            spark,
        );
    }
}

/// Publisher and subscriber QoS, with subscribers that can't be matched to a
/// publisher flagged (the usual reason a topic delivers no data).
fn push_endpoint_lines(lines: &mut Vec<Line<'static>>, endpoints: &TopicEndpoints) {
    let label = |e: &EndpointInfo| {
        if e.node_namespace.is_empty() || e.node_namespace == "/" {
            format!("/{}", e.node_name)
        } else {
            format!("{}/{}", e.node_namespace.trim_end_matches('/'), e.node_name)
        }
    };
    let dim = style::SECONDARY;

    lines.push(Line::from(Span::styled(
        format!("Publishers ({}):", endpoints.publishers.len()),
        dim,
    )));
    for publisher in &endpoints.publishers {
        lines.push(Line::from(vec![
            Span::raw(format!("  {}  ", label(publisher))),
            Span::styled(publisher.qos.to_string(), dim),
        ]));
    }
    lines.push(Line::from(Span::styled(
        format!("Subscribers ({}):", endpoints.subscribers.len()),
        dim,
    )));
    for subscriber in &endpoints.subscribers {
        lines.push(Line::from(vec![
            Span::raw(format!("  {}  ", label(subscriber))),
            Span::styled(subscriber.qos.to_string(), dim),
        ]));
        for publisher in &endpoints.publishers {
            if let Some(reason) = publisher.qos.incompatibility_with(&subscriber.qos) {
                lines.push(Line::from(Span::styled(
                    format!("    ⚠ no match with {}: {reason}", label(publisher)),
                    style::ERROR,
                )));
            }
        }
    }
}

fn format_bandwidth(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{bytes_per_sec:.0} B/s")
    }
}

fn format_value(value: &DynValue) -> String {
    match value {
        DynValue::Bool(b) => b.to_string(),
        DynValue::I8(v) => v.to_string(),
        DynValue::I16(v) => v.to_string(),
        DynValue::I32(v) => v.to_string(),
        DynValue::I64(v) => v.to_string(),
        DynValue::U8(v) => v.to_string(),
        DynValue::U16(v) => v.to_string(),
        DynValue::U32(v) => v.to_string(),
        DynValue::U64(v) => v.to_string(),
        DynValue::F32(v) => format!("{v:.4}"),
        DynValue::F64(v) => format!("{v:.4}"),
        DynValue::String(s) => {
            if s.chars().count() > 80 {
                format!("{}...", s.chars().take(77).collect::<String>())
            } else {
                s.clone()
            }
        }
        DynValue::Bytes(b) => format!("[{} bytes]", b.len()),
        DynValue::Array(arr) => {
            if arr.len() <= 6 {
                let items: Vec<String> = arr.iter().map(format_value).collect();
                format!("[{}]", items.join(", "))
            } else {
                format!("[{} items]", arr.len())
            }
        }
        DynValue::Struct { type_name, .. } => format!("{{{type_name}}}"),
    }
}

fn subscription_badge(topic: &crate::state::TopicData) -> (&'static str, ratatui::style::Style) {
    match topic.subscription {
        TopicSubscriptionState::Subscribed => ("[ON ]", style::SUCCESS),
        TopicSubscriptionState::Unsubscribed => ("[OFF]", style::SECONDARY),
        TopicSubscriptionState::PendingSubscribe => ("[+..]", style::WARNING),
        TopicSubscriptionState::PendingUnsubscribe => ("[-..]", style::WARNING),
        TopicSubscriptionState::Error => ("[ERR]", style::ERROR),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use talos_common::protocol::messages::Response;
    use talos_common::protocol::types::{TopicInfo, TopicStats};

    use super::*;

    /// Draw the Topics tab into a test buffer and return it as text rows.
    pub(crate) fn render(state: &AppState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        terminal.draw(|f| draw(f, state, f.area())).unwrap();
        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .chunks(buffer.area.width as usize)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn state_with_topic(name: &str) -> AppState {
        let mut state = AppState::default();
        state.handle_response(Response::TopicList(vec![TopicInfo {
            name: name.into(),
            type_name: "sensor_msgs/msg/LaserScan".into(),
            publisher_count: 1,
            subscriber_count: 0,
        }]));
        state
    }

    #[test]
    fn agent_stats_render_in_list_and_detail() {
        let mut state = state_with_topic("/scan");
        for rate_hz in [9.6, 10.2] {
            state.handle_topic_stats(vec![TopicStats {
                topic: "/scan".into(),
                rate_hz,
                bandwidth_bps: 2048.0,
                latency_ms: Some(3.5),
            }]);
        }

        let screen = render(&state);
        assert!(
            screen
                .lines()
                .any(|line| line.contains("/scan") && line.contains("10Hz")),
            "{screen}"
        );
        assert!(
            screen.contains("Agent: 10.2 Hz  2.0 KB/s  latency 3.5 ms"),
            "{screen}"
        );
        assert!(screen.contains(" rate, last 2s "), "{screen}");
    }

    #[test]
    fn topic_without_stats_renders_placeholder_rate() {
        let screen = render(&state_with_topic("/scan"));
        assert!(
            screen
                .lines()
                .any(|line| line.contains("/scan") && line.contains('—')),
            "{screen}"
        );
        assert!(!screen.contains("Agent:"), "{screen}");
    }

    #[test]
    fn endpoints_render_with_incompatibility_warning() {
        use talos_common::protocol::types::{
            Durability, EndpointInfo, History, QosInfo, Reliability,
        };
        let endpoint = |node: &str, reliability| EndpointInfo {
            node_name: node.into(),
            node_namespace: "/".into(),
            topic_type: "sensor_msgs/msg/LaserScan".into(),
            qos: QosInfo {
                reliability,
                durability: Durability::Volatile,
                history: History::KeepLast { depth: 5 },
                deadline_ms: None,
            },
        };
        let mut state = state_with_topic("/scan");
        state.show_endpoints = true;
        state.handle_response(Response::TopicEndpoints {
            topic: "/scan".into(),
            publishers: vec![endpoint("lidar", Reliability::BestEffort)],
            subscribers: vec![endpoint("talos_agent", Reliability::Reliable)],
        });

        let screen = render(&state);
        assert!(screen.contains("Publishers (1):"), "{screen}");
        assert!(
            screen.contains("/lidar  best_effort volatile keep_last(5)"),
            "{screen}"
        );
        assert!(
            screen.contains("/talos_agent  reliable volatile keep_last(5)"),
            "{screen}"
        );
        assert!(
            screen.contains("⚠ no match with /lidar: best-effort publisher, reliable subscriber"),
            "{screen}"
        );
    }

    #[test]
    fn endpoints_for_another_topic_are_not_shown() {
        let mut state = state_with_topic("/scan");
        state.show_endpoints = true;
        state.handle_response(Response::TopicEndpoints {
            topic: "/other".into(),
            publishers: vec![],
            subscribers: vec![],
        });
        assert!(!render(&state).contains("Publishers"));
    }

    #[test]
    fn bandwidth_is_formatted_with_units() {
        assert_eq!(format_bandwidth(512.0), "512 B/s");
        assert_eq!(format_bandwidth(1536.0), "1.5 KB/s");
        assert_eq!(format_bandwidth(3.0 * 1024.0 * 1024.0), "3.0 MB/s");
    }
}
