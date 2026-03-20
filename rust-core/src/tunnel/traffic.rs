use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::debug;

use crate::events::TunnelEventHandler;
use crate::types::TrafficSample;

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

/// Shared ring buffer of traffic samples. Max 60 entries (60 seconds).
pub type TrafficHistory = Arc<Mutex<VecDeque<TrafficSample>>>;

pub fn new_traffic_history() -> TrafficHistory {
    Arc::new(Mutex::new(VecDeque::with_capacity(60)))
}

/// Samples traffic counters every 1 second, stores in history, pushes via callback.
pub struct TrafficSampler;

impl TrafficSampler {
    pub async fn run(
        tunnel_id: String,
        counters: Arc<TrafficCounters>,
        history: TrafficHistory,
        event_handler: Arc<dyn TunnelEventHandler>,
    ) {
        let interval = std::time::Duration::from_secs(1);

        loop {
            tokio::time::sleep(interval).await;

            let (bytes_in, bytes_out) = counters.take();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let sample = TrafficSample {
                bytes_in,
                bytes_out,
                timestamp,
            };

            // Push to ring buffer
            {
                let mut buf = history.lock().unwrap();
                if buf.len() >= 60 {
                    buf.pop_front();
                }
                buf.push_back(sample.clone());
            }

            // Push to frontend via callback
            event_handler.on_traffic_update(tunnel_id.clone(), sample);

            debug!(
                "Traffic sample for {}: in={} out={}",
                tunnel_id, bytes_in, bytes_out
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrafficSample;
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
