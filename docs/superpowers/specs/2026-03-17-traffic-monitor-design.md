# Traffic Monitor Design

## Overview

Real-time traffic visualization embedded in each tunnel list row. Two sparklines — solid green for download, dashed blue for upload — drawn as a low-opacity SVG background behind the existing tunnel item content. Per-second byte counters collected in Rust, streamed to the React frontend via Tauri events, with a command to fetch full history on window open.

## Visual Design

**Style:** Dual line sparkline (option C from brainstorming).

- Solid green line: download (bytes received from remote)
- Dashed blue line: upload (bytes sent to remote)
- Lines drawn at ~25% opacity behind tunnel row content
- Y axis auto-scales to max value in current 60-sample window
- Traffic rate text on the right side of the row: `↓ 9.3 KB/s` (green), `↑ 3.1 KB/s` (blue)
- Near-zero traffic shows "idle" instead of numbers
- Disconnected tunnels: no chart, no traffic text (row unchanged from current design)

**Per-tunnel toggle:** `showTrafficChart` boolean in config (default `true`). When disabled, the row renders exactly as it does today — no sparkline, no KB/s text. Data is still collected in Rust regardless of this flag.

## Data Collection (Rust)

### TrafficCounters

Shared atomic counters incremented by the PortForwarder as it copies bytes.

```rust
pub struct TrafficCounters {
    pub bytes_in: AtomicU64,   // remote → local (download)
    pub bytes_out: AtomicU64,  // local → remote (upload)
}
```

Stored as `Arc<TrafficCounters>` on the tunnel's state. The PortForwarder receives a clone and increments on every data copy operation.

**Byte counting approach:** The current PortForwarder uses `tokio::io::copy` which is opaque — no way to hook into byte flow. Replace it with a custom copy loop that reads into a buffer, increments the counter by bytes read, then writes. Alternatively, wrap the reader/writer in a `CountingStream` adapter implementing `AsyncRead`/`AsyncWrite` that increments the counter on each `poll_read`/`poll_write`. The custom copy loop is simpler and recommended.

### TrafficSample

```rust
pub struct TrafficSample {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub timestamp: u64,  // unix millis
}
```

### TrafficSampler

A new async task spawned per tunnel alongside HealthMonitor when a tunnel connects.

- Runs every 1 second
- Reads and resets the AtomicU64 counters (swap with 0)
- Pushes a TrafficSample into a shared `Arc<Mutex<VecDeque<TrafficSample>>>` (max capacity 60)
- Emits a `tunnel-traffic` Tauri event via `AppHandle` (passed to sampler on spawn, same pattern as event forwarding in `lib.rs`)

Stops when the tunnel disconnects (via the same abort handle pattern used by HealthMonitor and PortForwarder).

### TunnelState Changes

Add to `TunnelState`:
- `traffic_counters: Option<Arc<TrafficCounters>>` — created on connect, cleared on disconnect
- `traffic_history: Arc<Mutex<VecDeque<TrafficSample>>>` — shared between sampler (writes) and manager (reads for `get_traffic_history` command). Cleared on disconnect. Using `Arc<Mutex>` rather than a ManagerCommand variant avoids per-second message overhead per tunnel while keeping the actor simple.

## Commands & Events (Rust)

### New ManagerCommand Variant

```rust
GetTrafficHistory {
    id: String,
    reply: oneshot::Sender<Result<Vec<TrafficSample>, TunnelError>>,
}
```

The handler locks `traffic_history` on the tunnel's state, clones the VecDeque into a Vec, and sends it back.

### New Tauri Command

`get_traffic_history(id: String) → Vec<TrafficSample>`

Sends `GetTrafficHistory` to the manager actor, returns up to 60 samples. Called by frontend when the tray window opens to populate charts for already-connected tunnels.

### New Event

`tunnel-traffic` — emitted every 1 second per connected tunnel.

Payload:
```json
{
  "id": "tunnel-id",
  "bytes_in": 9300,
  "bytes_out": 3100
}
```

### Config Changes

Add `show_traffic_chart: bool` (Rust field name) to:
- `TunnelConfig` — persisted, `#[serde(default = "default_true")]`, serializes as `showTrafficChart` via existing `camelCase` rename
- `TunnelInput` — from frontend, same serde behavior
- `TunnelInfo` — returned to frontend, serializes as `showTrafficChart`

Note: all three structs use `#[serde(rename_all = "camelCase")]`, so the Rust field `show_traffic_chart` auto-serializes to `showTrafficChart` in JSON.

## Frontend

### New Component: TrafficSparkline

`TrafficSparkline.tsx` — pure SVG rendering component.

**Props:**
- `samples: TrafficSample[]` — up to 60 data points
- Width/height derived from parent container

**Rendering:**
- SVG with viewBox matching the row dimensions
- Two `<polyline>` elements: green solid (download), blue dashed (upload)
- Y scale: `max(all values in window)` with a minimum floor to avoid flat-line jitter
- Positioned absolutely behind row content via CSS (`position: absolute; inset: 0; opacity: 0.25; pointer-events: none`)

### TunnelItem Changes

- **Sample buffer:** `useState<TrafficSample[]>` — since events arrive at 1Hz (not high frequency), update state directly in the event listener callback. No need for a ref + setInterval indirection at this cadence.
- **On mount (if connected):** calls `get_traffic_history(id)` to fill the buffer immediately.
- **Event listener:** subscribes to `tunnel-traffic` events, filters by tunnel ID, appends to buffer ref.
- **Traffic text:** displayed to the right of the tunnel info, before the toggle. Green `↓ X.X KB/s`, blue `↑ X.X KB/s`. Shows "idle" when both directions are near-zero for the last sample.
- **When `show_traffic_chart` is false:** no sparkline rendered, no traffic text. Row is identical to current design.
- **Row styling:** add `position: relative; overflow: hidden` to the row container so the absolute-positioned SVG stays within bounds.

### EditForm Changes

Add a checkbox at the bottom of the Connection section:
- Label: "Show traffic chart"
- Checked by default
- Maps to `showTrafficChart` field on TunnelInput
- Add `showTrafficChart: true` to the `emptyForm` constant
- Include in `getTunnelConfig` hydration (EditForm mount)

## Data Flow

```
PortForwarder (copies bytes)
    → increments AtomicU64 counters on TrafficCounters

TrafficSampler (1s interval, per connected tunnel)
    → reads & resets counters
    → pushes TrafficSample into VecDeque<60> on TunnelState
    → emits "tunnel-traffic" Tauri event

Frontend (window opens)
    → calls get_traffic_history(id) for each connected tunnel
    → populates sample buffer, renders chart immediately

Frontend (ongoing)
    → listens to "tunnel-traffic" events
    → appends to state array, triggers re-render of sparkline + KB/s text

Disconnect:
    → TrafficSampler aborted
    → traffic_history cleared
    → frontend buffer cleared, chart removed

Reconnect:
    → fresh counters, empty buffer
    → chart builds up over 60 seconds
```

## Edge Cases

- **Window closed then reopened:** `get_traffic_history` returns last 60s — chart appears instantly with full history.
- **Tunnel disconnects:** sampler stops, buffer cleared, chart disappears from row.
- **`showTrafficChart: false`:** data still collected in Rust (negligible cost), only rendering is skipped. If user re-enables the flag, chart populates from existing history on next window open.
- **Multiple TCP connections through one tunnel:** all bytes counted together — correct per-tunnel aggregate.
- **No traffic (idle tunnel):** flat lines near zero, "idle" text instead of KB/s numbers.
- **Very high traffic:** auto-scaling Y axis handles any throughput. KB/s text auto-formats: KB/s → MB/s → GB/s.

## Files to Create/Modify

### Create
- `src-tauri/src/tunnel/traffic.rs` — TrafficCounters, TrafficSample, TrafficSampler
- `src/components/TrafficSparkline.tsx` — SVG sparkline component

### Modify
- `src-tauri/src/tunnel/mod.rs` — add traffic module
- `src-tauri/src/tunnel/forwarder.rs` — accept and increment TrafficCounters
- `src-tauri/src/tunnel/manager.rs` — spawn TrafficSampler, store history, handle new command
- `src-tauri/src/commands.rs` — add get_traffic_history command
- `src-tauri/src/types.rs` — add showTrafficChart to TunnelConfig/TunnelInput/TunnelInfo, add TrafficSample serde
- `src-tauri/src/lib.rs` — register new command
- `src/components/TunnelItem.tsx` — integrate sparkline, traffic text, event listener
- `src/components/EditForm.tsx` — add showTrafficChart checkbox
- `src/types.ts` — add TrafficSample type, showTrafficChart field
- `src/hooks/useTunnels.ts` — no changes needed (events handled in TunnelItem directly)

## Testing

- **TrafficCounters:** unit test increment + swap-to-zero from multiple threads
- **TrafficSampler ring buffer:** unit test capacity enforcement (max 60), clearing on disconnect
- **get_traffic_history:** returns empty Vec for disconnected tunnels, returns samples for connected tunnels
- **Counting copy loop:** verify byte counts match actual data transferred
- **Frontend:** manual testing — connect tunnel, verify sparkline appears and updates, toggle showTrafficChart off and verify chart disappears
