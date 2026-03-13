# Issue 001: NSPanel dismisses when clicking outside select dropdown

**Priority:** High
**Component:** macOS NSPanel / EditForm

## Problem

When the user opens the jump host `<select>` dropdown and clicks on the window area (outside the dropdown) to close it, the entire NSPanel window disappears. The state is saved, but the window should remain visible — only the dropdown should close.

## Root Cause

The NSPanel `window_did_resign_key` handler in `src-tauri/src/lib.rs:64-69` hides the panel whenever it loses focus. The native `<select>` dropdown creates a separate OS-level popover, so clicking the window to dismiss the dropdown triggers a focus-loss event on the panel itself.

## Possible Fix

- Debounce the `window_did_resign_key` handler to ignore transient focus loss
- Or detect if focus moved to a child popover (the select dropdown) and skip hiding
- Or replace `<select>` with a custom dropdown that doesn't create a separate OS window
