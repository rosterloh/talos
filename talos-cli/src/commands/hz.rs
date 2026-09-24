use std::time::Duration;

use talos_common::protocol::messages::{Request, Response};
use talos_common::protocol::types::TopicStats;
use talos_common::session::ProtocolClient;
use tokio::time::Instant;

/// Print the agent's stats for `topic` once a second, until `duration` elapses
/// (forever if `None`).
pub async fn run<C: ProtocolClient>(
    client: &mut C,
    topic: String,
    duration: Option<u64>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = duration.map(|s| Instant::now() + Duration::from_secs(s));
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut warned = false;
    loop {
        interval.tick().await;
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return Ok(());
        }
        let stats = match client.request(Request::GetTopicStats).await? {
            Response::TopicStats(stats) => stats,
            Response::Error(e) => return Err(e.into()),
            _ => return Err("unexpected response".into()),
        };
        match stats.into_iter().find(|s| s.topic == topic) {
            Some(s) if json => println!("{}", serde_json::to_string(&s)?),
            Some(s) => println!("{}", format_stats(&s)),
            None if !warned => {
                eprintln!("no stats for '{topic}' yet (is it in the agent's [[subscriptions]]?)");
                warned = true;
            }
            None => {}
        }
    }
}

fn format_stats(s: &TopicStats) -> String {
    let latency = s
        .latency_ms
        .map_or("-".to_string(), |ms| format!("{ms:.1} ms"));
    format!(
        "rate: {:.2} Hz  bandwidth: {:.1} KB/s  latency: {latency}",
        s.rate_hz,
        s.bandwidth_bps / 1024.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_stats_shows_missing_latency_as_dash() {
        let s = TopicStats {
            topic: "/scan".into(),
            rate_hz: 10.0,
            bandwidth_bps: 2048.0,
            latency_ms: None,
        };
        assert_eq!(
            format_stats(&s),
            "rate: 10.00 Hz  bandwidth: 2.0 KB/s  latency: -"
        );
    }
}
