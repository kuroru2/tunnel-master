# Minimal Developer Theme + System Dark/Light Mode

## Goal

Replace the hardcoded dark theme with a "Minimal Developer" visual style that automatically follows the macOS system appearance (light/dark). Polish all UI components for consistency, clarity, and a refined developer-tool aesthetic.

## Architecture

- **Theme detection:** CSS `prefers-color-scheme` media query. Tailwind v4 uses media-query dark mode by default — no configuration needed. The `dark:` variant works out of the box.
- **Color strategy:** Near-monochrome palette. Color is reserved for status indicators (green/red/yellow dots) and destructive actions. All chrome is grayscale.
- **Implementation:** Tailwind `dark:` utility classes on every themed element. CSS custom properties in `index.css` for values that benefit from centralized definition (e.g., backdrop blur, box-shadow glow).

## Color Palette

### Dark Mode (system dark)

| Role | Value | Usage |
|------|-------|-------|
| Background | `#0f0f0f` | App background |
| Surface | `rgba(255,255,255,0.04)` | Grouped form sections, hover states |
| Text primary | `#e5e5e5` | Headings, tunnel names, primary actions (Done, Save) |
| Text secondary | `#666` | Labels, muted text, back/add links |
| Text tertiary | `#555` | Section labels, placeholders |
| Border | `rgba(255,255,255,0.06)` | Separators, card edges |
| Elevated surface | `#1a1a1a` | Passphrase dialog background |

### Light Mode (system light)

| Role | Value | Usage |
|------|-------|-------|
| Background | `#fafafa` | App background |
| Surface | `rgba(0,0,0,0.03)` | Grouped form sections, hover states |
| Text primary | `#1a1a1a` | Headings, tunnel names, primary actions |
| Text secondary | `#999` | Labels, muted text |
| Text tertiary | `#bbb` | Section labels, placeholders |
| Border | `rgba(0,0,0,0.06)` | Separators, card edges |
| Elevated surface | `#ffffff` | Passphrase dialog background |

### Shared (both modes)

| Role | Value |
|------|-------|
| Status dot: connected | `#4ade80` (green) with glow `0 0 4px rgba(74,222,128,0.4)` |
| Status dot: disconnected | `#333` dark / `#d4d4d4` light |
| Status dot: connecting | `#fbbf24` (amber, pulsing animation) |
| Status dot: error | `#ef4444` (red) |
| Action dot: start (disconnected/error) | `#4ade80` (green) |
| Action dot: stop (connected) | `#f87171` (red) |
| Destructive (delete circle) | `#dc2626` |
| Error text | `#f87171` dark / `#dc2626` light |
| Focus ring | `ring-1 ring-[#555]` dark / `ring-1 ring-[#bbb]` light (tertiary color) |

## Component Specifications

### 1. TunnelItem (main list item)

**Current:** Colored pill buttons ("Start"/"Stop"), system font for ports.

**New — two-dot model:**
- **Left dot** (status indicator, not clickable):
  - Connected: `#4ade80` green with subtle glow
  - Disconnected: `#333` dark / `#d4d4d4` light (gray)
  - Connecting: `#fbbf24` amber with CSS pulse animation
  - Disconnecting: `#fbbf24` amber (no pulse)
  - Error: `#ef4444` red

- **Right side** (action area, clickable):
  - Connected: red dot `#f87171` — click to disconnect
  - Disconnected: green dot `#4ade80` — click to connect
  - Error: green dot `#4ade80` — click to retry
  - Connecting: text "connecting" in amber, disabled (no click)
  - Disconnecting: text "disconnecting" in amber, disabled (no click)

- Port mappings use **monospace** font (`font-family: ui-monospace, SFMono-Regular, monospace`)
- Hover state: surface background color
- Error message shows **inline** below the port mapping line in error color

### 2. App.tsx (main view)

**Current:** `bg-gray-900 text-white`

**New:**
- Background: `bg-[#fafafa] dark:bg-[#0f0f0f]`
- Text: `text-[#1a1a1a] dark:text-[#e5e5e5]`
- Header border: `border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]`
- "Tunnel Master" title: primary text color (inherited)
- Subtitle ("2/3 active"): `text-[#999] dark:text-[#666]`
- Edit icon: `text-[#999] dark:text-[#888]` (muted, not blue)

### 3. Error Banner (applies to App.tsx AND EditForm.tsx)

**Current:** Red background box with red border (`bg-red-500/20 border border-red-500/30`).

**New (used everywhere an error banner appears):**
- Left-border accent stripe: `border-l-2 border-red-500`
- Background: `bg-red-500/[0.04] dark:bg-red-500/[0.06]`
- No outer border — just the left stripe
- Rounded right corners only: `rounded-r`
- Text: error color (`text-[#dc2626] dark:text-[#f87171]`)

### 4. EditList

**Current:** Blue text for Add/Done links, `bg-gray-900` background.

**New:**
- Background: same as App (bg/text themed)
- "Done" button: primary text color, font-weight 600 (stands out as the primary action)
- "+ Add" button: secondary text color (`#999` / `#666`)
- Delete minus circle: stays `#dc2626` (visible in both themes)
- Delete confirmation button ("Delete"): stays `bg-red-500 text-white` (destructive action, intentionally colored)
- Chevron (›): `text-[#ccc] dark:text-[#333]`
- Port mapping text: monospace, secondary color
- Row hover: surface color
- **Empty state:** "No tunnels configured" in secondary color, "Add your first tunnel" link in secondary color (not blue), matching the minimal style

### 5. EditForm

**Current:** Blue Back/Save links, `bg-white/5` sections, `border-white/5` separators, `bg-gray-900` background.

**New:**
- Background: same as App (bg/text themed)
- **Loading state** (when fetching tunnel config): themed background (`bg-[#fafafa] dark:bg-[#0f0f0f]`), secondary text color for "Loading..."
- "Save" button: primary text color, font-weight 600
- "‹ Back" button: secondary text color
- Disabled Save: tertiary text color
- Section labels: tertiary color, uppercase, tracking-wider (keep current style, adjust colors)
- Grouped sections: surface background with surface border
- Separators within groups: same surface border
- Input text: primary text color
- Placeholder text: tertiary color
- Label text: secondary color, slightly narrower width (70px instead of 96px/w-24)
- Port/key path values: monospace font
- Key file browse icon: secondary color
- **Error banner:** Same new error banner style as Section 3
- Auto-connect toggle: refined sizing
  - Off: `bg-[#ccc] dark:bg-[#333]` track, `bg-white dark:bg-[#888]` knob
  - On: `bg-[#4ade80]` track, `bg-white` knob (both modes)
  - Smaller: `w-8 h-[18px]` track, `w-[14px] h-[14px]` knob

### 6. PassphraseDialog

**Current:** Flat dark overlay, gray-800 background.

**New:**
- Backdrop: `bg-black/60` with `backdrop-blur-sm` (4px blur)
- Dialog background: elevated surface (`bg-[#ffffff] dark:bg-[#1a1a1a]`)
- Dialog border: surface border color, `rounded-xl` (10px)
- Title: primary text, semibold
- Description: secondary text. Keep `tunnelId` in the text (no prop change needed). Remove "macOS Keychain" mention — just say "It will be stored securely."
- Input: background color, surface border, `rounded-md`
- Input focus: `focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555]` (tertiary color ring, no blue)
- "Cancel" button: secondary text, no background
- "Unlock" button: **inverted** — `bg-[#1a1a1a] text-[#fafafa] dark:bg-[#e5e5e5] dark:text-[#0f0f0f]`, `rounded-md`, font-weight 500
- Button text: "Unlock" (remove "& Connect")

### 7. Empty State (TunnelList)

**Current:** Plain text "No tunnels configured" + config file path.

**New:**
- Centered vertically in scroll area
- Subtle icon: `⇌` character at ~28px, opacity 0.2
- Primary text: "No tunnels configured" in secondary color
- Secondary text: "Click ✎ to add your first tunnel" in tertiary color
- No config file path reference (users should use the UI)

### 8. Loading State (App.tsx)

**Current:** Plain "Loading..." text.

**New:** Same text, themed secondary text color.

## Files Changed

| File | Action | Purpose |
|------|--------|---------|
| `src/index.css` | Modify | Add CSS custom properties for glow shadow, backdrop blur |
| `src/App.tsx` | Modify | Theme all hardcoded colors, update error banner |
| `src/components/TunnelItem.tsx` | Modify | Two-dot model, monospace ports, theme colors |
| `src/components/TunnelList.tsx` | Modify | Update empty state |
| `src/components/EditList.tsx` | Modify | Theme colors, adjust action styling, theme empty state |
| `src/components/EditForm.tsx` | Modify | Theme form, refine toggle, error banner, loading state |
| `src/components/PassphraseDialog.tsx` | Modify | Backdrop blur, inverted button, themed colors, focus ring |

**Note:** No `tailwind.config.ts` needed — Tailwind v4 uses media-query dark mode by default.

## Testing

- Toggle macOS appearance in System Settings → Appearance to verify live switching
- Verify all five tunnel states render correctly in both themes: disconnected, connecting, connected, disconnecting, error
- Verify error banner (left-stripe style) in both App.tsx and EditForm.tsx, both themes
- Verify edit list empty state renders correctly in both themes
- Verify edit form loading state is themed (not hardcoded dark)
- Verify edit form is readable and usable in both themes
- Verify passphrase dialog backdrop blur works
- Verify passphrase dialog input focus ring (tertiary color, not blue)
- Verify empty state renders centered with icon
- Run `npx tsc --noEmit` — no type errors
- Run `npm run lint` — no lint errors
- Run `cd src-tauri && cargo test` — all pass (no Rust changes)

## Out of Scope

- User toggle for theme preference (follow system only)
- Animations beyond the connecting pulse
- Custom app icon variants for light/dark
- Linux/Windows theme detection (works via CSS media query on all platforms, but appearance may vary)
- Adding `tunnelName` prop to PassphraseDialog (keep using `tunnelId` for now)
