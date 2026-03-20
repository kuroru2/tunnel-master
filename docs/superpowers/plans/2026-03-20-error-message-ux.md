# Error Message UX Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the native hover tooltip on per-tunnel error icons with inline truncated error text that expands on click, enabling copyable error messages with no hover delay.

**Architecture:** Single-component change in `TunnelItem.tsx`. When `tunnel.errorMessage` is present, the port mapping and jump host lines are replaced by clickable red error text (truncated). Clicking expands to show full message with a Copy button. No new components or files needed.

**Tech Stack:** React 19, TypeScript, Tailwind CSS v4, Clipboard API

**Spec:** `docs/superpowers/specs/2026-03-20-error-message-ux-design.md`

**Note on line numbers:** Tasks modify the same file sequentially. Line numbers reference the file state *before that task starts*, not the original file. When in doubt, search for the content strings shown rather than relying on line numbers.

---

### Task 1: Add state, reset logic, and copy handler

**Files:**
- Modify: `src/components/TunnelItem.tsx`

- [ ] **Step 1: Add `errorExpanded` and `copied` state**

Find the line:
```tsx
const [showConnecting, setShowConnecting] = useState(false);
```

Add directly after it:
```tsx
const [errorExpanded, setErrorExpanded] = useState(false);
const [copied, setCopied] = useState(false);
```

(`copied` state drives the Copy button's "Copied!" feedback — implied by the spec's copy behavior.)

- [ ] **Step 2: Add useEffect to reset state when errorMessage changes**

Find the comment `// -- Traffic monitoring --`. Insert *before* that comment:

```tsx
// Reset error expand state when the error message changes
useEffect(() => {
  setErrorExpanded(false);
  setCopied(false);
}, [tunnel.errorMessage]);
```

- [ ] **Step 3: Add handleCopy function**

Find the closing brace of `handleToggle` (the `};` after `onConnect(tunnel.id);`). Insert *before* the `// -- Traffic monitoring --` comment:

```tsx
const handleCopy = () => {
  if (!tunnel.errorMessage || !navigator.clipboard) return;
  navigator.clipboard.writeText(tunnel.errorMessage).then(() => {
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }).catch(() => {});
};
```

(Guard on `navigator.clipboard` handles Tauri WebView contexts where the API may be undefined.)

- [ ] **Step 4: Commit**

```bash
git add src/components/TunnelItem.tsx
git commit -m "feat(error-ux): add expand/collapse state and copy handler"
```

---

### Task 2: Replace port mapping with inline error text and remove warning icon

**Files:**
- Modify: `src/components/TunnelItem.tsx`

- [ ] **Step 1: Replace the inner content block with error-aware rendering**

Find the `<div className="min-w-0">` block that contains the tunnel name, port mapping, and jump host lines. It currently looks like:

```tsx
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{tunnel.name}</div>
          <div className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
            :{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
          </div>
          {tunnel.jumpHostName && (
            <div className="text-xs text-[#999] dark:text-[#555] truncate">
              via {tunnel.jumpHostName}
            </div>
          )}
        </div>
```

Replace the entire block with:

```tsx
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{tunnel.name}</div>
          {tunnel.errorMessage ? (
            <div
              className={`text-xs text-[#dc2626] dark:text-[#f87171] cursor-pointer ${
                errorExpanded ? "leading-relaxed [overflow-wrap:anywhere] select-text" : "truncate"
              }`}
              style={{ fontFamily: "var(--font-mono)" }}
              role="button"
              aria-expanded={errorExpanded}
              onClick={() => setErrorExpanded(!errorExpanded)}
            >
              {tunnel.errorMessage}
              {errorExpanded && (
                <div className="mt-1.5 select-none">
                  <button
                    className="text-[10px] px-1.5 py-0.5 border border-current rounded opacity-40 hover:opacity-70"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleCopy();
                    }}
                  >
                    {copied ? "Copied!" : "Copy"}
                  </button>
                </div>
              )}
            </div>
          ) : (
            <>
              <div className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
                :{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
              </div>
              {tunnel.jumpHostName && (
                <div className="text-xs text-[#999] dark:text-[#555] truncate">
                  via {tunnel.jumpHostName}
                </div>
              )}
            </>
          )}
        </div>
```

(Uses `style={{ fontFamily: "var(--font-mono)" }}` to match the existing codebase convention for monospace — the project defines `--font-mono` as a CSS variable rather than using Tailwind's `font-mono` class.)

- [ ] **Step 2: Delete the warning icon block**

Find and delete the entire block:

```tsx
      {tunnel.errorMessage && (
        <div className="shrink-0 ml-auto text-[#ef4444]" title={tunnel.errorMessage}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-3.5 h-3.5">
            <path fillRule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.168 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 6a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 6zm0 9a1 1 0 100-2 1 1 0 000 2z" clipRule="evenodd"/>
          </svg>
        </div>
      )}
```

- [ ] **Step 3: Add self-start alignment to toggle button when expanded**

Find the toggle button's className. It starts with:

```tsx
        className={`shrink-0 ml-3 w-7 h-4 rounded-full relative transition-colors z-10 ${
          visuallyBusy || recentlyFailed ? "cursor-not-allowed" : "cursor-pointer"
```

Replace that opening portion with:

```tsx
        className={`shrink-0 ml-3 w-7 h-4 rounded-full relative transition-colors z-10 ${
          errorExpanded ? "self-start mt-1 " : ""
        }${
          visuallyBusy || recentlyFailed ? "cursor-not-allowed" : "cursor-pointer"
```

This keeps the toggle at the top of the row when error text expands. The outer flex container stays `items-center` — the status dot and name will center-align which is the intended behavior per the spec.

- [ ] **Step 4: Verify the app compiles**

Run: `cd /Users/sergiiolyva/ctbto/projects/tunnel-master && npm run lint && npx tsc -b`
Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/TunnelItem.tsx
git commit -m "feat(error-ux): inline error text with expand/collapse and copy"
```

---

### Task 3: Manual testing

- [ ] **Step 1: Run the app**

Run: `cd /Users/sergiiolyva/ctbto/projects/tunnel-master && npm run tauri dev`

- [ ] **Step 2: Test all scenarios**

| Scenario | How to trigger | Expected |
|----------|---------------|----------|
| No error | Any disconnected tunnel | Port mapping line shows normally |
| Short error | Connect to tunnel with wrong port | Short red error text inline, no truncation |
| Long error | Connect to tunnel with non-existent hostname | Truncated red error text with ellipsis |
| Expand | Click on truncated error text | Full error message expands, Copy button appears |
| Copy | Click Copy button | Clipboard has error text, button shows "Copied!" for 1.5s |
| Collapse | Click expanded error text | Collapses back to truncated |
| Retry | Toggle tunnel to reconnect | Error clears, port mapping returns |
| Dark mode | Toggle system appearance | Error text uses lighter red (#f87171) |
| Jump host tunnel | Error on a tunnel with jump host | Jump host line hidden during error, returns when cleared |
