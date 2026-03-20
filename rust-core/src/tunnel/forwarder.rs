// Stub — will be fully extracted in Task 7.
// Only the public API surface is defined here so manager.rs compiles.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::errors::TunnelError;
use crate::tunnel::connection::SshConnection;
use crate::tunnel::traffic::TrafficCounters;

/// Listens on a local port and forwards connections through an SSH channel.
pub struct PortForwarder;

impl PortForwarder {
    /// Bind a local port and forward incoming connections to remote_host:remote_port
    /// via the SSH connection. Runs until the listener errors out.
    pub async fn start(
        _ssh: Arc<SshConnection>,
        _local_port: u16,
        _remote_host: String,
        _remote_port: u16,
        _death_tx: mpsc::Sender<String>,
        _tunnel_id: String,
        _traffic_counters: Option<Arc<TrafficCounters>>,
    ) -> Result<(), TunnelError> {
        // Stub: real implementation in Task 7
        Ok(())
    }
}
