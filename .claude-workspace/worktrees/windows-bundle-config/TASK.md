---
id: windows-bundle-config
name: Add Windows bundle and installer configuration
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [config, windows, build]
---

## Objective

Add proper Windows-specific bundle configuration for installer generation (NSIS), code signing placeholders, and build settings.

## Context

The `tauri.conf.json` has a `macOS` bundle section but **no `windows` section**. This means Windows builds will use defaults, which may produce suboptimal installers. The app also uses `"transparent": true` and `"decorations": false` which require specific handling on Windows (WebView2).

Additionally, the `"macOSPrivateApi": true` setting enables transparent window support on macOS. On Windows, transparent windows with WebView2 work out of the box in Tauri 2.x but may need the window to have specific settings.

## Implementation

1. **Add Windows bundle config to `src-tauri/tauri.conf.json`**:
   ```json
   "windows": {
     "certificateThumbprint": null,
     "digestAlgorithm": "sha256",
     "timestampUrl": ""
   }
   ```

2. **Add NSIS installer configuration**:
   ```json
   "nsis": {
     "installMode": "currentUser",
     "displayLanguageSelector": false
   }
   ```

3. **Verify icon files exist** for Windows:
   - `icons/icon.ico` should be present (it's listed in bundle config)
   - Verify it contains multiple resolutions (16x16, 32x32, 48x48, 256x256)

4. **Add Windows-specific capabilities if needed** in Tauri 2.x permissions config.

5. **Update `src-tauri/Cargo.toml`** if any Windows-specific feature flags are needed.

## Acceptance Criteria

- [ ] `tauri.conf.json` has a `windows` section under `bundle`
- [ ] NSIS installer settings are configured
- [ ] Windows icon file (icon.ico) is verified to exist with proper resolutions
- [ ] `cargo tauri build` would produce a proper Windows installer (verified via config validation)
- [ ] No changes break the macOS build

## Files to Create/Modify

- `src-tauri/tauri.conf.json` - Add Windows bundle configuration
- `src-tauri/icons/` - Verify icon files (no modification if already present)

## Integration Points

- **Provides**: Windows installer configuration
- **Consumes**: None
- **Conflicts**: Avoid modifying existing macOS bundle settings
