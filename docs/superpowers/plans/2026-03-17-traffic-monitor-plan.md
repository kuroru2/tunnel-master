# Traffic Monitor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real-time traffic sparklines (download/upload) to each tunnel row, with per-second byte counting in Rust and SVG rendering in React.

**Architecture:** AtomicU64 counters in the PortForwarder data path, a per-tunnel TrafficSampler task that reads/resets counters every 1s and emits Tauri events, an Arc<Mutex<VecDeque>> ring buffer for history, and a React SVG sparkline component rendered behind each TunnelItem.

**Tech Stack:** Rust (tokio, std::sync::atomic), Tauri v2 events, React SVG, TypeScript

**Spec:** `docs/superpowers/specs/2026-03-17-traffic-monitor-design.md`

---

## Chunk 1: Rust Backend — Counters, Sampler, and Types

### Task 1: Add TrafficCounters and TrafficSample types

**Files:**
- Create: `src-tauri/src/tunnel/traffic.rs`
- Modify: `src-tauri/src/tunnel/mod.rs`

- [ ] **Step 1: Write the test for TrafficCounters**

In `src-tauri/src/tunnel/traffic.rs`:

```rust
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
```

- [ ] **Step 2: Add traffic module to mod.rs**

In `src-tauri/src/tunnel/mod.rs`, add:

```rust
pub mod traffic;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib tunnel::traffic`
Expected: 3 tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tunnel/traffic.rs src-tauri/src/tunnel/mod.rs
git commit -m "feat(traffic): add TrafficCounters, TrafficSample, and TrafficHistory types"
```

---

### Task 2: Add TrafficSampler

**Files:**
- Modify: `src-tauri/src/tunnel/traffic.rs`

- [ ] **Step 1: Add the TrafficSampler to traffic.rs**

Append to `src-tauri/src/tunnel/traffic.rs`, before the `#[cfg(test)]` block:

```rust
/// Samples traffic counters every 1 second, stores in history, emits Tauri event.
pub struct TrafficSampler;

impl TrafficSampler {
    pub async fn run(
        tunnel_id: String,
        counters: Arc<TrafficCounters>,
        history: TrafficHistory,
        app_handle: tauri::AppHandle,
    ) {
        use tauri::Emitter;

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

            // Emit event to frontend
            let event = TrafficEvent {
                id: tunnel_id.clone(),
                bytes_in,
                bytes_out,
            };
            let _ = app_handle.emit("tunnel-traffic", &event);

            debug!(
                "Traffic sample for {}: in={} out={}",
                tunnel_id, bytes_in, bytes_out
            );
        }
    }
}
```

- [ ] **Step 2: Run tests (ensure nothing broke)**

Run: `cd src-tauri && cargo test --lib tunnel::traffic`
Expected: 3 tests still pass

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tunnel/traffic.rs
git commit -m "feat(traffic): add TrafficSampler task"
```

---

### Task 3: Replace tokio::io::copy with counting copy loop in PortForwarder

**Files:**
- Modify: `src-tauri/src/tunnel/forwarder.rs`

- [ ] **Step 1: Update PortForwarder to accept TrafficCounters**

Replace the entire `src-tauri/src/tunnel/forwarder.rs` with:

```rust
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
        counters: Option<Arc<TrafficCounters>>,
    ) -> Result<(), TunnelError> {
        let channel = ssh
            .open_direct_tcpip(remote_host, remote_port, "127.0.0.1", local_port)
            .await?;

        let (mut channel_reader, mut channel_writer) =
            tokio::io::split(channel.into_stream());
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
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test --lib`
Expected: all existing tests pass (PortForwarder is not unit-tested, but compilation must succeed and manager tests must pass)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tunnel/forwarder.rs
git commit -m "feat(traffic): replace tokio::io::copy with counting copy loop in PortForwarder"
```

---

### Task 4: Add show_traffic_chart to config types

**Files:**
- Modify: `src-tauri/src/types.rs`

- [ ] **Step 1: Add the field to TunnelConfig, TunnelInput, TunnelInfo**

In `src-tauri/src/types.rs`:

Add a default helper function next to the existing ones (near line 77):

```rust
fn default_true() -> bool { true }
```

Add to `TunnelConfig` struct (after `jump_host` field, line 52):

```rust
    #[serde(default = "default_true")]
    pub show_traffic_chart: bool,
```

Add to `TunnelInput` struct (after `jump_host` field, line 74):

```rust
    #[serde(default = "default_true")]
    pub show_traffic_chart: bool,
```

Update `TunnelInput::to_config` (add to the returned TunnelConfig):

```rust
            show_traffic_chart: self.show_traffic_chart,
```

Add to `TunnelInfo` struct (after `jump_host_name` field, line 145):

```rust
    pub show_traffic_chart: bool,
```

- [ ] **Step 2: Update test_config in manager.rs tests**

In `src-tauri/src/tunnel/manager.rs`, in the `test_config()` function, add `show_traffic_chart: true` to both TunnelConfig entries (after `jump_host: None`):

```rust
                    show_traffic_chart: true,
```

And in the `reload_config_adds_new_tunnels` test, add it to the new tunnel config as well.

- [ ] **Step 3: Update tunnel_to_info in manager.rs**

In the `tunnel_to_info` method (around line 170), add to the TunnelInfo construction:

```rust
            show_traffic_chart: tunnel.config.show_traffic_chart,
```

- [ ] **Step 4: Update types.rs tests**

In `src-tauri/src/types.rs`, update `tunnel_input_to_config` test — add `show_traffic_chart: true` to the TunnelInput construction.

- [ ] **Step 5: Run tests**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/tunnel/manager.rs
git commit -m "feat(traffic): add show_traffic_chart field to TunnelConfig, TunnelInput, TunnelInfo"
```

---

### Task 5: Integrate TrafficSampler into TunnelManagerActor

**Files:**
- Modify: `src-tauri/src/tunnel/manager.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add traffic fields to TunnelState and imports**

In `src-tauri/src/tunnel/manager.rs`, add import at the top:

```rust
use crate::tunnel::traffic::{self, TrafficCounters, TrafficHistory, TrafficSample};
```

Add to `TunnelState` struct (after `generation: u64`):

```rust
    /// Traffic byte counters, shared with PortForwarder
    traffic_counters: Option<Arc<TrafficCounters>>,
    /// Ring buffer of traffic samples (last 60 seconds)
    traffic_history: TrafficHistory,
```

Update `TunnelState::new` to initialize:

```rust
            traffic_counters: None,
            traffic_history: traffic::new_traffic_history(),
```

- [ ] **Step 2: Add GetTrafficHistory to ManagerCommand**

In the `ManagerCommand` enum, add:

```rust
    GetTrafficHistory {
        id: String,
        reply: oneshot::Sender<Result<Vec<TrafficSample>, TunnelError>>,
    },
```

- [ ] **Step 3: Handle GetTrafficHistory in the run loop**

In the `run` method's match block, add before the `Shutdown` arm:

```rust
                ManagerCommand::GetTrafficHistory { id, reply } => {
                    let result = match self.tunnels.get(&id) {
                        Some(tunnel) => {
                            let buf = tunnel.traffic_history.lock().unwrap();
                            Ok(buf.iter().cloned().collect())
                        }
                        None => Err(TunnelError::TunnelNotFound(id)),
                    };
                    let _ = reply.send(result);
                }
```

- [ ] **Step 4: Spawn TrafficSampler and pass counters to PortForwarder in handle_connect**

In `handle_connect`, **before** the PortForwarder spawn block (before line 548, before `// Spawn port forwarder`):

Create traffic counters:

```rust
        // Create traffic counters (always, even without app_handle — forwarder counts regardless)
        let traffic_counters = Arc::new(TrafficCounters::new());
```

Update the PortForwarder spawn to pass `traffic_counters`. Change the existing forwarder spawn block:

Replace:
```rust
        let forwarder_handle = tokio::spawn(async move {
            if let Err(e) = PortForwarder::start(
                fwd_ssh,
                local_port,
                fwd_remote_host,
                remote_port,
                fwd_death_tx.clone(),
                fwd_tunnel_id,
            )
```

With:
```rust
        let fwd_counters = Some(traffic_counters.clone());
        let forwarder_handle = tokio::spawn(async move {
            if let Err(e) = PortForwarder::start(
                fwd_ssh,
                local_port,
                fwd_remote_host,
                remote_port,
                fwd_death_tx.clone(),
                fwd_tunnel_id,
                fwd_counters,
            )
```

**After** the health monitor spawn block (around line 582), spawn the traffic sampler conditionally:

```rust
        // Spawn traffic sampler (only if we have an app handle for emitting events)
        let sampler_abort = if let Some(sampler_app_handle) = self.app_handle.clone() {
            let sampler_counters = traffic_counters.clone();
            let sampler_history = {
                let tunnel = self.tunnels.get(id).unwrap();
                tunnel.traffic_history.clone()
            };
            let sampler_tunnel_id = id.to_string();
            let sampler_handle = tokio::spawn(async move {
                traffic::TrafficSampler::run(
                    sampler_tunnel_id,
                    sampler_counters,
                    sampler_history,
                    sampler_app_handle,
                )
                .await;
            });
            Some(sampler_handle.abort_handle())
        } else {
            None
        };
```

In the "Store state" block, add the sampler abort handle and store counters:

```rust
            if let Some(abort) = sampler_abort {
                tunnel.abort_handles.push(abort);
            }
            tunnel.traffic_counters = Some(traffic_counters);
```

- [ ] **Step 5: Clear traffic state on disconnect**

In `handle_disconnect`, after `tunnel.ki_slot = None;` add:

```rust
            tunnel.traffic_counters = None;
            tunnel.traffic_history.lock().unwrap().clear();
```

In `handle_tunnel_died`, after `tunnel.ki_slot = None;` add:

```rust
            tunnel.traffic_counters = None;
            tunnel.traffic_history.lock().unwrap().clear();
```

- [ ] **Step 6: Add get_traffic_history command**

In `src-tauri/src/commands.rs`, add:

```rust
use crate::tunnel::traffic::TrafficSample;
```

And add the command function:

```rust
#[tauri::command]
pub async fn get_traffic_history(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TrafficSample>, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::GetTrafficHistory {
            id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 7: Register the command in lib.rs**

In `src-tauri/src/lib.rs`, add to the `invoke_handler` list:

```rust
            commands::get_traffic_history,
```

- [ ] **Step 8: Run tests**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/tunnel/manager.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(traffic): integrate TrafficSampler into manager, add get_traffic_history command"
```

---

## Chunk 2: Frontend — Sparkline, TunnelItem, and EditForm

### Task 6: Add TypeScript types for traffic

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Add TrafficSample and update TunnelInfo**

In `src/types.ts`, add:

```typescript
export interface TrafficSample {
  bytesIn: number;
  bytesOut: number;
  timestamp: number;
}

export interface TrafficEvent {
  id: string;
  bytesIn: number;
  bytesOut: number;
}
```

Add `showTrafficChart` to `TunnelInfo`:

```typescript
  showTrafficChart: boolean;
```

Add `showTrafficChart` to `TunnelInput`:

```typescript
  showTrafficChart: boolean;
```

Add `showTrafficChart` to `TunnelConfig`:

```typescript
  showTrafficChart: boolean;
```

- [ ] **Step 2: Commit**

```bash
git add src/types.ts
git commit -m "feat(traffic): add TrafficSample, TrafficEvent types and showTrafficChart field"
```

---

### Task 7: Create TrafficSparkline component

**Files:**
- Create: `src/components/TrafficSparkline.tsx`

- [ ] **Step 1: Create the component**

```tsx
import type { TrafficSample } from "../types";

interface TrafficSparklineProps {
  samples: TrafficSample[];
}

export function TrafficSparkline({ samples }: TrafficSparklineProps) {
  if (samples.length < 2) return null;

  const maxPoints = 60;
  const viewWidth = 320;
  const viewHeight = 56;

  // Find max value for Y scaling (minimum floor of 100 to avoid jitter)
  const maxVal = Math.max(
    100,
    ...samples.map((s) => Math.max(s.bytesIn, s.bytesOut))
  );

  const toPoints = (getValue: (s: TrafficSample) => number): string => {
    const step = viewWidth / (maxPoints - 1);
    const startIndex = Math.max(0, maxPoints - samples.length);
    return samples
      .map((s, i) => {
        const x = (startIndex + i) * step;
        const y = viewHeight - (getValue(s) / maxVal) * (viewHeight - 4);
        return `${x},${y}`;
      })
      .join(" ");
  };

  const downloadPoints = toPoints((s) => s.bytesIn);
  const uploadPoints = toPoints((s) => s.bytesOut);

  return (
    <svg
      viewBox={`0 0 ${viewWidth} ${viewHeight}`}
      preserveAspectRatio="none"
      style={{
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        opacity: 0.25,
        pointerEvents: "none",
      }}
    >
      <polyline
        points={downloadPoints}
        fill="none"
        stroke="#4ade80"
        strokeWidth="1.5"
      />
      <polyline
        points={uploadPoints}
        fill="none"
        stroke="#60a5fa"
        strokeWidth="1"
        strokeDasharray="3,2"
      />
    </svg>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/TrafficSparkline.tsx
git commit -m "feat(traffic): create TrafficSparkline SVG component"
```

---

### Task 8: Integrate traffic into TunnelItem

**Files:**
- Modify: `src/components/TunnelItem.tsx`

- [ ] **Step 1: Add traffic state, event listener, and rendering**

At the top of `TunnelItem.tsx`, add imports:

```typescript
import { useRef, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TunnelInfo, TunnelStatus, TrafficSample, TrafficEvent } from "../types";
import { TrafficSparkline } from "./TrafficSparkline";
```

(Replace the existing `import { useRef, useState, useEffect } from "react";` and `import type { TunnelInfo, TunnelStatus } from "../types";`)

Inside the `TunnelItem` component, after the existing `useEffect` cleanup block (line ~70), add:

```typescript
  // -- Traffic monitoring --
  const [trafficSamples, setTrafficSamples] = useState<TrafficSample[]>([]);
  const showChart = tunnel.showTrafficChart;

  // Fetch history when tunnel is connected and chart is enabled
  useEffect(() => {
    if (tunnel.status !== "connected" || !showChart) {
      setTrafficSamples([]);
      return;
    }
    invoke<TrafficSample[]>("get_traffic_history", { id: tunnel.id })
      .then(setTrafficSamples)
      .catch(() => {}); // ignore errors
  }, [tunnel.id, tunnel.status, showChart]);

  // Listen for real-time traffic events
  useEffect(() => {
    if (tunnel.status !== "connected" || !showChart) return;

    const unlisten = listen<TrafficEvent>("tunnel-traffic", (event) => {
      if (event.payload.id !== tunnel.id) return;
      const sample: TrafficSample = {
        bytesIn: event.payload.bytesIn,
        bytesOut: event.payload.bytesOut,
        timestamp: Date.now(),
      };
      setTrafficSamples((prev) => {
        const next = [...prev, sample];
        return next.length > 60 ? next.slice(-60) : next;
      });
    });

    return () => { unlisten.then((fn) => fn()); };
  }, [tunnel.id, tunnel.status, showChart]);

  // Format bytes/s for display
  const formatRate = (bytes: number): string => {
    if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB/s`;
    if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(1)} KB/s`;
    return `${bytes} B/s`;
  };

  const lastSample = trafficSamples.length > 0 ? trafficSamples[trafficSamples.length - 1] : null;
  const isIdle = lastSample ? lastSample.bytesIn + lastSample.bytesOut < 10 : true;
  const showTraffic = showChart && isConnected && trafficSamples.length > 0;
```

Update the root `<div>` of the return JSX to add `position: relative` and `overflow: hidden`:

Change:
```tsx
    <div className="flex items-center justify-between px-3 py-2.5 hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] rounded-lg transition-colors">
```

To:
```tsx
    <div className="relative overflow-hidden flex items-center justify-between px-3 py-2.5 hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] rounded-lg transition-colors">
```

Right after the opening `<div>`, before the content flex container, add the sparkline:

```tsx
      {showTraffic && <TrafficSparkline samples={trafficSamples} />}
```

Add traffic text before the error icon. Find the `{tunnel.errorMessage && (` block and add before it:

```tsx
      {showTraffic && (
        <div className="shrink-0 ml-auto text-right z-10" style={{ fontFamily: "var(--font-mono)" }}>
          {isIdle ? (
            <div className="text-[10px] text-[#6b7280]">idle</div>
          ) : (
            <>
              <div className="text-[10px] text-[#4ade80]">↓ {formatRate(lastSample!.bytesIn)}</div>
              <div className="text-[10px] text-[#60a5fa]">↑ {formatRate(lastSample!.bytesOut)}</div>
            </>
          )}
        </div>
      )}
```

Also add `z-10` to the existing content div and button to ensure they render above the sparkline:

The inner flex div with tunnel name/ports should get `z-10`:
```tsx
      <div className="flex items-center gap-3 min-w-0 z-10">
```

The toggle button should get `z-10`:
```tsx
      <button
        onClick={handleToggle}
        disabled={visuallyBusy || recentlyFailed}
        className={`shrink-0 ml-3 w-7 h-4 rounded-full relative transition-colors z-10 ${
```

- [ ] **Step 2: Run type check**

Run: `npx tsc -b`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/TunnelItem.tsx
git commit -m "feat(traffic): integrate sparkline and traffic rates into TunnelItem"
```

---

### Task 9: Add showTrafficChart to EditForm

**Files:**
- Modify: `src/components/EditForm.tsx`

- [ ] **Step 1: Add showTrafficChart to emptyForm and config hydration**

In `src/components/EditForm.tsx`, update `emptyForm` to include:

```typescript
  showTrafficChart: true,
```

In the `getTunnelConfig` hydration (the `.then((config) => {` block), add:

```typescript
            showTrafficChart: config.showTrafficChart ?? true,
```

- [ ] **Step 2: Add the toggle in the Options section**

In the Options section, after the Auto Connect toggle `</div>`, add another toggle row:

```tsx
          <div className="flex items-center justify-between px-3 py-2.5 border-t border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
            <span className="text-sm text-[#999] dark:text-[#999]">Traffic Chart</span>
            <button
              onClick={() => updateField("showTrafficChart", !form.showTrafficChart)}
              className={`w-8 h-[18px] rounded-full relative transition-colors ${
                form.showTrafficChart ? "bg-[#4ade80]" : "bg-[#ccc] dark:bg-[#333]"
              }`}
            >
              <div
                className={`w-[14px] h-[14px] rounded-full absolute top-[2px] transition-transform ${
                  form.showTrafficChart
                    ? "translate-x-[14px] bg-white"
                    : "translate-x-[2px] bg-white dark:bg-[#888]"
                }`}
              />
            </button>
          </div>
```

- [ ] **Step 3: Run type check and lint**

Run: `npx tsc -b && npm run lint`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/components/EditForm.tsx
git commit -m "feat(traffic): add showTrafficChart toggle to EditForm"
```

---

### Task 10: Final verification

- [ ] **Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass

- [ ] **Step 2: Run frontend checks**

Run: `npx tsc -b && npm run lint`
Expected: no errors

- [ ] **Step 3: Build the app**

Run: `npx tauri build --bundles app`
Expected: build succeeds

- [ ] **Step 4: Commit any remaining changes and tag**

```bash
git add -A
git commit -m "feat: traffic monitor with real-time sparkline charts in tunnel rows"
```
