# Tunnel Master — In-App Config Editor Design Spec

## Problem

Tunnels are configured by editing `~/.tunnel-master/config.json` manually. Users need to add, edit, and delete tunnels from within the app.

## Scope

- Add, edit, and delete tunnels via UI
- iOS Reminders-style edit mode with two-step delete
- iOS Settings-style grouped form for tunnel fields
- Auto-generated tunnel IDs from name
- Validation on Rust side
- Connected tunnels silently disconnected before edit/delete
- Data model unchanged — same `TunnelConfig`, same JSON file

## UX Flow

Three views managed by state in `App.tsx`:

```
NormalView  ←→  EditList  →  EditForm  →  EditList
                              (add/edit)
```

### Normal View (existing)

- Header gains an "Edit" button (right side)
- Everything else unchanged — tunnel list with Start/Stop controls

### Edit List

- Header: "+ Add" (left), "Edit Tunnels" (center), "Done" (right)
- Each tunnel row shows: red minus button, tunnel name, summary (user@host → localhost:port), chevron
- Tapping red minus reveals a "Delete" confirmation button on the row (iOS two-step pattern)
- Tapping the row navigates to Edit Form
- Tapping "+ Add" navigates to Edit Form with empty fields
- Tapping "Done" returns to Normal View

### Edit Form

- Header: "< Back" (left), "Edit Tunnel" or "New Tunnel" (center), "Save" (right)
- Three grouped sections (iOS Settings style):

**Connection**
| Field | Type | Required | Default |
|-------|------|----------|---------|
| Name | text | yes | — |
| Host | text | yes | — |
| Port | number | no | 22 |
| Username | text | yes | — |
| Key Path | text | no | — |

**Port Forwarding**
| Field | Type | Required | Default |
|-------|------|----------|---------|
| Local Port | number | yes | — |
| Remote Host | text | yes | — |
| Remote Port | number | yes | — |

**Options**
| Field | Type | Required | Default |
|-------|------|----------|---------|
| Auto Connect | toggle | no | false |

- `type` is always `"local"` — hardcoded, not shown in form (reverse/dynamic tunnels are deferred)
- "Save" validates required fields (frontend: disable if empty; backend: full validation)
- "Back" discards unsaved changes without confirmation (deliberate — form is small, not worth a modal)

## New Tauri Commands

| Command | Args | Returns | Description |
|---------|------|---------|-------------|
| `add_tunnel` | `TunnelInput` (no `id`) | `Result<TunnelInfo>` | Generates ID from name, validates, saves config, notifies manager |
| `update_tunnel` | `id: String` + `TunnelInput` | `Result<TunnelInfo>` | Disconnects if connected, updates config (preserves original ID), saves, notifies manager |
| `delete_tunnel` | `id: String` | `Result<()>` | Disconnects if connected, removes from config, saves, notifies manager |
| `get_tunnel_config` | `id: String` | `Result<TunnelConfig>` | Returns full config for a tunnel (needed to populate edit form — `TunnelInfo` lacks `host`, `user`, `keyPath`, etc.) |

## New Manager Commands

| Command | Description |
|---------|-------------|
| `AddTunnel { config, reply }` | Adds tunnel to manager's internal state |
| `UpdateTunnel { config, reply }` | Disconnects if connected, replaces tunnel config in manager state |
| `RemoveTunnel { id, reply }` | Disconnects (if connected) and removes tunnel from manager |

## ID Generation

Slugify the tunnel name: lowercase, replace non-alphanumeric chars with hyphens, collapse consecutive hyphens, trim leading/trailing hyphens.

Examples:
- "ORA Web" → "ora-web"
- "ORA Web (prod)" → "ora-web-prod"

If duplicate exists, append `-2`, `-3`, etc.

**On update:** the original ID is preserved, even if the name changes. IDs are only generated on add.

## Validation (Rust side)

- `name` — non-empty
- `host` — non-empty
- `port` — 1–65535, defaults to 22
- `user` — non-empty
- `localPort` — 1–65535, must not conflict with another configured tunnel (excluding self on update). Runtime port-in-use conflicts are caught separately at connect time.
- `remoteHost` — non-empty
- `remotePort` — 1–65535
- `keyPath` — if provided, validate file exists (with tilde expansion). Empty string treated as "no key" (for future password/agent auth).

Validation errors returned as structured error messages surfaced in the UI error banner.

## Connected Tunnel Handling

- **Edit:** silently disconnect, apply changes, leave disconnected
- **Delete:** silently disconnect, remove from config

No confirmation modal — the user explicitly chose the action.

## Frontend Refresh After Mutations

After `add_tunnel`, `update_tunnel`, or `delete_tunnel` succeeds, the frontend calls `list_tunnels` to refresh the tunnel list. No new event type needed — the existing `tunnel-status-changed` event already triggers a refetch for status changes, and explicit refetch after mutation covers the CRUD case.

## ConfigStore: Atomic Writes

The `save` method writes to a temporary file in the same directory, then renames it over the original. This prevents corruption if the app crashes mid-write.

```
config.json.tmp  →  (write)  →  (rename to config.json)
```

## Rust Type Changes

```rust
/// Input for add/update — no id field
#[derive(Deserialize)]
pub struct TunnelInput {
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub key_path: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    #[serde(default)]
    pub auto_connect: bool,
}
```

`TunnelConfig.key_path` remains `String` (not `Option<String>`). Empty string = no key specified.

## New Frontend Components

### `EditList.tsx`
- Receives `tunnels: TunnelInfo[]`, callbacks for delete/edit/add
- Renders iOS-style rows with minus button, name, summary, chevron
- Two-step delete: tap minus → reveal "Delete" button → tap to confirm

### `EditForm.tsx`
- Receives optional `tunnelId` (null = new tunnel)
- If editing, calls `get_tunnel_config` to load full config fields
- Grouped form fields matching the sections above
- Local form state, submits via `add_tunnel` or `update_tunnel` command
- Disables "Save" if required fields are empty

### `App.tsx` modifications
- New state: `{ view: "normal" | "edit-list" | "edit-form", editingTunnelId: string | null }`
- Header renders conditionally based on view
- Renders `TunnelList`, `EditList`, or `EditForm` based on view

## Files Changed

### Rust (src-tauri/src/)
- `commands.rs` — add `add_tunnel`, `update_tunnel`, `delete_tunnel`, `get_tunnel_config` commands
- `tunnel/manager.rs` — add `AddTunnel`, `UpdateTunnel`, `RemoveTunnel` commands to actor
- `config/store.rs` — add `save` method with atomic write
- `types.rs` — add `TunnelInput` struct
- `lib.rs` — register new commands in `invoke_handler`

### TypeScript (src/)
- `App.tsx` — view state machine, conditional header/content rendering
- `components/EditList.tsx` — new component
- `components/EditForm.tsx` — new component
- `hooks/useTunnels.ts` — add `addTunnel`, `updateTunnel`, `deleteTunnel`, `getTunnelConfig` functions
- `types.ts` — add `TunnelInput` type

## Future Compatibility

The data model stays the same `TunnelConfig` with `keyPath` string. When shared keychain is added later, `keyPath` can be extended to support a `keyId` reference without breaking existing configs (via serde enum or optional field).
