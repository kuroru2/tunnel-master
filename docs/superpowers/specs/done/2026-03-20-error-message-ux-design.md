# Error Message UX Redesign

## Problem

Per-tunnel error messages use a native HTML `title` tooltip on a warning icon. This has two issues:
1. **Can't copy text** — native tooltips are not selectable
2. **Hover delay** — OS-imposed wait before tooltip appears

## Solution: Inline Truncated + Expand

Replace the warning icon + hover tooltip with inline error text that replaces the port mapping line when a tunnel is in an error state.

## States

### No Error (unchanged)
Normal tunnel row with status dot, name, port mapping line, optional jump host line, and toggle.

### Error — Collapsed (default)
- The port mapping line (`:5432 → db.prod:5432`) and jump host line (`via jumphost`) are **replaced** by the error message in red monospace text
- Text is truncated to one line with CSS `text-overflow: ellipsis` via Tailwind's `truncate` class
- Short errors (e.g. "Connection lost") that fit in one line display fully with no truncation
- The entire error text line is clickable to expand
- Status dot shows red (`bg-[#ef4444]`)
- The warning triangle icon is **removed** — the red error text is the indicator
- No conflict with traffic chart area: `showTraffic` is only true when `tunnel.status === "connected"`, and errors only appear when disconnected/errored, so they are mutually exclusive

### Error — Expanded (after click on error text)
- Error text wraps to show the full message
- A small "Copy" button appears below the error text
- Clicking the Copy button copies the full error message to the clipboard
- Clicking the error text area again collapses back to truncated state
- The error text is user-selectable (`user-select: text`) for manual selection/copy
- Toggle button uses `self-start` to stay at the top of the expanded row (outer flex container remains `items-center`)

### Error Clears
When tunnel status changes (reconnect attempt, successful connection, etc.), the error message clears and the port mapping line returns. This already happens via the existing `tunnel-status-changed` event flow — no change needed.

## Component Changes

### TunnelItem.tsx

**New state:**
- `const [errorExpanded, setErrorExpanded] = useState(false)` — tracks expand/collapse

**Reset on error change:**
- `errorExpanded` resets to `false` when `tunnel.errorMessage` changes (useEffect). This is intentional — if a retry produces a different error, the user sees the new message in collapsed state first.

**Render logic:**
- Error text renders whenever `tunnel.errorMessage` is not null, regardless of `tunnel.status`. In practice these are mutually exclusive with connected state, but the check is on `errorMessage` presence, not status.
- When `tunnel.errorMessage` is present:
  - Hide the port mapping line (`:localPort → remoteHost:remotePort`)
  - Hide the jump host line (`via jumpHostName`)
  - Remove the warning icon `<div>` entirely
  - Show error text in its place:
    - Collapsed: single line, `truncate` (Tailwind), monospace, `text-[#dc2626] dark:text-[#f87171]`, `cursor-pointer`, `onClick` toggles `errorExpanded`
    - Expanded: word-wrap with `[overflow-wrap:anywhere]` (handles long unbreakable tokens like URLs), `user-select: text`, same colors, Copy button below
- When `tunnel.errorMessage` is null: render port mapping + jump host as before (no changes)

**Copy button:**
- Uses `navigator.clipboard.writeText(tunnel.errorMessage)`
- On success: text changes to "Copied!" for ~1.5s, then reverts
- On failure (clipboard API unavailable): silently ignore — the text is already user-selectable for manual copy as a fallback

**Styling:**
- Error text: `text-xs font-mono text-[#dc2626] dark:text-[#f87171]`
- Copy button: `text-[10px] px-1.5 py-0.5 border border-current rounded opacity-40 hover:opacity-70` (border inherits the red text color in both light/dark modes via `border-current` — intentional)
- Expanded text: `leading-relaxed [overflow-wrap:anywhere]`

## What's NOT Changing
- Global error banner in App.tsx (separate scope)
- EditForm error banner (separate session)
- Modal dialogs (passphrase, host key, password, keyboard-interactive)
- Status dot colors and animations (shake, flash-red)
- Toggle button behavior
- Traffic sparkline display

## Accessibility
- Error text is in the DOM (not a tooltip), so screen readers can read it
- Clickable error text uses `role="button"` and `aria-expanded={errorExpanded}` for screen reader interactivity
- Copy button is a real `<button>` element
- `cursor-pointer` provides visual affordance for clickable area
