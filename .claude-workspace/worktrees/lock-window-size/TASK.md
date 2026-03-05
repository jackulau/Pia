---
id: lock-window-size
name: Lock Window to Fixed Size and Remove Resize Capabilities
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [frontend, tauri-config, cleanup]
---

## Objective

Make the Pia window a fixed, non-resizable size that displays the full UI on launch — no manual stretching or widening allowed.

## Context

Currently the app window is resizable (`resizable: true` in tauri.conf.json) with min/max constraints, an S/M/L size selector, an expanded mode toggle, and a resize handle. The user wants a single fixed-size window that shows the complete UI. All manual and preset-based resizing should be removed. Programmatic resize will still be needed for the minimize mode feature (handled by a separate task).

## Implementation

### 1. Tauri Config (`src-tauri/tauri.conf.json`)
- Set `"resizable": false` on the main window
- Remove `minWidth`, `minHeight`, `maxWidth`, `maxHeight` (not needed when non-resizable)
- Set `width` and `height` to a size that properly displays the full UI (recommend ~420x450 or test to find the right fit — should show: drag handle, header, template row, input area, metrics bar, and have room for action display/timeline when agent is running)

### 2. Remove Resize Handle from HTML (`index.html`)
- Remove the `.resize-handle` div (around line 2203)
- Remove all `.resize-handle` CSS rules from the inline `<style>` block (around lines 1660-1693)

### 3. Remove Size Selector (S/M/L) from HTML (`index.html`)
- Remove the `.size-selector` container and its 3 buttons from `.header-actions` in the HTML
- Remove all `.size-selector`, `.size-btn`, `.size-mini`, `.size-standard`, `.size-detailed` CSS rules from the inline `<style>` block

### 4. Remove Expanded Mode Toggle from HTML (`index.html`)
- Remove the expand button (`.expand-btn`) from `.header-actions`
- Remove `.expanded-only` CSS rules and `.modal.expanded` CSS rules from the inline `<style>` block
- Remove `expanded-only` class from any HTML elements (the `.action-history` div uses it)
- Make the action history always visible (remove the `expanded-only` class so it renders normally)

### 5. Clean Up JavaScript (`src/main.js`)
- Remove `SIZE_PRESETS` constant (lines 8-12)
- Remove `COMPACT_SIZE` and `EXPANDED_SIZE` constants (lines 170-171)
- Remove `isExpanded` state variable (line 146) and `currentSizePreset` (line 158)
- Remove `setupResizeListener()` function and its call in `init()`
- Remove `saveWindowSize()` and `restoreWindowSize()` functions and their calls in `init()`
- Remove `applySizePreset()`, `setupSizeSelector()`, `loadSavedSizePreset()` functions and their calls in `init()`
- Remove `toggleExpandedMode()`, `applyExpandedState()`, `restoreExpandedState()` functions and their calls in `init()`
- Remove related DOM element references (`expandBtn`, size selector buttons, resize handle)
- Clean up any event listeners tied to removed elements
- Remove related localStorage usage: `pia-window-size`, `pia-size-preset`, `pia-expanded-mode`

### 6. Verify
- Build and run the app (`npm run tauri dev`)
- Confirm the window opens at the correct fixed size showing all UI elements
- Confirm the window CANNOT be resized by dragging edges/corners
- Confirm S/M/L buttons and expand button are gone
- Confirm no console errors

## Acceptance Criteria

- [ ] Window opens at a fixed size displaying the full UI
- [ ] Window cannot be manually resized (no drag-to-resize)
- [ ] Resize handle is removed from the UI
- [ ] S/M/L size selector buttons are removed
- [ ] Expanded mode toggle button is removed
- [ ] No JavaScript errors in console related to removed features
- [ ] Window can still be dragged via the drag handle
- [ ] Settings panel still opens and closes correctly
- [ ] Close button and kill switch still work

## Files to Create/Modify

- `src-tauri/tauri.conf.json` - Set resizable: false, remove min/max constraints, set fixed dimensions
- `index.html` - Remove resize handle HTML/CSS, size selector HTML/CSS, expanded mode HTML/CSS
- `src/main.js` - Remove resize/size/expanded JS functions, constants, event listeners, localStorage

## Integration Points

- **Provides**: Fixed-size window that other features (minimize mode) can programmatically resize
- **Consumes**: None (foundational change)
- **Conflicts**: Do NOT remove `appWindow.setSize()` import — the minimize mode task needs it for programmatic resize. Do NOT touch the `.input-area`, `.drag-handle`, or `.modal-header` structure — the minimize mode task depends on those.
