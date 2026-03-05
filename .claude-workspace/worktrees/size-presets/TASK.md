---
id: size-presets
name: Size Presets - Mini, Standard, and Detailed Views
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [frontend, layout, ux]
---

## Objective

Add 2-3 size presets (mini, standard, detailed) that users can quickly switch between, each optimized for different use cases.

## Context

Different workflows benefit from different window sizes. A mini view for monitoring, standard for normal use, and detailed for when users want to see everything. Presets provide quick access to optimized layouts rather than requiring manual resizing.

## Implementation

1. **Define size presets** (`src/main.js`)
   ```javascript
   const SIZE_PRESETS = {
     mini: { width: 300, height: 180, name: 'Mini' },
     standard: { width: 420, height: 280, name: 'Standard' },
     detailed: { width: 550, height: 420, name: 'Detailed' }
   };
   ```

2. **Add size selector UI** (`index.html`, `src/styles/modal.css`)
   - Add size toggle in header (next to settings)
   - Options:
     - Three buttons (S/M/L icons)
     - Or dropdown menu
     - Or cycle button that rotates through sizes

3. **Mini preset layout** (300x180)
   - Minimal: status dot, single-line action, stop button only
   - Hide metrics bar
   - Smaller fonts
   - Compact spacing
   - Ideal for: Monitoring running agent

4. **Standard preset layout** (420x280 - current)
   - Current layout as-is
   - All features visible
   - Ideal for: Normal interaction

5. **Detailed preset layout** (550x420)
   - Expanded action history (multiple lines)
   - Additional metrics (elapsed time, action count)
   - Larger input area
   - More vertical space for content
   - Ideal for: Detailed monitoring/debugging

6. **Apply preset** (`src/main.js`)
   - Use Tauri `window.setSize()` to change dimensions
   - Apply corresponding CSS class to modal
   - Smooth transition animation

7. **Responsive CSS for each preset** (`src/styles/modal.css`)
   ```css
   .modal.size-mini { /* compact styles */ }
   .modal.size-standard { /* default styles */ }
   .modal.size-detailed { /* expanded styles */ }
   ```

8. **Persist selected preset** (`src/main.js`)
   - Save preference in localStorage
   - Restore on app launch

## Acceptance Criteria

- [ ] Size selector visible in UI
- [ ] Three distinct presets work correctly
- [ ] Each preset has optimized layout
- [ ] Transition between presets is smooth
- [ ] All content remains functional at all sizes
- [ ] Selected preset persists across restarts

## Files to Create/Modify

- `index.html` - Add size selector UI elements
- `src/styles/modal.css` - Preset-specific layout styles
- `src/main.js` - Preset logic, size changes, persistence
- `src-tauri/tauri.conf.json` - Ensure min/max allow all presets

## Integration Points

- **Provides**: Quick size switching
- **Consumes**: Tauri window API
- **Conflicts**: Coordinate with expandable-mode (may overlap in purpose)

## Notes

The difference between this and expandable-mode:
- **Size presets**: 3 fixed options with distinct layouts
- **Expandable mode**: Toggle between 2 modes (compact/expanded)

These could be combined or kept separate depending on design preference.
