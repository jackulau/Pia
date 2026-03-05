---
id: window-resize
name: Window Resize Support with Responsive Layout
wave: 1
priority: 2
dependencies: []
estimated_hours: 4
tags: [frontend, layout, ux]
---

## Objective

Enable window resizing with responsive layout that adapts gracefully to different sizes while maintaining usability.

## Context

The window is currently fixed at 420x280px with `resizable: false`. Users may want to expand the window to see more content or minimize it for less intrusion. The UI should respond gracefully to size changes.

## Implementation

1. **Enable window resizing** (`src-tauri/tauri.conf.json`)
   - Set `resizable: true`
   - Keep `minWidth: 300`, `minHeight: 200`
   - Add `maxWidth: 600`, `maxHeight: 500` to prevent excessive sizing

2. **Add resize handle** (`index.html`, `src/modal.css`)
   - Add visual resize grip in bottom-right corner
   - Style with subtle diagonal lines indicator
   - Use `resize: both` CSS or custom resize handle
   - Ensure resize handle doesn't interfere with content

3. **Responsive layout adjustments** (`src/modal.css`)
   - Use CSS Grid or Flexbox for fluid layout
   - Action display area should grow with window height
   - Metrics bar should adapt spacing
   - Input area maintains proportion

4. **Responsive breakpoints**
   - Small (300-350px width): Compact mode, smaller fonts
   - Medium (350-450px): Default layout
   - Large (450-600px): Expanded action display, more metrics

5. **Settings panel responsiveness**
   - Scroll if content exceeds available height
   - Maintain padding and spacing ratios

6. **Persist window size**
   - Store preferred size in config
   - Restore on app launch

## Acceptance Criteria

- [ ] Window can be resized by dragging edges/corners
- [ ] Minimum size enforced (300x200)
- [ ] Maximum size enforced (600x500)
- [ ] Layout adapts smoothly to different sizes
- [ ] All content remains visible and usable at all sizes
- [ ] Resize grip visible in corner
- [ ] Window size persists across app restarts

## Files to Create/Modify

- `src-tauri/tauri.conf.json` - Enable resizing, set bounds
- `index.html` - Add resize handle element
- `src/modal.css` - Responsive styles, resize handle styling
- `src/main.js` - Size persistence logic

## Integration Points

- **Provides**: Flexible window sizing
- **Consumes**: Tauri window API
- **Conflicts**: None - orthogonal to other features
