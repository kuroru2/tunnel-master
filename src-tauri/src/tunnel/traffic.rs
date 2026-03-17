use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tokio::sync::mpsc;
use tracing::debug;

/// Shared atomic counters incremented by the PortForwarder on every copy.
pub struct TrafficCounters {
    pub bytes_in: AtomicU64,
    pub bytes_out: AtomicU64,
}

impl TrafficCounters {
    pub fn new() -> Self {
        Self {
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
        }
    }

    /// Read and reset the counters. Returns (bytes_in, bytes_out) since last call.
    pub fn take(&self) -> (u64, u64) {
        let bytes_in = self.bytes_in.swap(0, Ordering::Relaxed);
        let bytes_out = self.bytes_out.swap(0, Ordering::Relaxed);
        (bytes_in, bytes_out)
    }
}

/// A single traffic sample (1 second of data).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSample {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub timestamp: u64,
}

/// Shared ring buffer of traffic samples. Max 60 entries (60 seconds).
pub type TrafficHistory = Arc<Mutex<VecDeque<TrafficSample>>>;

pub fn new_traffic_history() -> TrafficHistory {
    Arc::new(Mutex::new(VecDeque::with_capacity(60)))
}

/// Tauri event payload for tunnel-traffic.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficEvent {
    pub id: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn counters_increment_and_take() {
        let counters = TrafficCounters::new();
        counters.bytes_in.fetch_add(1000, Ordering::Relaxed);
        counters.bytes_out.fetch_add(500, Ordering::Relaxed);

        let (r_in, r_out) = counters.take();
        assert_eq!(r_in, 1000);
        assert_eq!(r_out, 500);

        // After take, counters should be zero
        let (r_in2, r_out2) = counters.take();
        assert_eq!(r_in2, 0);
        assert_eq!(r_out2, 0);
    }

    #[test]
    fn counters_concurrent_increment() {
        let counters = Arc::new(TrafficCounters::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let c = counters.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    c.bytes_in.fetch_add(1, Ordering::Relaxed);
                    c.bytes_out.fetch_add(2, Ordering::Relaxed);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let (r_in, r_out) = counters.take();
        assert_eq!(r_in, 1000);
        assert_eq!(r_out, 2000);
    }

    #[test]
    fn traffic_history_capacity() {
        let history = new_traffic_history();
        let mut buf = history.lock().unwrap();

        // Fill beyond 60
        for i in 0..70 {
            if buf.len() >= 60 {
                buf.pop_front();
            }
            buf.push_back(TrafficSample {
                bytes_in: i,
                bytes_out: i,
                timestamp: i,
            });
        }

        assert_eq!(buf.len(), 60);
        assert_eq!(buf.front().unwrap().timestamp, 10); // oldest is #10
        assert_eq!(buf.back().unwrap().timestamp, 69); // newest is #69
    }
}
