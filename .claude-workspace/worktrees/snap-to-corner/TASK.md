---
id: snap-to-corner
name: Snap to Corner - Quick Position Presets
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [frontend, layout, ux]
---

## Objective

Add quick position presets to snap the window to screen corners or edges (top-left, top-right, bottom-left, bottom-right, center).

## Context

Users may want to quickly position the always-on-top window to a preferred location without manually dragging it. Corner snapping provides quick access to common positions, especially useful on large monitors or multi-monitor setups.

## Implementation

1. **Add position menu/buttons** (`index.html`, `src/main.js`)
   - Add a position button in header (grid/position icon)
   - On click, show dropdown with position options:
     - Top Left
     - Top Right
     - Bottom Left
     - Bottom Right
     - Center
   - Or use keyboard shortcuts (optional)

2. **Implement snap positions** (`src/main.js`)
   - Use Tauri's `window.setPosition()` API
   - Get screen dimensions via Tauri or JavaScript
   - Calculate corner positions with padding (e.g., 20px from edge)
   - Account for window size when positioning

3. **Position calculation** (with example for 420x280 window):
   ```javascript
   const PADDING = 20;
   const positions = {
     'top-left': { x: PADDING, y: PADDING },
     'top-right': { x: screenWidth - windowWidth - PADDING, y: PADDING },
     'bottom-left': { x: PADDING, y: screenHeight - windowHeight - PADDING },
     'bottom-right': { x: screenWidth - windowWidth - PADDING, y: screenHeight - windowHeight - PADDING },
     'center': { x: (screenWidth - windowWidth) / 2, y: (screenHeight - windowHeight) / 2 }
   };
   ```

4. **Visual feedback** (`src/styles/modal.css`)
   - Highlight current position in dropdown
   - Brief animation on snap (subtle scale or glow)

5. **Persist last position** (`src/main.js`)
   - Save selected position preset in localStorage
   - Restore on app launch

6. **Multi-monitor support** (optional enhancement)
   - Detect available monitors
   - Allow snapping to corners of each monitor

## Acceptance Criteria

- [ ] Position button/menu accessible in UI
- [ ] All 5 positions work correctly (4 corners + center)
- [ ] Window snaps immediately on selection
- [ ] Position accounts for window size
- [ ] Works correctly on different screen sizes
- [ ] Last position preference persists

## Files to Create/Modify

- `index.html` - Add position button and dropdown menu
- `src/styles/modal.css` - Dropdown styles, position menu
- `src/main.js` - Position logic, Tauri window API calls

## Integration Points

- **Provides**: Quick window positioning
- **Consumes**: Tauri window position API, screen dimensions
- **Conflicts**: None - works independently of other features
