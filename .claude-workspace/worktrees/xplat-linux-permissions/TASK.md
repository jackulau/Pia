---
id: xplat-linux-permissions
name: Implement Linux and Windows permission checks
wave: 1
priority: 2
dependencies: []
estimated_hours: 4
tags: [backend, linux, windows, permissions, P1]
---

## Objective

Replace the stub permission checks for Windows and Linux with real platform-appropriate detection, so users get meaningful feedback about missing permissions.

## Context

In `src-tauri/src/permissions.rs`, non-macOS platforms unconditionally return `{screen_capture: true, accessibility: true}` (lines 17-23). This means:

- **Windows**: Users are told permissions are fine even if UAC or enterprise policies block automation
- **Linux/X11**: Generally works without special permissions, but `xdotool`/`libxdo` may need to be installed
- **Linux/Wayland**: Screen capture requires `xdg-desktop-portal` with ScreenCast portal, and input injection is severely restricted

The macOS implementation (`mod macos` at line 26) uses FFI to `CGPreflightScreenCaptureAccess()` and `AXIsProcessTrusted()`. Follow the same pattern: OS-specific submodules.

## Implementation Steps

1. **Restructure `permissions.rs`** to use per-platform modules:
   ```rust
   #[cfg(target_os = "macos")]
   mod macos;
   #[cfg(target_os = "windows")]
   mod windows;
   #[cfg(target_os = "linux")]
   mod linux;
   ```

2. **Windows module** (`mod windows`):
   - Screen capture: Always available (return `true`), unless running in a restricted sandbox
   - Accessibility/automation: Check if the process can use Windows UI Automation — generally always works unless enterprise Group Policy restricts it. A simple heuristic: return `true` (Windows doesn't gate this like macOS)
   - Consider checking if running as admin when relevant

3. **Linux module** (`mod linux`):
   - Detect display server: check `$XDG_SESSION_TYPE` env var (`x11` vs `wayland`)
   - **X11 path**:
     - Screen capture: Check if X11 display is accessible (`$DISPLAY` is set)
     - Accessibility: Check if `xdotool` or `libxdo` is available (attempt `which xdotool`)
   - **Wayland path**:
     - Screen capture: Check if `xdg-desktop-portal` is available via D-Bus. As a heuristic, check if `org.freedesktop.portal.ScreenCast` interface exists
     - Accessibility: Check for `libei` availability or XWayland fallback (`$DISPLAY` set alongside Wayland)
   - Return structured info so the frontend can show appropriate guidance

4. **Extend `PermissionStatus`** struct if needed — consider adding a `details: Option<String>` field for platform-specific guidance messages (e.g., "Install xdotool for input simulation on X11")

5. **Update frontend** (`src/main.js` ~line 265): Remove the `if (isMac)` gate on permissions display so all platforms see permission status.

6. **Add tests** for each platform module using `#[cfg(test)]` with `#[cfg(target_os = ...)]` guards.

## Acceptance Criteria

- [ ] Windows returns meaningful (if simple) permission status
- [ ] Linux detects X11 vs Wayland and checks appropriate permissions
- [ ] Frontend shows permission status on all platforms, not just macOS
- [ ] All existing tests pass
- [ ] New platform-conditional tests exist
- [ ] `PermissionStatus` struct still serializes correctly for frontend consumption

## Files to Create/Modify

- **Modify:** `src-tauri/src/permissions.rs` — restructure with platform modules
- **Modify:** `src/main.js` — remove macOS-only gate on permissions display (~line 265)

## Integration Points

- `check_permissions()` is called from the `check_permissions` Tauri command in `lib.rs`
- Frontend calls `invoke('check_permissions')` during onboarding and settings display
- The returned `PermissionStatus` is consumed by `main.js` to show/hide permission UI
