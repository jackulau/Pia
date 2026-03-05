---
id: xplat-frontend-detection
name: Replace deprecated navigator.platform with Tauri OS API
wave: 1
priority: 3
dependencies: []
estimated_hours: 2
tags: [frontend, cross-platform, P3]
---

## Objective

Replace the deprecated `navigator.platform` API with Tauri's `os.platform()` for reliable cross-platform detection in the frontend, and ensure all platform-conditional UI works correctly on Windows and Linux.

## Context

In `src/main.js` (~line 179), `navigator.platform` is used to detect macOS for:
- Kill switch shortcut display text (Cmd+Shift+Escape vs Ctrl+Shift+Escape)
- Onboarding wizard permission display (only shown on macOS, line ~265)

`navigator.platform` is deprecated and may return inconsistent values across WebView implementations on different platforms. Tauri v2 provides `@tauri-apps/plugin-os` with `platform()` that returns reliable values.

## Implementation Steps

1. **Add `@tauri-apps/plugin-os`** dependency if not already present. In Tauri v2, this requires:
   - `npm install @tauri-apps/plugin-os`
   - Add the plugin to `src-tauri/Cargo.toml` dependencies: `tauri-plugin-os`
   - Register in `lib.rs`: `.plugin(tauri_plugin_os::init())`
   - Add to `capabilities/default.json` if needed

2. **Alternative approach** (simpler): Create a Tauri command that returns the platform:
   ```rust
   #[tauri::command]
   fn get_platform() -> String {
       if cfg!(target_os = "macos") { "macos" }
       else if cfg!(target_os = "windows") { "windows" }
       else { "linux" }
   }
   ```
   This avoids adding a plugin dependency for a single value.

3. **Update `main.js`**:
   - Replace `navigator.platform.toUpperCase().includes('MAC')` with the Tauri-provided platform value
   - Initialize platform detection at app startup (async)
   - Update all conditional UI: kill switch text, onboarding permissions, any keyboard shortcut hints

4. **Ensure onboarding works on all platforms**:
   - Line ~265: Remove `if (isMac)` gate or replace with platform-appropriate content
   - Show relevant permission guidance for Windows ("No special permissions needed") and Linux ("Ensure X11 display access" / "Wayland may have limitations")

5. **Update keyboard shortcut display** throughout the UI to show platform-correct modifier keys.

## Acceptance Criteria

- [ ] `navigator.platform` is no longer used anywhere in the frontend
- [ ] Platform detection uses Tauri API or backend command
- [ ] Kill switch shortcut text shows correctly on all platforms
- [ ] Onboarding wizard shows platform-appropriate content
- [ ] All existing functionality preserved on macOS

## Files to Create/Modify

- **Modify:** `src/main.js` — replace platform detection, update conditional UI
- **Maybe modify:** `src-tauri/src/lib.rs` — add `get_platform` command (if not using plugin)
- **Maybe modify:** `src-tauri/Cargo.toml` — add `tauri-plugin-os` (if using plugin approach)

## Integration Points

- Platform detection is used throughout `main.js` for UI rendering
- The onboarding flow reads permission status from `check_permissions` command
- Kill switch shortcut is registered in `lib.rs` and displayed in `main.js`
