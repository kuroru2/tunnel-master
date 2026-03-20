// Stub — will be fully extracted in Task 7.
// Only the public API surface is defined here so manager.rs compiles.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::tunnel::connection::SshConnection;

/// Monitors an SSH connection's health by polling the connection state.
pub struct HealthMonitor;

impl HealthMonitor {
    /// Run the health monitor loop.
    pub async fn run(
        _ssh: Arc<SshConnection>,
        _tunnel_id: String,
        _check_interval_secs: u64,
        _keepalive_timeout_secs: u64,
        _death_tx: mpsc::Sender<String>,
    ) {
        // Stub: real implementation in Task 7
    }
}
