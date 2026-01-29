---
id: global-hotkey
name: Global Hotkey Window Summon
wave: 1
priority: 3
dependencies: []
estimated_hours: 3
tags: [backend, tauri-plugin, system]
---

## Objective

Add a system-wide keyboard shortcut (e.g., Cmd+Shift+P on macOS, Ctrl+Shift+P on Windows/Linux) to summon or dismiss the Pia window from anywhere.

## Context

The `tauri-plugin-global-shortcut` is already declared as a dependency in `Cargo.toml` but is commented out in `lib.rs`. This feature enables the plugin, registers a global hotkey, and toggles window visibility when pressed.

## Implementation

### Backend (Rust)

1. **Enable plugin** in `src-tauri/src/lib.rs`:
   - Uncomment `.plugin(tauri_plugin_global_shortcut::Builder::new().build())`
   - Register shortcut handler

2. **Add shortcut registration** in `src-tauri/src/lib.rs`:
   - Use platform-specific shortcuts:
     - macOS: `Cmd+Shift+P`
     - Windows/Linux: `Ctrl+Shift+P`
   - Handler toggles window visibility
   - Handle registration errors gracefully

3. **Add configuration** in `src-tauri/src/config/settings.rs`:
   - Add `global_hotkey: Option<String>` to GeneralConfig
   - Default: "CmdOrCtrl+Shift+P"
   - Allow users to customize the shortcut

4. **Add Tauri commands** in `src-tauri/src/lib.rs`:
   - `get_current_hotkey()` - returns registered shortcut
   - `set_global_hotkey(shortcut)` - changes and re-registers shortcut
   - `unregister_global_hotkey()` - disables the feature

5. **Window toggle logic**:
   - If window hidden: show and focus
   - If window visible but not focused: focus
   - If window visible and focused: hide

### Frontend (JavaScript)

6. **Add settings UI** in `index.html`:
   - Add hotkey configuration field in settings panel
   - Show current registered hotkey
   - Input to customize shortcut

7. **Add settings logic** in `src/main.js`:
   - Display current hotkey in settings
   - Handle hotkey change and save

## Acceptance Criteria

- [ ] Default hotkey (Cmd/Ctrl+Shift+P) summons window from anywhere
- [ ] Pressing hotkey when window visible hides it
- [ ] Hotkey works even when Pia is not in focus
- [ ] Users can customize the hotkey in settings
- [ ] Invalid hotkey combinations show error message
- [ ] Hotkey persists across app restarts
- [ ] Graceful handling if hotkey is already registered by another app

## Files to Create/Modify

- `src-tauri/src/lib.rs` - Enable plugin, add shortcut registration and commands
- `src-tauri/src/config/settings.rs` - Add hotkey to config
- `src-tauri/capabilities/default.json` - Add global-shortcut permission if needed
- `index.html` - Add hotkey settings UI
- `src/main.js` - Add hotkey settings handling

## Integration Points

- **Provides**: System-wide window toggle
- **Consumes**: Config for hotkey preference
- **Conflicts**: None - uses existing show_window/hide_window commands

## Platform Notes

- macOS: May require accessibility permissions
- Windows: Works out of box
- Linux: Depends on window manager support
