# Issue 002: Connect toggle needs visual feedback for connection attempts

**Priority:** Medium
**Component:** TunnelItem toggle

## Problem

When the user toggles the connect switch, there's no visual feedback indicating a connection attempt is in progress. On success, it's fast enough to not matter. On failure, the toggle stays gray with no indication that anything happened — the user can't tell if it tried and failed or didn't try at all.

## Expected Behavior

- Toggle should animate/pulse during connection attempt
- On failure, toggle should visibly slide back to "off" position
- Brief error indication (color flash, shake animation, or similar)

## Possible Fix

- Add a "connecting" state to the toggle with a subtle animation (e.g., pulsing color)
- On connection failure, animate the toggle sliding back with a brief red flash
- Consider a small spinner overlay on the toggle during connection
