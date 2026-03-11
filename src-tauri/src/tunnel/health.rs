use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::tunnel::connection::SshConnection;

/// Monitors an SSH connection's health by sending periodic keepalive pings.
pub struct HealthMonitor;

impl HealthMonitor {
    /// Run the health monitor loop. Sends keepalive pings at the configured interval.
    /// If a keepalive fails or times out, sends the tunnel ID to death_tx.
    pub async fn run(
        ssh: Arc<SshConnection>,
        tunnel_id: String,
        keepalive_interval_secs: u64,
        keepalive_timeout_secs: u64,
        death_tx: mpsc::Sender<String>,
    ) {
        let interval = Duration::from_secs(keepalive_interval_secs);
        let timeout = Duration::from_secs(keepalive_timeout_secs);

        loop {
            tokio::time::sleep(interval).await;

            let result = tokio::time::timeout(timeout, ssh.send_keepalive()).await;

            match result {
                Ok(Ok(())) => {
                    debug!("Keepalive OK for tunnel {}", tunnel_id);
                }
                Ok(Err(e)) => {
                    warn!("Keepalive error for tunnel {}: {}", tunnel_id, e);
                    let _ = death_tx
                        .send(format!("Keepalive error: {}", e))
                        .await;
                    break;
                }
                Err(_) => {
                    warn!("Keepalive timeout for tunnel {}", tunnel_id);
                    let _ = death_tx
                        .send("Keepalive timeout".to_string())
                        .await;
                    break;
                }
            }
        }
    }
}
