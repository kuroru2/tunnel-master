use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::errors::TunnelError;
use crate::tunnel::connection::SshConnection;

/// Listens on a local port and forwards connections through an SSH channel.
pub struct PortForwarder;

impl PortForwarder {
    /// Bind a local port and forward incoming connections to remote_host:remote_port
    /// via the SSH connection. Runs until the listener errors out.
    pub async fn start(
        ssh: Arc<SshConnection>,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
        death_tx: mpsc::Sender<String>,
        tunnel_id: String,
    ) -> Result<(), TunnelError> {
        let addr: SocketAddr = ([127, 0, 0, 1], local_port).into();
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                TunnelError::PortInUse(local_port)
            } else {
                TunnelError::SshError(format!("Failed to bind port {}: {}", local_port, e))
            }
        })?;

        info!(
            "Forwarding localhost:{} -> {}:{}",
            local_port, remote_host, remote_port
        );

        loop {
            match listener.accept().await {
                Ok((tcp_stream, peer_addr)) => {
                    debug!(
                        "Accepted connection from {} on port {}",
                        peer_addr, local_port
                    );

                    let ssh = ssh.clone();
                    let rh = remote_host.clone();
                    let rp = remote_port;
                    let lp = local_port;
                    let tid = tunnel_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(ssh, tcp_stream, &rh, rp, lp).await
                        {
                            warn!("Connection handling error on tunnel {}: {}", tid, e);
                            // Don't report individual connection errors as tunnel death
                            // The health monitor handles tunnel-level failures
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error on port {}: {}", local_port, e);
                    let _ = death_tx
                        .send(format!("Listener error: {}", e))
                        .await;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        ssh: Arc<SshConnection>,
        tcp_stream: tokio::net::TcpStream,
        remote_host: &str,
        remote_port: u16,
        local_port: u16,
    ) -> Result<(), TunnelError> {
        let channel = ssh
            .open_direct_tcpip(remote_host, remote_port, "127.0.0.1", local_port)
            .await?;

        // Use into_stream() to get an AsyncRead+AsyncWrite wrapper around the SSH channel
        let (mut channel_reader, mut channel_writer) =
            tokio::io::split(channel.into_stream());
        let (mut tcp_reader, mut tcp_writer) = tokio::io::split(tcp_stream);

        // Bidirectional pipe
        tokio::select! {
            result = tokio::io::copy(&mut tcp_reader, &mut channel_writer) => {
                if let Err(e) = result {
                    debug!("TCP->SSH copy ended: {}", e);
                }
                // Signal EOF to the SSH side
                let _ = channel_writer.shutdown().await;
            }
            result = tokio::io::copy(&mut channel_reader, &mut tcp_writer) => {
                if let Err(e) = result {
                    debug!("SSH->TCP copy ended: {}", e);
                }
                // Signal EOF to the TCP side
                let _ = tcp_writer.shutdown().await;
            }
        }

        debug!("Connection closed on port {}", local_port);
        Ok(())
    }
}
