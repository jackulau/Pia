---
id: xplat-wayland-support
name: Add Wayland screen capture and input support
wave: 2
priority: 1
dependencies: [xplat-linux-permissions, xplat-linux-overlay]
estimated_hours: 8
tags: [backend, linux, wayland, P0]
---

## Objective

Ensure the agent can capture screenshots and simulate input on Linux Wayland sessions, either through native Wayland protocols or XWayland fallback.

## Context

Both `enigo` (input) and `xcap` (screen capture) have limited Wayland support:

| Capability | X11 | Wayland |
|---|---|---|
| Screen capture (`xcap` 0.8) | Works | Partial — may need PipeWire/portal |
| Mouse input (`enigo` 0.3) | Works | Limited — uses X11 compat layer |
| Keyboard input (`enigo` 0.3) | Works | Limited — uses X11 compat layer |

Many modern Linux distributions default to Wayland (Ubuntu 22.04+, Fedora, GNOME). Without Wayland support, the agent is unusable for a large segment of Linux users.

## Implementation Steps

### Phase 1: Runtime Detection

1. **Add display server detection** to the capture and input modules:
   ```rust
   pub enum DisplayServer {
       X11,
       Wayland,
       WaylandWithXWayland,
       Unknown,
   }
   
   pub fn detect_display_server() -> DisplayServer {
       let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
       let has_wayland = session_type == "wayland" || std::env::var("WAYLAND_DISPLAY").is_ok();
       let has_x11 = std::env::var("DISPLAY").is_ok();
       
       match (has_wayland, has_x11) {
           (true, true) => DisplayServer::WaylandWithXWayland,
           (true, false) => DisplayServer::Wayland,
           (false, true) => DisplayServer::X11,
           _ => DisplayServer::Unknown,
       }
   }
   ```

2. **Place this in a shared location** — either a new `src-tauri/src/platform.rs` module or in `permissions.rs`.

### Phase 2: Screen Capture on Wayland

3. **Test `xcap` 0.8 on Wayland** — xcap may already use `xdg-desktop-portal` ScreenCast on Wayland. Check xcap's changelog/docs for Wayland support status.

4. **If xcap doesn't work on Wayland**, implement a fallback using `xdg-desktop-portal` D-Bus API:
   - Use `zbus` crate to call `org.freedesktop.portal.ScreenCast` 
   - Create a portal session, select sources, start the stream
   - Capture frames from PipeWire
   - This is complex — consider using the `ashpd` crate which wraps xdg-desktop-portal

5. **Add Cargo.toml conditional dependency**:
   ```toml
   [target.'cfg(target_os = "linux")'.dependencies]
   ashpd = { version = "0.9", optional = true }
   ```

### Phase 3: Input Simulation on Wayland

6. **Test `enigo` on Wayland** — enigo may work via XWayland if `$DISPLAY` is set alongside Wayland. Most Wayland compositors run XWayland by default.

7. **If enigo doesn't work on pure Wayland** (no XWayland):
   - Option A: Require XWayland (document this requirement) — simplest approach
   - Option B: Use `libei` (input emulation interface) via its Rust bindings
   - Option C: Use `wtype` (Wayland equivalent of `xdotool`) as a subprocess fallback
   - Recommend Option A for initial implementation, with clear documentation

8. **Add a startup check** that warns the user if running on pure Wayland without XWayland.

### Phase 4: Integration

9. **Update `screenshot.rs`** to try xcap first, fall back to portal-based capture on Wayland.

10. **Update `loop_runner.rs`** error handling to provide Wayland-specific error messages.

11. **Add comprehensive tests** with `#[cfg(target_os = "linux")]` guards.

## Acceptance Criteria

- [ ] Display server detection works correctly (X11, Wayland, WaylandWithXWayland)
- [ ] Screen capture works on Wayland (either via xcap or portal fallback)
- [ ] Input simulation works on Wayland (via XWayland or alternative)
- [ ] Clear error messages when Wayland features are unavailable
- [ ] No regression on X11 Linux or other platforms
- [ ] All existing tests pass
- [ ] New tests for display server detection

## Files to Create/Modify

- **Create:** `src-tauri/src/platform.rs` — display server detection, platform utilities
- **Modify:** `src-tauri/src/lib.rs` — register `mod platform`, expose detection via Tauri command
- **Modify:** `src-tauri/src/capture/screenshot.rs` — Wayland fallback for screen capture
- **Modify:** `src-tauri/src/input/mouse.rs` — Wayland compatibility check
- **Modify:** `src-tauri/src/input/keyboard.rs` — Wayland compatibility check
- **Modify:** `src-tauri/Cargo.toml` — add Linux-specific dependencies if needed

## Integration Points

- `capture_primary_screen_with_config()` is called from `loop_runner.rs` on every agent loop iteration
- `MouseController` and `KeyboardController` are created fresh per action in `action.rs` via `spawn_blocking`
- Display server detection should be cached (checked once at startup, not per-action)
- `xplat-linux-permissions` task provides the detection foundation this task builds on
