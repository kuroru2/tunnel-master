# Minimal Developer Theme Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hardcoded dark theme with a "Minimal Developer" visual style that follows macOS system appearance (light/dark), and polish all UI components.

**Architecture:** Tailwind v4 `dark:` utility classes on every themed element. No config file needed — Tailwind v4 uses media-query dark mode by default. CSS custom properties in `index.css` for box-shadow glow and monospace font stack. Each component is updated independently to be theme-aware.

**Tech Stack:** React, Tailwind CSS v4, Vite, Tauri v2

**Spec:** `docs/superpowers/specs/2026-03-12-minimal-developer-theme-design.md`

---

## File Structure

All modifications — no new files created.

| File | Responsibility |
|------|---------------|
| `src/index.css` | CSS custom properties for glow, monospace font, base theme vars |
| `src/App.tsx` | Main view shell — background, header, error banner, loading state |
| `src/components/TunnelItem.tsx` | Tunnel row — two-dot model, monospace ports, inline errors |
| `src/components/TunnelList.tsx` | List wrapper — themed empty state with icon |
| `src/components/EditList.tsx` | Edit mode list — themed actions, empty state, delete confirmation |
| `src/components/EditForm.tsx` | Form — themed sections, inputs, toggle, loading/error states |
| `src/components/PassphraseDialog.tsx` | Dialog — backdrop blur, inverted button, focus ring |

---

## Chunk 1: Foundation and Main Views

### Task 1: CSS Custom Properties

**Files:**
- Modify: `src/index.css`

- [ ] **Step 1: Add CSS custom properties**

Replace the contents of `src/index.css` with:

```css
@import "tailwindcss";

:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;

  --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
  --glow-green: 0 0 4px rgba(74, 222, 128, 0.4);
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
}
```

- [ ] **Step 2: Verify the app still builds**

Run: `cd /Users/sergiiolyva/ctbto/projects/tunnel-master && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/index.css
git commit -m "style: add CSS custom properties for theme foundation"
```

---

### Task 2: App.tsx — Theme Shell, Header, Error Banner, Loading

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Update App.tsx**

Replace the entire file. The component logic stays the same — only className values change. Key changes:
- `bg-gray-900 text-white` → `bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5]`
- Header border: `border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]`
- Subtitle: `text-[#999] dark:text-[#666]`
- Edit icon: `text-[#999] dark:text-[#888]` (remove blue)
- Error banner: left-stripe style — `border-l-2 border-red-500 bg-red-500/[0.04] dark:bg-red-500/[0.06] rounded-r`
- Error text: `text-[#dc2626] dark:text-[#f87171]`
- Loading text: `text-[#999] dark:text-[#666]`

Full file contents:

```tsx
import { useState } from "react";
import { TunnelList } from "./components/TunnelList";
import { PassphraseDialog } from "./components/PassphraseDialog";
import { EditList } from "./components/EditList";
import { EditForm } from "./components/EditForm";
import { useTunnels } from "./hooks/useTunnels";
import type { TunnelInput } from "./types";

type View =
  | { kind: "normal" }
  | { kind: "edit-list" }
  | { kind: "edit-form"; tunnelId: string | null };

function App() {
  const {
    tunnels,
    loading,
    error,
    connect,
    disconnect,
    passphrasePrompt,
    submitPassphrase,
    cancelPassphrase,
    addTunnel,
    updateTunnel,
    deleteTunnel,
    getTunnelConfig,
  } = useTunnels();

  const [view, setView] = useState<View>({ kind: "normal" });

  const handleSave = async (input: TunnelInput, id: string | null) => {
    if (id) {
      await updateTunnel(id, input);
    } else {
      await addTunnel(input);
    }
    setView({ kind: "edit-list" });
  };

  // Edit list view
  if (view.kind === "edit-list") {
    return (
      <EditList
        tunnels={tunnels}
        onEdit={(id) => setView({ kind: "edit-form", tunnelId: id })}
        onAdd={() => setView({ kind: "edit-form", tunnelId: null })}
        onDelete={deleteTunnel}
        onDone={() => setView({ kind: "normal" })}
      />
    );
  }

  // Edit form view
  if (view.kind === "edit-form") {
    return (
      <EditForm
        tunnelId={view.tunnelId}
        getTunnelConfig={getTunnelConfig}
        onSave={handleSave}
        onBack={() => setView({ kind: "edit-list" })}
      />
    );
  }

  // Normal view
  const connectedCount = tunnels.filter((t) => t.status === "connected").length;
  const totalCount = tunnels.length;

  return (
    <div className="h-screen flex flex-col bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]">
        <div>
          <h1 className="text-sm font-semibold">Tunnel Master</h1>
          <p className="text-xs text-[#999] dark:text-[#666]">
            {connectedCount}/{totalCount} active
          </p>
        </div>
        <button
          onClick={() => setView({ kind: "edit-list" })}
          className="text-[#999] dark:text-[#888] hover:text-[#666] dark:hover:text-[#aaa]"
          title="Edit tunnels"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
            <path d="M2.695 14.763l-1.262 3.154a.5.5 0 00.65.65l3.155-1.262a4 4 0 001.343-.885L17.5 5.5a2.121 2.121 0 00-3-3L3.58 13.42a4 4 0 00-.885 1.343z" />
          </svg>
        </button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 border-l-2 border-red-500 bg-red-500/[0.04] dark:bg-red-500/[0.06] rounded-r">
          <p className="text-xs text-[#dc2626] dark:text-[#f87171]">{error}</p>
        </div>
      )}

      {/* Content — scrollable */}
      <div className="flex-1 overflow-y-auto p-2">
        {loading ? (
          <div className="py-8 text-center">
            <p className="text-[#999] dark:text-[#666] text-sm">Loading...</p>
          </div>
        ) : (
          <TunnelList
            tunnels={tunnels}
            onConnect={connect}
            onDisconnect={disconnect}
          />
        )}
      </div>

      {/* Passphrase dialog */}
      {passphrasePrompt && (
        <PassphraseDialog
          tunnelId={passphrasePrompt.tunnelId}
          onSubmit={submitPassphrase}
          onCancel={cancelPassphrase}
        />
      )}
    </div>
  );
}

export default App;
```

- [ ] **Step 2: Verify build**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Visual check**

Run: `npx tauri dev`
Verify: App renders with themed background. Toggle System Settings → Appearance between Light and Dark — background should switch between `#fafafa` and `#0f0f0f`. Header text, subtitle, edit icon all theme correctly. Error banner shows left red stripe (trigger an error to test).

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx
git commit -m "style: theme App.tsx shell with minimal developer palette"
```

---

### Task 3: TunnelItem — Two-Dot Model + Monospace

**Files:**
- Modify: `src/components/TunnelItem.tsx`

- [ ] **Step 1: Rewrite TunnelItem.tsx**

Replace the entire file with the two-dot model. Key changes:
- Left dot: status indicator (green/gray/amber/red) — not clickable
- Right side: action dot (green=start, red=stop) or text label (connecting/disconnecting)
- Monospace font for port mappings
- Inline error text below ports
- Connected dot gets glow shadow

```tsx
import type { TunnelInfo, TunnelStatus } from "../types";

interface TunnelItemProps {
  tunnel: TunnelInfo;
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

const STATUS_DOT: Record<TunnelStatus, string> = {
  disconnected: "bg-[#d4d4d4] dark:bg-[#333]",
  connecting: "bg-[#fbbf24] animate-pulse",
  connected: "bg-[#4ade80]",
  error: "bg-[#ef4444]",
  disconnecting: "bg-[#fbbf24]",
};

export function TunnelItem({ tunnel, onConnect, onDisconnect }: TunnelItemProps) {
  const isConnected = tunnel.status === "connected";
  const isBusy = tunnel.status === "connecting" || tunnel.status === "disconnecting";

  const handleToggle = () => {
    if (isBusy) return;
    if (isConnected) {
      onDisconnect(tunnel.id);
    } else {
      onConnect(tunnel.id);
    }
  };

  return (
    <div className="flex items-center justify-between px-3 py-2.5 hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] rounded-lg transition-colors">
      <div className="flex items-center gap-3 min-w-0">
        <div
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${STATUS_DOT[tunnel.status]}`}
          style={isConnected ? { boxShadow: "var(--glow-green)" } : undefined}
        />
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{tunnel.name}</div>
          <div className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
            :{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
          </div>
          {tunnel.errorMessage && (
            <div className="text-xs text-[#dc2626] dark:text-[#f87171] truncate mt-0.5" style={{ fontFamily: "var(--font-mono)" }}>
              {tunnel.errorMessage}
            </div>
          )}
        </div>
      </div>

      {isBusy ? (
        <span className="shrink-0 ml-3 text-xs text-[#fbbf24]">
          {tunnel.status === "connecting" ? "connecting" : "disconnecting"}
        </span>
      ) : (
        <button
          onClick={handleToggle}
          className="shrink-0 ml-3 p-1 cursor-pointer"
          title={isConnected ? "Disconnect" : "Connect"}
        >
          <div
            className={`w-2 h-2 rounded-full ${
              isConnected ? "bg-[#f87171]" : "bg-[#4ade80]"
            }`}
          />
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Visual check**

Verify in dev mode: tunnel items show left status dot + right action dot. Port mappings are monospace. Toggle light/dark — colors adapt. Hover shows subtle background.

- [ ] **Step 4: Commit**

```bash
git add src/components/TunnelItem.tsx
git commit -m "style: TunnelItem two-dot model with monospace ports"
```

---

### Task 4: TunnelList — Themed Empty State

**Files:**
- Modify: `src/components/TunnelList.tsx`

- [ ] **Step 1: Update empty state**

Replace the entire file:

```tsx
import type { TunnelInfo } from "../types";
import { TunnelItem } from "./TunnelItem";

interface TunnelListProps {
  tunnels: TunnelInfo[];
  onConnect: (id: string) => void;
  onDisconnect: (id: string) => void;
}

export function TunnelList({ tunnels, onConnect, onDisconnect }: TunnelListProps) {
  if (tunnels.length === 0) {
    return (
      <div className="py-12 text-center">
        <div className="text-3xl opacity-20 mb-2">⇌</div>
        <p className="text-[#999] dark:text-[#666] text-sm">No tunnels configured</p>
        <p className="text-[#bbb] dark:text-[#555] text-xs mt-1">
          Click ✎ to add your first tunnel
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-0.5">
      {tunnels.map((tunnel) => (
        <TunnelItem
          key={tunnel.id}
          tunnel={tunnel}
          onConnect={onConnect}
          onDisconnect={onDisconnect}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Visual check**

Run `npx tauri dev` with no tunnels configured (empty `~/.tunnel-master/config.json` tunnels array). Verify the ⇌ icon, centered text, and correct colors in both light and dark modes.

- [ ] **Step 4: Commit**

```bash
git add src/components/TunnelList.tsx
git commit -m "style: themed empty state with icon for TunnelList"
```

---

## Chunk 2: Edit Views and Passphrase Dialog

### Task 5: EditList — Themed Actions and Empty State

**Files:**
- Modify: `src/components/EditList.tsx`

- [ ] **Step 1: Update EditList.tsx**

Replace the entire file. Key changes:
- Background: `bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5]`
- "Done": primary text, font-weight 600
- "+ Add": secondary text (`text-[#999] dark:text-[#666]`)
- Chevron: `text-[#ccc] dark:text-[#333]`
- Port text: monospace, secondary color
- Row hover: surface color
- Empty state: secondary/tertiary colors (no blue)
- Delete confirmation button: stays red (destructive)

```tsx
import { useState } from "react";
import type { TunnelInfo } from "../types";

interface EditListProps {
  tunnels: TunnelInfo[];
  onEdit: (id: string) => void;
  onAdd: () => void;
  onDelete: (id: string) => Promise<void>;
  onDone: () => void;
}

export function EditList({ tunnels, onEdit, onAdd, onDelete, onDone }: EditListProps) {
  const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const handleMinusClick = (id: string) => {
    setConfirmingDelete(confirmingDelete === id ? null : id);
  };

  const handleDelete = async (id: string) => {
    setDeleting(id);
    try {
      await onDelete(id);
    } finally {
      setDeleting(null);
      setConfirmingDelete(null);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]">
        <button
          onClick={onAdd}
          className="text-sm text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
        >
          + Add
        </button>
        <h1 className="text-sm font-semibold">Edit Tunnels</h1>
        <button
          onClick={onDone}
          className="text-sm font-semibold hover:opacity-80"
        >
          Done
        </button>
      </div>

      {/* Tunnel list */}
      <div className="flex-1 overflow-y-auto p-2">
        {tunnels.length === 0 ? (
          <div className="py-8 text-center">
            <p className="text-[#999] dark:text-[#666] text-sm">No tunnels configured</p>
            <button
              onClick={onAdd}
              className="mt-2 text-[#999] dark:text-[#666] text-sm hover:text-[#666] dark:hover:text-[#999]"
            >
              Add your first tunnel
            </button>
          </div>
        ) : (
          <div className="space-y-0.5">
            {tunnels.map((tunnel) => (
              <div key={tunnel.id} className="flex items-center rounded-lg hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] transition-colors">
                {/* Delete minus button */}
                <button
                  onClick={() => handleMinusClick(tunnel.id)}
                  className="flex-shrink-0 w-8 h-8 flex items-center justify-center ml-1"
                  disabled={deleting === tunnel.id}
                >
                  <div className="w-5 h-5 rounded-full bg-[#dc2626] flex items-center justify-center text-white text-sm font-bold leading-none">
                    &minus;
                  </div>
                </button>

                {/* Tunnel info — clickable to edit */}
                <button
                  onClick={() => onEdit(tunnel.id)}
                  className="flex-1 flex items-center justify-between px-2 py-2.5 text-left"
                >
                  <div className="min-w-0">
                    <p className="text-sm truncate">{tunnel.name}</p>
                    <p className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
                      localhost:{tunnel.localPort} → {tunnel.remoteHost}:{tunnel.remotePort}
                    </p>
                  </div>
                  <span className="text-[#ccc] dark:text-[#333] text-lg ml-2">&rsaquo;</span>
                </button>

                {/* Slide-in delete confirmation */}
                {confirmingDelete === tunnel.id && (
                  <button
                    onClick={() => handleDelete(tunnel.id)}
                    disabled={deleting === tunnel.id}
                    className="flex-shrink-0 bg-red-500 text-white text-xs px-3 py-1.5 rounded-md mr-2 hover:bg-red-600 disabled:opacity-50"
                  >
                    {deleting === tunnel.id ? "..." : "Delete"}
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Visual check**

Run `npx tauri dev`, click the edit icon. Verify: "Done" is primary text weight, "+ Add" is muted, chevrons are subtle, port mappings are monospace. Toggle light/dark. Check empty state by deleting all tunnels.

- [ ] **Step 4: Commit**

```bash
git add src/components/EditList.tsx
git commit -m "style: theme EditList with minimal developer palette"
```

---

### Task 6: EditForm — Themed Form, Toggle, Loading, Error

**Files:**
- Modify: `src/components/EditForm.tsx`

- [ ] **Step 1: Update EditForm.tsx**

Replace the entire file. Key changes:
- Background: themed
- Loading state: themed
- Error banner: left-stripe style
- Section labels: tertiary color
- Sections: surface bg + surface border
- Separators: surface border
- Input text: primary, placeholder: tertiary
- Labels: secondary, 70px width
- Monospace for port/key values
- Toggle: refined smaller size, themed colors
- Save/Back: primary/secondary text (no blue)

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TunnelInput, TunnelConfig } from "../types";

interface EditFormProps {
  tunnelId: string | null;
  getTunnelConfig: (id: string) => Promise<TunnelConfig>;
  onSave: (input: TunnelInput, id: string | null) => Promise<void>;
  onBack: () => void;
}

const emptyForm: TunnelInput = {
  name: "",
  host: "",
  port: 22,
  user: "",
  keyPath: "",
  localPort: 0,
  remoteHost: "",
  remotePort: 0,
  autoConnect: false,
};

export function EditForm({ tunnelId, getTunnelConfig, onSave, onBack }: EditFormProps) {
  const [form, setForm] = useState<TunnelInput>(emptyForm);
  const [loading, setLoading] = useState(!!tunnelId);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (tunnelId) {
      getTunnelConfig(tunnelId)
        .then((config) => {
          setForm({
            name: config.name,
            host: config.host,
            port: config.port,
            user: config.user,
            keyPath: config.keyPath,
            localPort: config.localPort,
            remoteHost: config.remoteHost,
            remotePort: config.remotePort,
            autoConnect: config.autoConnect,
          });
        })
        .catch((e) => setError(String(e)))
        .finally(() => setLoading(false));
    }
  }, [tunnelId, getTunnelConfig]);

  const isValid =
    form.name.trim() !== "" &&
    form.host.trim() !== "" &&
    form.user.trim() !== "" &&
    form.localPort > 0 &&
    form.remoteHost.trim() !== "" &&
    form.remotePort > 0;

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave(form, tunnelId);
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  const updateField = <K extends keyof TunnelInput>(key: K, value: TunnelInput[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  if (loading) {
    return (
      <div className="h-screen flex items-center justify-center bg-[#fafafa] dark:bg-[#0f0f0f]">
        <p className="text-[#999] dark:text-[#666] text-sm">Loading...</p>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]">
        <button
          onClick={onBack}
          className="text-sm text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
        >
          &lsaquo; Back
        </button>
        <h1 className="text-sm font-semibold">
          {tunnelId ? "Edit Tunnel" : "New Tunnel"}
        </h1>
        <button
          onClick={handleSave}
          disabled={!isValid || saving}
          className="text-sm font-semibold disabled:text-[#bbb] dark:disabled:text-[#555] disabled:cursor-not-allowed hover:opacity-80"
        >
          {saving ? "..." : "Save"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 border-l-2 border-red-500 bg-red-500/[0.04] dark:bg-red-500/[0.06] rounded-r">
          <p className="text-xs text-[#dc2626] dark:text-[#f87171]">{error}</p>
        </div>
      )}

      {/* Form */}
      <div className="flex-1 overflow-y-auto px-4 py-3">
        {/* Connection section */}
        <SectionLabel>Connection</SectionLabel>
        <div className="bg-[rgba(0,0,0,0.03)] dark:bg-[rgba(255,255,255,0.04)] rounded-lg overflow-hidden mb-4 border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
          <FormRow label="Name" value={form.name} onChange={(v) => updateField("name", v)} />
          <FormRow label="Host" value={form.host} onChange={(v) => updateField("host", v)} />
          <FormRow
            label="Port"
            value={String(form.port)}
            onChange={(v) => updateField("port", parseInt(v) || 0)}
            type="number"
            mono
          />
          <FormRow label="Username" value={form.user} onChange={(v) => updateField("user", v)} />
          <div className="flex items-center px-3 py-2">
            <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">Key</label>
            <input
              type="text"
              value={form.keyPath}
              onChange={(e) => updateField("keyPath", e.target.value)}
              placeholder="~/.ssh/id_rsa"
              className="flex-1 bg-transparent text-sm outline-none placeholder-[#bbb] dark:placeholder-[#555]"
              style={{ fontFamily: "var(--font-mono)" }}
            />
            <button
              type="button"
              onClick={async () => {
                const path = await invoke<string | null>("pick_key_file");
                if (path) updateField("keyPath", path);
              }}
              className="ml-2 text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] flex-shrink-0"
              title="Browse for key file"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
                <path fillRule="evenodd" d="M3.75 3A1.75 1.75 0 002 4.75v3.26a3.235 3.235 0 011.75-.51h12.5c.644 0 1.245.188 1.75.51V6.75A1.75 1.75 0 0016.25 5h-4.836a.25.25 0 01-.177-.073L9.823 3.513A1.75 1.75 0 008.586 3H3.75zM3.75 9A1.75 1.75 0 002 10.75v4.5c0 .966.784 1.75 1.75 1.75h12.5A1.75 1.75 0 0018 15.25v-4.5A1.75 1.75 0 0016.25 9H3.75z" clipRule="evenodd" />
              </svg>
            </button>
          </div>
        </div>

        {/* Port Forwarding section */}
        <SectionLabel>Port Forwarding</SectionLabel>
        <div className="bg-[rgba(0,0,0,0.03)] dark:bg-[rgba(255,255,255,0.04)] rounded-lg overflow-hidden mb-4 border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
          <FormRow
            label="Local"
            value={form.localPort === 0 ? "" : String(form.localPort)}
            onChange={(v) => updateField("localPort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
            mono
          />
          <FormRow
            label="Remote"
            value={form.remoteHost}
            onChange={(v) => updateField("remoteHost", v)}
            placeholder="e.g. localhost"
            mono
          />
          <FormRow
            label="Port"
            value={form.remotePort === 0 ? "" : String(form.remotePort)}
            onChange={(v) => updateField("remotePort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
            last
            mono
          />
        </div>

        {/* Options section */}
        <SectionLabel>Options</SectionLabel>
        <div className="bg-[rgba(0,0,0,0.03)] dark:bg-[rgba(255,255,255,0.04)] rounded-lg overflow-hidden border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
          <div className="flex items-center justify-between px-3 py-2.5">
            <span className="text-sm text-[#999] dark:text-[#999]">Auto Connect</span>
            <button
              onClick={() => updateField("autoConnect", !form.autoConnect)}
              className={`w-8 h-[18px] rounded-full relative transition-colors ${
                form.autoConnect ? "bg-[#4ade80]" : "bg-[#ccc] dark:bg-[#333]"
              }`}
            >
              <div
                className={`w-[14px] h-[14px] rounded-full absolute top-[2px] transition-transform ${
                  form.autoConnect
                    ? "translate-x-[14px] bg-white"
                    : "translate-x-[2px] bg-white dark:bg-[#888]"
                }`}
              />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-xs text-[#bbb] dark:text-[#555] uppercase tracking-wider mb-1.5 px-1">
      {children}
    </p>
  );
}

interface FormRowProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
  last?: boolean;
  mono?: boolean;
}

function FormRow({ label, value, onChange, type = "text", placeholder, last, mono }: FormRowProps) {
  return (
    <div
      className={`flex items-center px-3 py-2 ${
        last ? "" : "border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]"
      }`}
    >
      <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1 bg-transparent text-sm outline-none placeholder-[#bbb] dark:placeholder-[#555]"
        style={mono ? { fontFamily: "var(--font-mono)" } : undefined}
      />
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Visual check**

Verify: form sections have subtle background, borders adapt to theme. Toggle switch is smaller and themed. Section labels are uppercase tertiary. Port fields show monospace. Error banner uses left-stripe.

- [ ] **Step 4: Commit**

```bash
git add src/components/EditForm.tsx
git commit -m "style: theme EditForm with minimal developer palette and refined toggle"
```

---

### Task 7: PassphraseDialog — Backdrop Blur + Inverted Button

**Files:**
- Modify: `src/components/PassphraseDialog.tsx`

- [ ] **Step 1: Update PassphraseDialog.tsx**

Replace the entire file:

```tsx
import { useState } from "react";

interface PassphraseDialogProps {
  tunnelId: string;
  onSubmit: (passphrase: string) => void;
  onCancel: () => void;
}

export function PassphraseDialog({
  tunnelId,
  onSubmit,
  onCancel,
}: PassphraseDialogProps) {
  const [passphrase, setPassphrase] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (passphrase.trim()) {
      onSubmit(passphrase);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]"
      >
        <h3 className="text-sm font-semibold mb-1">Passphrase Required</h3>
        <p className="text-xs text-[#999] dark:text-[#666] mb-3">
          Enter the passphrase for <span className="text-[#1a1a1a] dark:text-[#e5e5e5]">{tunnelId}</span>'s SSH key.
          It will be stored securely.
        </p>
        <input
          type="password"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          placeholder="SSH key passphrase"
          autoFocus
          className="w-full px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md text-sm placeholder-[#bbb] dark:placeholder-[#555] focus:outline-none focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555] mb-3"
        />
        <div className="flex gap-2 justify-end">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90"
          >
            Unlock
          </button>
        </div>
      </form>
    </div>
  );
}
```

- [ ] **Step 2: Verify build**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src/components/PassphraseDialog.tsx
git commit -m "style: theme PassphraseDialog with backdrop blur and inverted button"
```

---

### Task 8: Final Verification

- [ ] **Step 1: Type check**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 2: Lint**

Run: `npm run lint`
Expected: No errors (fix any that appear)

- [ ] **Step 3: Rust tests** (sanity check — no Rust changes)

Run: `cd /Users/sergiiolyva/ctbto/projects/tunnel-master/src-tauri && cargo test`
Expected: All pass

- [ ] **Step 4: Visual verification**

Run: `npx tauri dev`

Check in **Light mode** (System Settings → Appearance → Light):
- [ ] App background is `#fafafa`, text is dark
- [ ] Tunnel items: left gray dot, right green/red dot
- [ ] Port mappings are monospace
- [ ] Edit list: muted actions, no blue text
- [ ] Edit form: subtle sections, themed toggle
- [ ] Empty state: centered icon + text

Switch to **Dark mode** (System Settings → Appearance → Dark):
- [ ] App background is `#0f0f0f`, text is light
- [ ] Connected dot has green glow
- [ ] Error banner shows left red stripe
- [ ] Passphrase dialog has backdrop blur, inverted button
- [ ] All borders and separators visible but subtle

- [ ] **Step 5: Commit any lint fixes if needed**

```bash
git add -A
git commit -m "style: fix lint issues from theme migration"
```
