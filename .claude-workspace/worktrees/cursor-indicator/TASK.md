---
id: cursor-indicator
name: Visual Cursor Indicator Overlay
wave: 1
priority: 3
dependencies: []
estimated_hours: 5
tags: [frontend, backend, ui, tauri]
---

## Objective

Create a visual overlay showing where the agent intends to click before executing the action, especially useful when "confirm dangerous actions" is enabled.

## Context

Users currently have no visual indication of where the agent will click or move the cursor. When dangerous action confirmation is enabled, users see a text description but can't visually verify the target location on screen. A cursor indicator overlay would show the exact pixel coordinates on screen.

## Implementation

### Approach: Separate Overlay Window

Since the main modal is small and positioned in a corner, we need a separate transparent overlay window that covers the entire screen to show cursor indicators.

### Backend Changes (src-tauri/src/)

1. **Modify `src-tauri/tauri.conf.json`**:
   - Add a second window configuration for the cursor overlay:
   ```json
   {
     "label": "cursor-overlay",
     "title": "Cursor Overlay",
     "transparent": true,
     "decorations": false,
     "alwaysOnTop": true,
     "skipTaskbar": true,
     "fullscreen": false,
     "resizable": false,
     "visible": false,
     "url": "cursor-overlay.html"
   }
   ```

2. **Modify `lib.rs`**:
   - Add command to show cursor indicator: `show_cursor_indicator(x, y)`
   - Add command to hide cursor indicator: `hide_cursor_indicator()`
   - These commands control the overlay window visibility and position

3. **Modify `agent/loop_runner.rs`**:
   - Before executing click/move actions, emit cursor position
   - When waiting for confirmation, show the indicator
   - Hide indicator after action execution

### Frontend Changes

4. **Create `cursor-overlay.html`**:
   - Minimal HTML for the overlay window
   - Contains a single cursor indicator element
   - Full-screen transparent background

5. **Create `src/cursor-overlay.js`**:
   - Listen for cursor position events
   - Update indicator position
   - Handle show/hide animations

6. **Create `src/styles/cursor-overlay.css`**:
   - Style the cursor indicator (animated circle/crosshair)
   - Pulsing animation to draw attention
   - Different colors for different action types (click vs move)

### Alternative: Emit Events to Main Window

If creating a separate window is complex, an alternative is to:
- Make the main modal window full-screen but transparent
- Position the UI elements in a corner
- Draw the cursor indicator on the same window

## Acceptance Criteria

- [ ] Visual indicator appears at the target coordinates before actions
- [ ] Indicator is visible even when target is outside the main modal
- [ ] Different visual styles for click vs move actions
- [ ] Indicator shown during confirmation dialog (if enabled)
- [ ] Indicator disappears after action is executed
- [ ] Animation/pulsing effect to make indicator noticeable
- [ ] Works across multiple monitors (uses absolute screen coordinates)

## Files to Create/Modify

- `src-tauri/tauri.conf.json` - Add overlay window config
- `src-tauri/src/lib.rs` - Add cursor indicator commands
- `src-tauri/src/agent/loop_runner.rs` - Emit cursor position before actions
- `cursor-overlay.html` - New overlay window HTML
- `src/cursor-overlay.js` - Overlay window logic
- `src/styles/cursor-overlay.css` - Cursor indicator styles

## Integration Points

- **Provides**: Visual feedback of cursor target location
- **Consumes**: Action coordinates from agent loop
- **Conflicts**: None - new window/functionality

## Technical Notes

- Tauri supports multiple windows with different configurations
- Screen coordinates from actions are absolute (matches screenshot capture)
- The overlay window should be:
  - Transparent background
  - Click-through (doesn't capture mouse events)
  - Always on top but below the main modal
- Consider using CSS `pointer-events: none` for click-through behavior
- The indicator should be prominent but not obstruct the view
- Crosshair or animated ring designs work well for this purpose
