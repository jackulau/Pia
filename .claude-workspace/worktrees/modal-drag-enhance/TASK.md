---
id: modal-drag-enhance
name: Enhanced Modal Dragging with Visual Feedback
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [frontend, ux, interaction]
---

## Objective

Enhance the modal dragging experience with visual feedback, bounds checking, and smooth position persistence.

## Context

The current drag handle uses Tauri's native `-webkit-app-region: drag` which provides basic dragging but lacks visual feedback during drag operations. Users need clear indication when dragging is active and the window should remember its position.

## Implementation

1. **Enhance drag handle visual feedback** (`index.html`, `src/modal.css`)
   - Add active/dragging state styles with opacity change
   - Show grab cursor on hover, grabbing cursor during drag
   - Add subtle scale transform during drag (1.02)
   - Highlight drag handle bar during active drag

2. **Add drag state detection** (`src/main.js`)
   - Listen for mousedown/mouseup on drag region
   - Toggle `.dragging` class on modal during drag
   - Add subtle shadow/glow effect during drag

3. **Position persistence** (optional enhancement)
   - Store window position in localStorage or Tauri store
   - Restore position on app launch
   - Add reset position option in settings

4. **Drag bounds awareness**
   - Ensure window stays partially visible on screen
   - Add visual feedback when hitting screen edges

## Acceptance Criteria

- [ ] Drag handle shows grab cursor on hover
- [ ] Drag handle shows grabbing cursor during drag
- [ ] Modal has visual feedback (opacity/shadow) during drag
- [ ] Drag handle bar becomes more visible during drag
- [ ] Smooth transitions between states
- [ ] No interference with existing Tauri drag functionality

## Files to Create/Modify

- `index.html` - Add dragging state classes
- `src/modal.css` - Add .dragging styles, cursor states
- `src/main.js` - Add drag state detection listeners

## Integration Points

- **Provides**: Enhanced drag UX foundation
- **Consumes**: Existing Tauri drag region
- **Conflicts**: None - builds on existing drag implementation
