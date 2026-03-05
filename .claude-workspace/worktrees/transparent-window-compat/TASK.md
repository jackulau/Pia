---
id: transparent-window-compat
name: Fix transparent window handling for Windows
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [backend, windows, ui]
---

## Objective

Ensure transparent, decoration-less windows work properly on Windows with Tauri 2.x and WebView2.

## Context

The app uses `"transparent": true` and `"decorations": false` for all windows (main, overlay, cursor-overlay). On macOS this works via `macOSPrivateApi: true` which enables private WebKit APIs for transparency. On Windows with WebView2, transparent windows require different handling:

1. **Tauri 2.x on Windows** supports transparent windows but may need explicit WebView2 configuration
2. The `macOSPrivateApi` flag is macOS-only and doesn't affect Windows
3. Windows transparent windows with `decorations: false` may have issues with:
   - Window shadow (no native shadow without decorations)
   - Rounded corners (Windows 11 has native rounded corners, Windows 10 doesn't)
   - Hit-testing for window dragging (the app uses a custom drag handle)
   - The window may appear with a white background briefly on startup

## Implementation

1. **Verify Tauri 2.x transparent window config for Windows**:
   - Check if any additional Tauri config is needed for Windows transparency
   - In Tauri 2.x, transparent windows on Windows should work with `"transparent": true`
   - May need to set the WebView2 background to transparent explicitly

2. **Handle window drag on Windows** (`src/main.js`):
   - The drag handle uses `data-tauri-drag-region` which works cross-platform in Tauri 2.x
   - Verify this works with `decorations: false` on Windows

3. **Fix window shadow on Windows**:
   - On macOS, `macOSPrivateApi` provides window shadow even without decorations
   - On Windows, need to add CSS shadow to the main modal container to simulate window shadow:
     ```css
     .modal {
       box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
     }
     ```

4. **Handle Windows 10 vs 11 differences**:
   - Windows 11 has native rounded corners for undecorated windows
   - Windows 10 needs CSS `border-radius` on the content area
   - Both should be handled via CSS (already using `border-radius` on `.modal`)

5. **Fix potential white flash on Windows startup**:
   - Set window background color in Tauri config or via the setup hook
   - In `lib.rs` setup, ensure window is shown only after content is loaded

6. **Ensure `skipTaskbar: true` works on Windows** - verify the main window skips the taskbar on Windows (it's a system tray app).

## Acceptance Criteria

- [ ] Transparent windows render correctly on Windows (no white background)
- [ ] Window dragging works via the custom drag handle on Windows
- [ ] Main modal has visible shadow/border on both platforms
- [ ] No white flash on startup on Windows
- [ ] `skipTaskbar` works on Windows
- [ ] Rounded corners work on both Windows 10 and 11
- [ ] All existing macOS functionality is preserved

## Files to Create/Modify

- `src-tauri/tauri.conf.json` - Verify/add Windows transparency settings
- `src-tauri/src/lib.rs` - Add Windows-specific window setup if needed
- `index.html` / `src/styles/modal.css` - CSS shadow/border for Windows
- `src/overlay.html` - Verify overlay transparency on Windows

## Integration Points

- **Provides**: Working transparent windows on Windows
- **Consumes**: None
- **Conflicts**: Coordinate with css-webview-compat task for CSS changes
