---
id: xplat-linux-overlay
name: Enable overlay click-through on Linux
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [backend, linux, overlay, P0]
---

## Objective

Make the overlay and cursor-overlay windows click-through on Linux, preventing them from intercepting mouse clicks during agent operation.

## Context

In `src-tauri/src/lib.rs` (~line 1085), `set_ignore_cursor_events(true)` is gated behind `#[cfg(target_os = "macos")]`. The `windows-overlay-clickthrough` task (already complete) addressed Windows. Linux remains unhandled.

Tauri's `set_ignore_cursor_events(true)` is documented to work on Linux via `gtk` under X11, but behavior on Wayland varies by compositor. This task needs to:
1. Enable click-through for Linux
2. Handle the X11 vs Wayland difference gracefully

## Implementation Steps

1. **Remove the macOS-only gate** on `set_ignore_cursor_events` in `lib.rs`. Check what the `windows-overlay-clickthrough` branch did — it likely already made this cross-platform for Windows. The fix may be as simple as removing `#[cfg(target_os = "macos")]` entirely since Tauri's API is cross-platform.

2. **Verify Tauri v2.5 behavior** for `set_ignore_cursor_events` on Linux:
   - On X11: Should work via GTK's `input_shape` / `gdk_window_set_pass_through`
   - On Wayland: May not work depending on compositor. Need graceful fallback.

3. **Add fallback for Wayland** if `set_ignore_cursor_events` fails:
   - Log a warning: "Overlay click-through not supported on this Wayland compositor"
   - Consider hiding the overlay entirely on Wayland if click-through isn't supported
   - Alternatively, use a very small/minimal overlay that stays out of the way

4. **Test the overlay window creation** in `setup_overlay_windows()` and `show_cursor_indicator()` / `hide_cursor_indicator()` functions — ensure the click-through call is applied to both `overlay` and `cursor-overlay` windows.

5. **Add error handling** — the current macOS code already uses `if let Err(e) = ...`, follow that pattern.

## Acceptance Criteria

- [ ] `set_ignore_cursor_events(true)` is called for overlay windows on Linux (not just macOS)
- [ ] X11: Overlay is fully click-through
- [ ] Wayland: Graceful fallback if click-through isn't supported (warning + hide or minimize overlay)
- [ ] No regression on macOS (still works)
- [ ] All existing tests pass
- [ ] Check that the windows-overlay-clickthrough changes are compatible

## Files to Create/Modify

- **Modify:** `src-tauri/src/lib.rs` — remove/adjust the `#[cfg(target_os = "macos")]` gate on click-through calls

## Integration Points

- Overlay windows are created in `setup_overlay_windows()` in `lib.rs`
- `show_cursor_indicator()` / `hide_cursor_indicator()` manage the cursor overlay
- `show_overlay()` / `hide_overlay()` manage the coordinate overlay
- The agent loop in `loop_runner.rs` triggers overlay show/hide via Tauri events
