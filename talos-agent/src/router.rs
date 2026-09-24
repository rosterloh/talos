//! Per-client subscription tracking and topic routing.
//!
//! [`TopicRouter`] replaces the `broadcast::channel` used in v0.1.  Instead of
//! sending every message to every client, it keeps a per-client subscription
//! set and routes `TopicData` responses only to clients that have subscribed to
//! the relevant topic.

use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use talos_common::protocol::messages::Response;
use talos_common::protocol::types::{DynValue, Timestamp, TopicStats};
use tokio::sync::mpsc;

pub type ClientId = u64;

/// Per-client backlog of undelivered topic frames. A client that falls further
/// behind loses new frames instead of growing agent memory without bound.
// ponytail: one shared queue per client, so a flooding topic can crowd out a
// slow one; use per-topic latest-value slots if that matters.
pub const CLIENT_QUEUE_CAPACITY: usize = 1024;

/// Manages per-client subscriptions and routes topic data.
///
/// Wrap in `Arc<tokio::sync::Mutex<TopicRouter>>` and share between the bridge
/// task (caller of [`route`]) and all client handler tasks.
pub struct TopicRouter {
    clients: HashMap<ClientId, ClientEntry>,
    next_id: ClientId,
    stats: HashMap<String, TopicCounter>,
    window_start: Instant,
}

/// Weight of the newest window in the smoothed stats.
const STATS_EMA_ALPHA: f64 = 0.3;

/// Per-topic counts for the current window, plus the smoothed values that
/// [`TopicRouter::tick_stats`] derives from them.
#[derive(Default)]
struct TopicCounter {
    msgs: u64,
    bytes: u64,
    latency_sum_ms: f64,
    latency_samples: u64,
    windows: u64,
    rate_hz: f64,
    bandwidth_bps: f64,
    latency_ms: Option<f64>,
}

struct ClientEntry {
    subscriptions: HashSet<String>,
    tx: mpsc::Sender<Response>,
}

impl TopicRouter {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            next_id: 0,
            stats: HashMap::new(),
            window_start: Instant::now(),
        }
    }

    /// Register a new client and return its ID and the channel it should receive
    /// data on.
    pub fn register(&mut self) -> (ClientId, mpsc::Receiver<Response>) {
        let (tx, rx) = mpsc::channel(CLIENT_QUEUE_CAPACITY);
        let id = self.next_id;
        self.next_id += 1;
        self.clients.insert(
            id,
            ClientEntry {
                subscriptions: HashSet::new(),
                tx,
            },
        );
        (id, rx)
    }

    /// Remove a client's subscription state.
    pub fn deregister(&mut self, id: ClientId) {
        self.clients.remove(&id);
    }

    /// Add topics to a client's subscription set.
    pub fn subscribe(&mut self, id: ClientId, topics: impl IntoIterator<Item = String>) {
        if let Some(entry) = self.clients.get_mut(&id) {
            for t in topics {
                entry.subscriptions.insert(t);
            }
        }
    }

    /// Remove topics from a client's subscription set.
    pub fn unsubscribe(&mut self, id: ClientId, topics: &[String]) {
        if let Some(entry) = self.clients.get_mut(&id) {
            for t in topics {
                entry.subscriptions.remove(t);
            }
        }
    }

    /// Route a `TopicData` response to all subscribed clients.
    ///
    /// Non-`TopicData` responses are silently ignored, as are frames for
    /// clients whose queue is full.
    pub fn route(&mut self, response: &Response) {
        if let Response::TopicData {
            topic, stamp, data, ..
        } = response
        {
            let counter = self.stats.entry(topic.clone()).or_default();
            counter.msgs += 1;
            counter.bytes += approx_payload_size(data) as u64;
            if let Some(latency) = latency_ms(stamp, SystemTime::now()) {
                counter.latency_sum_ms += latency;
                counter.latency_samples += 1;
            }

            for entry in self.clients.values() {
                if entry.subscriptions.contains(topic.as_str()) {
                    let _ = entry.tx.try_send(response.clone());
                }
            }
        }
    }
}

impl TopicRouter {
    /// Close the current stats window: turn its counts into rates, fold them
    /// into the smoothed values, and start a new window. Call about once a
    /// second.
    pub fn tick_stats(&mut self, now: Instant) {
        let elapsed = now
            .duration_since(self.window_start)
            .as_secs_f64()
            .max(1e-6);
        self.window_start = now;
        for c in self.stats.values_mut() {
            let rate = c.msgs as f64 / elapsed;
            let bandwidth = c.bytes as f64 / elapsed;
            if c.windows == 0 {
                c.rate_hz = rate;
                c.bandwidth_bps = bandwidth;
            } else {
                c.rate_hz += STATS_EMA_ALPHA * (rate - c.rate_hz);
                c.bandwidth_bps += STATS_EMA_ALPHA * (bandwidth - c.bandwidth_bps);
            }
            if c.latency_samples > 0 {
                let mean = c.latency_sum_ms / c.latency_samples as f64;
                c.latency_ms = Some(match c.latency_ms {
                    Some(prev) => prev + STATS_EMA_ALPHA * (mean - prev),
                    None => mean,
                });
            }
            c.windows += 1;
            c.msgs = 0;
            c.bytes = 0;
            c.latency_sum_ms = 0.0;
            c.latency_samples = 0;
        }
    }

    /// Smoothed stats for every topic seen so far, sorted by topic.
    pub fn topic_stats(&self) -> Vec<TopicStats> {
        let mut stats: Vec<TopicStats> = self
            .stats
            .iter()
            .filter(|(_, c)| c.windows > 0)
            .map(|(topic, c)| TopicStats {
                topic: topic.clone(),
                rate_hz: c.rate_hz,
                bandwidth_bps: c.bandwidth_bps,
                latency_ms: c.latency_ms,
            })
            .collect();
        stats.sort_by(|a, b| a.topic.cmp(&b.topic));
        stats
    }
}

/// Receive time minus the message stamp. Unstamped messages carry a zero
/// stamp and have no latency.
fn latency_ms(stamp: &Timestamp, now: SystemTime) -> Option<f64> {
    if stamp.sec == 0 && stamp.nanosec == 0 {
        return None;
    }
    let now = now.duration_since(UNIX_EPOCH).ok()?.as_secs_f64();
    let stamp = stamp.sec as f64 + stamp.nanosec as f64 * 1e-9;
    Some((now - stamp) * 1000.0)
}

/// Size of the message's data, roughly its CDR payload without length
/// prefixes or alignment padding.
fn approx_payload_size(value: &DynValue) -> usize {
    match value {
        DynValue::Bool(_) | DynValue::I8(_) | DynValue::U8(_) => 1,
        DynValue::I16(_) | DynValue::U16(_) => 2,
        DynValue::I32(_) | DynValue::U32(_) | DynValue::F32(_) => 4,
        DynValue::I64(_) | DynValue::U64(_) | DynValue::F64(_) => 8,
        DynValue::String(s) => s.len(),
        DynValue::Bytes(b) => b.len(),
        DynValue::Array(items) => items.iter().map(approx_payload_size).sum(),
        DynValue::Struct { fields, .. } => fields.iter().map(|(_, v)| approx_payload_size(v)).sum(),
    }
}

impl Default for TopicRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn data(topic: &str, stamp: Timestamp, data: DynValue) -> Response {
        Response::TopicData {
            topic: topic.into(),
            type_name: "t".into(),
            stamp,
            data,
        }
    }

    const NO_STAMP: Timestamp = Timestamp { sec: 0, nanosec: 0 };

    #[test]
    fn stats_count_rate_and_bandwidth_per_window() {
        let mut router = TopicRouter::new();
        let start = router.window_start;
        for _ in 0..20 {
            router.route(&data("/scan", NO_STAMP, DynValue::Bytes(vec![0; 100])));
        }
        router.tick_stats(start + Duration::from_secs(2));

        let stats = router.topic_stats();
        assert_eq!(stats.len(), 1);
        assert!((stats[0].rate_hz - 10.0).abs() < 1e-9);
        assert!((stats[0].bandwidth_bps - 1000.0).abs() < 1e-9);
        assert_eq!(stats[0].latency_ms, None);
    }

    #[test]
    fn stats_decay_when_topic_goes_quiet() {
        let mut router = TopicRouter::new();
        let start = router.window_start;
        for _ in 0..10 {
            router.route(&data("/t", NO_STAMP, DynValue::Bool(true)));
        }
        router.tick_stats(start + Duration::from_secs(1));
        router.tick_stats(start + Duration::from_secs(2));
        assert!((router.topic_stats()[0].rate_hz - 7.0).abs() < 1e-9);
    }

    #[test]
    fn stats_latency_uses_header_stamp() {
        let mut router = TopicRouter::new();
        let sent = SystemTime::now() - Duration::from_millis(250);
        let since_epoch = sent.duration_since(UNIX_EPOCH).unwrap();
        let stamp = Timestamp {
            sec: since_epoch.as_secs() as i32,
            nanosec: since_epoch.subsec_nanos(),
        };
        router.route(&data("/imu", stamp, DynValue::Bool(true)));
        router.tick_stats(Instant::now());

        let latency = router.topic_stats()[0].latency_ms.unwrap();
        assert!((250.0..1000.0).contains(&latency), "{latency}");
    }

    #[test]
    fn slow_client_backlog_is_bounded() {
        let mut router = TopicRouter::new();
        let (id, mut rx) = router.register();
        router.subscribe(id, ["/t".to_string()]);
        let frame = Response::TopicData {
            topic: "/t".into(),
            type_name: "t".into(),
            stamp: Timestamp { sec: 0, nanosec: 0 },
            data: DynValue::Bool(true),
        };
        for _ in 0..CLIENT_QUEUE_CAPACITY + 10 {
            router.route(&frame);
        }
        let mut queued = 0;
        while rx.try_recv().is_ok() {
            queued += 1;
        }
        assert_eq!(queued, CLIENT_QUEUE_CAPACITY);
    }
}
