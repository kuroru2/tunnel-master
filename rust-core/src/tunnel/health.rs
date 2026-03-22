use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::tunnel::connection::SshConnection;

/// Monitors an SSH connection's health by polling the connection state.
/// Relies on russh's built-in keepalive (keepalive@openssh.com) to detect dead connections.
pub struct HealthMonitor;

impl HealthMonitor {
    /// Run the health monitor loop. Checks connection liveness at the configured interval.
    /// If the connection is dead, sends an error to death_tx.
    pub async fn run(
        ssh: Arc<SshConnection>,
        tunnel_id: String,
        check_interval_secs: u64,
        _keepalive_timeout_secs: u64,
        death_tx: mpsc::Sender<String>,
    ) {
        let interval = Duration::from_secs(check_interval_secs);

        loop {
            tokio::time::sleep(interval).await;

            if ssh.is_alive() {
                debug!("Connection alive for tunnel {}", tunnel_id);
            } else {
                warn!("Connection lost for tunnel {}", tunnel_id);
                let _ = death_tx
                    .send("Connection lost".to_string())
                    .await;
                break;
            }
        }
    }
}
