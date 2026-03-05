---
id: update-bundle-icons
name: Update Tauri Bundle Icon Configuration
wave: 1
priority: 3
dependencies: []
estimated_hours: 1
tags: [backend, tauri, config]
---

## Objective

Ensure all icon formats are properly configured in tauri.conf.json for complete cross-platform support.

## Context

The current bundle.icon configuration only lists PNG files:
- `icons/32x32.png`
- `icons/128x128.png`
- `icons/128x128@2x.png`

However, the icons directory contains additional formats (icon.ico, icon.icns) that should be explicitly referenced for proper Windows and macOS support.

## Implementation

1. Update `src-tauri/tauri.conf.json` bundle.icon array
2. Include icon.ico for Windows
3. Include icon.icns for macOS
4. Verify icon.png (main high-res) is included
5. Test that bundle generates correctly

## Acceptance Criteria

- [ ] bundle.icon includes icon.ico for Windows
- [ ] bundle.icon includes icon.icns for macOS
- [ ] bundle.icon includes icon.png (high resolution)
- [ ] Build succeeds without icon-related warnings
- [ ] Icons display correctly in built app on each platform

## Files to Create/Modify

- `src-tauri/tauri.conf.json` - Update bundle.icon array

## Integration Points

- **Provides**: Complete icon configuration for all platforms
- **Consumes**: Existing icon files in src-tauri/icons/
- **Conflicts**: None - only modifies JSON config
