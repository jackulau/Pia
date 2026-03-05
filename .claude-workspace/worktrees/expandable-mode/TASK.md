---
id: expandable-mode
name: Expandable Mode - Toggle Between Compact and Expanded View
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [frontend, layout, ux]
---

## Objective

Add a toggle button to switch between compact (current) and expanded view that shows more detail and information.

## Context

The current UI shows a fixed compact view with limited information. Users may want to see more details like full action history, detailed metrics, or expanded status information. An expandable mode provides this without permanently increasing the UI footprint.

## Implementation

1. **Add expand/collapse toggle button** (`index.html`, `src/main.js`)
   - Add toggle button in header area (next to settings button)
   - Use expand/collapse icon (chevron or maximize icon)
   - Toggle between `expanded` and `compact` CSS classes on modal

2. **Define compact mode layout** (current default)
   - Current 420x280px dimensions
   - Single-line action display
   - Minimal metrics (3 columns)
   - Input + send button

3. **Define expanded mode layout** (`src/styles/modal.css`)
   - Window size: 500x450px (or larger)
   - Multi-line action history (last 5-10 actions)
   - Additional metrics:
     - Elapsed time
     - Actions performed count
     - Current step in iteration
   - Instruction input with more space
   - Optional: Show thinking/reasoning preview

4. **Animate transition** (`src/styles/modal.css`)
   - Smooth CSS transition between sizes
   - Use Tauri `window.setSize()` to resize window
   - Content fades/slides during transition

5. **Persist expanded state** (`src/main.js`)
   - Save preference in localStorage
   - Restore on app launch

## Acceptance Criteria

- [ ] Toggle button visible in header
- [ ] Click toggles between compact and expanded view
- [ ] Expanded view shows additional information
- [ ] Transition is smooth and animated
- [ ] State persists across app restarts
- [ ] All content readable and properly styled in both modes

## Files to Create/Modify

- `index.html` - Add toggle button, expanded view containers
- `src/styles/modal.css` - Compact/expanded layout styles
- `src/main.js` - Toggle logic, size change, persistence
- `src-tauri/tauri.conf.json` - May need to adjust maxWidth/maxHeight

## Integration Points

- **Provides**: User-selectable view mode
- **Consumes**: Tauri window API for resizing
- **Conflicts**: Coordinate with size-presets if both are implemented
