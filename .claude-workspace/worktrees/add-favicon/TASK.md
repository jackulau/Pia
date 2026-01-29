---
id: add-favicon
name: Add Web Favicon Support
wave: 1
priority: 1
dependencies: []
estimated_hours: 1
tags: [frontend, html]
---

## Objective

Add proper favicon support to the HTML for browser/webview contexts.

## Context

The current index.html lacks `<link rel="icon">` tags. While Tauri handles native app icons via tauri.conf.json, adding favicon links ensures proper icon display in development mode, webview contexts, and potential future PWA support.

## Implementation

1. Copy/reference the existing icon assets for web use
2. Add favicon link tags to `index.html` `<head>` section
3. Support multiple sizes for different contexts (16x16, 32x32, favicon.ico)

## Acceptance Criteria

- [ ] `<link rel="icon">` tag added with SVG favicon (best for modern browsers)
- [ ] Fallback ICO favicon reference added
- [ ] Favicon displays correctly in browser dev mode
- [ ] No console errors related to missing favicon

## Files to Create/Modify

- `index.html` - Add favicon link tags in `<head>`
- May need to copy `src-tauri/icons/icon.svg` to web-accessible location or use relative path

## Integration Points

- **Provides**: Web favicon support
- **Consumes**: Existing icon.svg asset
- **Conflicts**: None - only adds to `<head>` section
