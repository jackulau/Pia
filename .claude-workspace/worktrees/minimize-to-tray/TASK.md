---
id: minimize-to-tray
name: Minimize to Menubar/Tray - System Tray Integration
wave: 1
priority: 1
dependencies: []
estimated_hours: 5
tags: [backend, frontend, system]
---

## Objective

Add system tray (Windows/Linux) or menubar (macOS) icon for quick access, allowing the window to be minimized to tray and restored with a click.

## Context

The window is always-on-top which can be intrusive. A system tray/menubar icon allows users to completely hide the window when not needed and quickly restore it. This is a common pattern for utility apps that need to be accessible but not always visible.

## Implementation

1. **Add Tauri tray plugin** (`src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`)
   - Add `tauri-plugin-system-tray` dependency
   - Configure tray in Tauri capabilities

2. **Create tray icon** (`src-tauri/icons/`)
   - Create small tray icon (16x16, 32x32 for HiDPI)
   - Use app logo or simplified version
   - Provide both light and dark variants for macOS

3. **Initialize system tray** (`src-tauri/src/lib.rs`)
   ```rust
   use tauri::tray::{TrayIconBuilder, TrayIcon};

   let tray = TrayIconBuilder::new()
       .icon(app.default_window_icon().unwrap().clone())
       .menu(&menu)
       .on_tray_icon_event(|tray, event| {
           // Handle clicks
       })
       .build(app)?;
   ```

4. **Tray menu options**
   - Show/Hide Pia
   - Start/Stop Agent (if running)
   - Settings
   - Separator
   - Quit

5. **Click behavior**
   - Single click: Toggle window visibility
   - Right click: Show context menu
   - macOS: Click on menubar icon shows menu

6. **Minimize to tray on close** (`src/main.js`, `src-tauri/src/lib.rs`)
   - Instead of closing, hide window when close button clicked
   - Window can be restored from tray
   - Add "Quit" option in tray menu for actual exit

7. **Status indicator in tray**
   - Show different icon when agent is running (optional)
   - Tooltip shows current status

8. **Keyboard shortcut** (optional)
   - Global hotkey to show/hide (e.g., Cmd+Shift+P on macOS)
   - Use `tauri-plugin-global-shortcut`

## Acceptance Criteria

- [ ] Tray/menubar icon appears on app launch
- [ ] Click on icon toggles window visibility
- [ ] Right-click shows context menu
- [ ] Close button hides to tray instead of quitting
- [ ] "Quit" option in menu fully exits app
- [ ] Works on macOS (menubar) and Windows/Linux (tray)
- [ ] Tray icon visible in both light and dark system themes

## Files to Create/Modify

- `src-tauri/Cargo.toml` - Add tray plugin dependency
- `src-tauri/tauri.conf.json` - Configure tray capabilities
- `src-tauri/src/lib.rs` - Tray initialization and event handling
- `src-tauri/icons/` - Tray icon assets (tray-icon.png, tray-icon-dark.png)
- `src/main.js` - Update close button behavior

## Integration Points

- **Provides**: System tray access, minimize-to-tray behavior
- **Consumes**: Tauri tray API
- **Conflicts**: Changes close button behavior (currently calls hide_window)
