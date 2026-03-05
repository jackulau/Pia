---
id: windows-overlay-clickthrough
name: Fix overlay click-through for Windows
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [backend, windows, overlay]
---

## Objective

Make the overlay window click-through on Windows, matching the existing macOS behavior.

## Context

The app has overlay windows (coordinate overlay + cursor indicator overlay) that display visual feedback on top of the screen. On macOS, `set_ignore_cursor_events(true)` is called to make these click-through, but this is wrapped in `#[cfg(target_os = "macos")]` and **never runs on Windows**. This means on Windows, the overlay would intercept all mouse clicks, making the app unusable.

## Implementation

1. In `src-tauri/src/lib.rs` around line 1052, extend the click-through logic to also work on Windows:
   ```rust
   // Currently macOS-only:
   #[cfg(target_os = "macos")]
   {
       if let Err(e) = overlay.set_ignore_cursor_events(true) {
           log::warn!("Failed to set overlay click-through: {}", e);
       }
   }
   ```
   Should become platform-agnostic since Tauri 2.x's `set_ignore_cursor_events()` works on both macOS and Windows:
   ```rust
   if let Err(e) = overlay.set_ignore_cursor_events(true) {
       log::warn!("Failed to set overlay click-through: {}", e);
   }
   ```

2. Also apply click-through to the `cursor-overlay` window if not already done.

3. Verify that `set_ignore_cursor_events` is available for Windows in Tauri 2.x (it is - it uses `WS_EX_TRANSPARENT` on Windows).

## Acceptance Criteria

- [ ] Overlay windows are click-through on both macOS and Windows
- [ ] The `#[cfg(target_os = "macos")]` guard is removed for `set_ignore_cursor_events`
- [ ] Cursor overlay also has click-through enabled on both platforms
- [ ] Code compiles without warnings on both platforms

## Files to Create/Modify

- `src-tauri/src/lib.rs` - Remove macOS-only guard around click-through code

## Integration Points

- **Provides**: Click-through overlay behavior on all platforms
- **Consumes**: None
- **Conflicts**: Avoid large-scale changes to lib.rs setup logic
