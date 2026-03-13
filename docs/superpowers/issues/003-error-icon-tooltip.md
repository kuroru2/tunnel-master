# Issue 003: Error display needs icon with tooltip instead of inline text

**Priority:** Medium
**Component:** TunnelItem error display

## Problem

Inline error messages look scrambled in the small menu bar panel. The text is too long for the available space and disrupts the layout.

## Expected Behavior

- Show a small error icon (e.g., warning triangle or exclamation circle) inline with the tunnel item
- Full error message appears in a tooltip on hover
- Clean, compact presentation that doesn't break the panel layout

## Possible Fix

- Replace inline error `<span>` with an error icon component
- Use CSS `title` attribute or a custom tooltip for the full message
- Icon should use the existing error/warning color from the theme
