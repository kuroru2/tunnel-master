use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::errors::TunnelError;
use crate::tunnel::connection::SshConnection;
use crate::tunnel::traffic::TrafficCounters;

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
        traffic_counters: Option<Arc<TrafficCounters>>,
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
                    let counters = traffic_counters.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(ssh, tcp_stream, &rh, rp, lp, counters).await
                        {
                            warn!("Connection handling error on tunnel {}: {}", tid, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error on port {}: {}", local_port, e);
                    let _ = death_tx.send(format!("Listener error: {}", e)).await;
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
        counters: Option<Arc<TrafficCounters>>,
    ) -> Result<(), TunnelError> {
        let channel = ssh
            .open_direct_tcpip(remote_host, remote_port, "127.0.0.1", local_port)
            .await?;

        let (mut channel_reader, mut channel_writer) = tokio::io::split(channel.into_stream());
        let (mut tcp_reader, mut tcp_writer) = tokio::io::split(tcp_stream);

        // Bidirectional pipe with byte counting
        tokio::select! {
            result = Self::copy_with_counting(&mut tcp_reader, &mut channel_writer, &counters, false) => {
                if let Err(e) = result {
                    debug!("TCP->SSH copy ended: {}", e);
                }
                let _ = channel_writer.shutdown().await;
            }
            result = Self::copy_with_counting(&mut channel_reader, &mut tcp_writer, &counters, true) => {
                if let Err(e) = result {
                    debug!("SSH->TCP copy ended: {}", e);
                }
                let _ = tcp_writer.shutdown().await;
            }
        }

        debug!("Connection closed on port {}", local_port);
        Ok(())
    }

    /// Copy bytes from reader to writer, incrementing traffic counters.
    /// `is_inbound` = true means remote→local (bytes_in), false means local→remote (bytes_out).
    async fn copy_with_counting<R, W>(
        reader: &mut R,
        writer: &mut W,
        counters: &Option<Arc<TrafficCounters>>,
        is_inbound: bool,
    ) -> Result<(), std::io::Error>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut buf = [0u8; 8192];
        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                return Ok(()); // EOF
            }
            writer.write_all(&buf[..n]).await?;

            if let Some(c) = counters {
                if is_inbound {
                    c.bytes_in.fetch_add(n as u64, Ordering::Relaxed);
                } else {
                    c.bytes_out.fetch_add(n as u64, Ordering::Relaxed);
                }
            }
        }
    }
}
