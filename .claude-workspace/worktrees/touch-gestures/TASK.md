---
id: touch-gestures
name: Touch and Gesture Support
wave: 3
priority: 3
dependencies: [modal-drag-enhance, window-resize]
estimated_hours: 3
tags: [frontend, ux, touch]
---

## Objective

Add touch-friendly interactions and gesture support for touchscreen and trackpad users.

## Context

While Pia is primarily a desktop app, many users have touchscreens or use trackpad gestures. Adding touch support improves usability on hybrid devices and ensures consistent behavior across input methods.

## Implementation

1. **Touch-friendly tap targets** (`src/modal.css`)
   - Ensure all buttons are at least 44x44px touch target
   - Add padding/margin for touch area without changing visual size
   - Larger hit areas for close and settings buttons

2. **Touch event handling** (`src/main.js`)
   - Add `touchstart`, `touchend` listeners alongside click
   - Prevent double-firing on touch devices
   - Touch feedback (brief highlight on tap)

3. **Swipe gestures**
   - Swipe down on drag handle to minimize/hide
   - Swipe right on settings to close panel
   - Horizontal swipe in action area to scroll history (future)

4. **Pinch-to-resize** (if resize enabled)
   - Two-finger pinch to resize window
   - Integrate with window-resize feature
   - Smooth resize animation during gesture

5. **Long-press actions**
   - Long press on status indicator for detailed info
   - Long press on action text to copy
   - Visual feedback for long-press recognition

6. **Trackpad gesture support**
   - Two-finger scroll in scrollable areas
   - Force touch for quick actions (macOS)
   - Smooth scrolling physics

7. **Touch feedback styles**
   - `-webkit-tap-highlight-color` customization
   - Active state styles for touch
   - Prevent text selection on touch
   - Disable context menu on long press (where appropriate)

## Acceptance Criteria

- [ ] All buttons have 44px minimum touch target
- [ ] Touch events work correctly without double-firing
- [ ] Swipe down on drag handle hides window
- [ ] Long-press shows feedback before action
- [ ] No accidental text selection on touch
- [ ] Smooth scrolling in scrollable areas
- [ ] Touch feedback visible on interaction

## Files to Create/Modify

- `src/modal.css` - Touch target sizes, tap highlight, touch states
- `src/main.js` - Touch event listeners, gesture detection
- `index.html` - Touch-related meta tags if needed

## Integration Points

- **Provides**: Multi-input-method support
- **Consumes**: modal-drag-enhance (drag classes), window-resize (resize state)
- **Conflicts**: May need coordination with native Tauri touch handling
