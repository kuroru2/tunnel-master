# Tunnel List Reordering

## Goal

Fix unstable tunnel ordering and add drag-to-reorder in edit mode.

## Problem

The tunnel manager stores tunnels in a `HashMap<String, TunnelState>`, so `ListTunnels` returns them in non-deterministic order. Editing a tunnel causes it to appear in a different position.

## Design

### Part 1: Stable ordering (backend)

Add `tunnel_order: Vec<String>` to `TunnelManagerActor`. This vec tracks tunnel IDs in their config array order. `ListTunnels` returns tunnels sorted by this vec.

Updated on:
- **Startup/reload**: populated from `config.tunnels` array order
- **Add**: ID appended to end
- **Delete**: ID removed from vec

### Part 2: Reorder command (backend)

New `ManagerCommand::ReorderTunnels { ids: Vec<String>, reply }` variant. New Tauri command:

```rust
#[tauri::command]
pub async fn reorder_tunnels(ids: Vec<String>, state: State<'_, AppState>) -> Result<(), String>
```

Validates all IDs exist, updates `tunnel_order`, reorders `config.tunnels` array to match, saves to disk.

### Part 3: Drag-to-reorder (frontend)

Drag-and-drop in `EditList.tsx` only (not the main `TunnelList.tsx`).

- Each row gets `draggable="true"` with a grip handle icon on the left
- `onDragStart` stores dragged index
- `onDragOver` shows drop indicator line
- `onDrop` reorders local state and calls `reorder_tunnels`
- No new dependencies

## Files to modify

- `src-tauri/src/tunnel/manager.rs` — add `tunnel_order` vec, update ListTunnels/Add/Delete/Reload
- `src-tauri/src/commands.rs` — add `reorder_tunnels` command
- `src-tauri/src/lib.rs` — register new command in invoke_handler
- `src/components/EditList.tsx` — add drag-and-drop UI
- `src/hooks/useTunnels.ts` — add `reorderTunnels` function
