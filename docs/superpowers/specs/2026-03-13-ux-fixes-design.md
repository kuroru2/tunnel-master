# UX Fixes Design Spec

Three targeted fixes for UX issues discovered during auth-methods user testing.

## Issue 1: NSPanel Dismisses on Jump Host Dropdown Click

**Problem:** The native `<select>` element for the jump host field creates an OS-level popover. Clicking the panel window to close this popover triggers `window_did_resign_key`, which hides the entire NSPanel.

**Solution:** Replace the native `<select>` with a custom React dropdown component that renders entirely within the webview DOM. No OS-level popover means no focus loss on the panel.

### Custom Dropdown Component

Create a `CustomSelect` component in `src/components/CustomSelect.tsx`:

- A trigger element showing the current value with a chevron indicator
- An absolutely-positioned dropdown menu that opens on click
- Click-outside detection to close the dropdown (using a `useEffect` with document click listener)
- Keyboard support: Escape to close
- Styled to match the existing form row aesthetic (transparent background, 13px text, same colors)
- The dropdown menu uses `position: absolute` within the form row, with white/dark background, subtle border, and shadow
- Items highlight on hover; selected item gets a subtle background

**Props interface:**
```typescript
interface CustomSelectProps {
  value: string | null;
  onChange: (value: string | null) => void;
  options: Array<{ value: string; label: string }>;
  placeholder?: string;
}
```

**Integration:** Replace the `<select>` in `EditForm.tsx` line 179-185 with `<CustomSelect>`. The parent builds the options array from the tunnels list (filtering out current tunnel).

## Issue 2: Connection Toggle Feedback

**Problem:** Connection attempts succeed or fail too quickly for the user to perceive the "connecting" state. On failure, the toggle stays gray — no visible indication that an attempt was made.

**Solution:** Add a minimum visible duration for the connecting state and a failure animation on the toggle.

### Minimum Visible Duration

In `useTunnels.ts`, the `connect` function currently calls `invoke("connect_tunnel")` and either succeeds or throws. The backend already transitions the tunnel to "connecting" status and emits a `tunnel-status-changed` event, but the subsequent success/error event follows too quickly.

Rather than adding artificial delay to the backend, handle this in `TunnelItem.tsx`:

- Track a local `recentlyFailed` state (boolean, auto-clears after 600ms)
- Track a local `connectingMinVisible` state to ensure the connecting appearance stays for at least 400ms
- When the tunnel transitions from "connecting" to "error" or "disconnected" (detected via prop change), set `recentlyFailed = true` and start a 600ms timer to clear it
- When the tunnel transitions to "connecting", record the timestamp. When it transitions away, if less than 400ms elapsed, keep showing connecting appearance for the remainder

### Failure Animation

When `recentlyFailed` is true:
- Apply a CSS shake animation (0.4s) to the toggle
- Flash the toggle track briefly red before returning to gray
- Both animations are CSS `@keyframes`, applied via conditional class names

### CSS additions in `TunnelItem.tsx` (inline or via Tailwind arbitrary values):

```css
@keyframes shake {
  0%, 100% { transform: translateX(0); }
  20% { transform: translateX(-3px); }
  40% { transform: translateX(3px); }
  60% { transform: translateX(-2px); }
  80% { transform: translateX(2px); }
}

@keyframes flash-red {
  0% { background-color: #ef4444; }
  100% { background-color: #ccc; }
}
```

These keyframes go in `src/index.css` (alongside existing `--glow-green`).

## Issue 3: Error Icon with Tooltip

**Problem:** Inline error messages are too long for the small menu bar panel and disrupt the layout.

**Solution:** Replace inline error text with a warning triangle icon. Full error message appears in a native tooltip on hover (using the `title` attribute for simplicity — no custom tooltip library needed).

### Changes to `TunnelItem.tsx`

Remove the error text `<div>` (lines 47-51). Add a warning triangle SVG icon between the tunnel info and the toggle button:

- Icon is 14x14px, colored `#ef4444` (matching existing error red)
- Uses `title` attribute with the full error message for native tooltip
- Icon only renders when `tunnel.errorMessage` is truthy
- Positioned with `ml-auto` or in the flex row between info and toggle

The icon uses the same SVG as shown in the mockup (Heroicons exclamation-triangle, `viewBox="0 0 20 20"`).

### Layout adjustment

Current structure: `[dot + info] ... [toggle]`

New structure: `[dot + info] ... [error-icon?] [toggle]`

The error icon sits to the left of the toggle, separated by a small gap. This keeps the toggle in its consistent position while the icon appears/disappears cleanly.

## Files Changed

| File | Change |
|------|--------|
| `src/components/CustomSelect.tsx` | New — reusable dropdown component |
| `src/components/EditForm.tsx` | Replace `<select>` with `<CustomSelect>` |
| `src/components/TunnelItem.tsx` | Add error icon, toggle feedback animations, local state for min-visible + failure |
| `src/index.css` | Add `@keyframes shake` and `@keyframes flash-red` |

## What's NOT Changing

- No backend changes — all fixes are frontend-only
- No new dependencies
- No changes to types, hooks, or other components
- The `useTunnels` hook stays unchanged — all timing logic is local to `TunnelItem`
