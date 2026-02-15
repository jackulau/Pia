---
id: css-webview-compat
name: Fix CSS for cross-platform WebView compatibility
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [frontend, css, cross-platform]
---

## Objective

Ensure all CSS works correctly across macOS WebKit (WKWebView) and Windows WebView2 (Chromium/Edge).

## Context

Tauri uses different WebView engines per platform:
- **macOS**: WKWebView (Safari/WebKit)
- **Windows**: WebView2 (Chromium/Edge)

The app uses several CSS features that may behave differently:
- `backdrop-filter: blur()` - Works in both but needs `-webkit-` prefix for WebKit
- `::-webkit-scrollbar` - WebKit-specific, Chromium also supports it
- `-webkit-user-select: none` - Needs unprefixed version too
- `transparent` background - Rendered differently between WebKit and WebView2
- Font rendering - `-webkit-font-smoothing` vs standard

The current CSS is mostly fine but needs some hardening for Windows.

## Implementation

1. **Audit and fix `index.html` inline styles**:
   - `-webkit-user-select: none` already paired with `user-select: none` ✓
   - Ensure transparent background works on both platforms

2. **Audit `src/styles/modal.css`**:
   - `backdrop-filter: blur(20px)` + `-webkit-backdrop-filter: blur(20px)` ✓ (both present)
   - `::-webkit-scrollbar` works in both WebKit and Chromium ✓
   - Check for any WebKit-only CSS that won't work in Chromium

3. **Add Windows-specific CSS fixes**:
   - On Windows WebView2, transparent windows may show a white flash on load. Add:
     ```css
     body { background: transparent !important; }
     ```
   - Ensure `border-radius` on the main modal works with transparent windows on Windows
   - Windows may need explicit `overflow: hidden` on the root to prevent scrollbar flash

4. **Fix font rendering for Windows**:
   - Add `font-smooth: never` and `-moz-osx-font-smoothing` for cross-platform text
   - Ensure 'Segoe UI' is properly positioned in the font stack (already is ✓)

5. **Test range input styling**:
   - Range inputs (speed slider) may look different on Windows
   - Add Chromium-compatible styling alongside WebKit styling

6. **Verify `pointer-events` and `touch-action` work across both engines**

## Acceptance Criteria

- [ ] All CSS features work in both WebKit (macOS) and WebView2 (Windows)
- [ ] No visual glitches with transparent windows on Windows
- [ ] Scrollbar styling works on both platforms
- [ ] Font rendering is consistent across platforms
- [ ] Range inputs are styled consistently
- [ ] No browser-specific CSS warnings

## Files to Create/Modify

- `index.html` - Minor CSS fixes for Windows compatibility
- `src/styles/modal.css` - Cross-platform CSS hardening
- `src/styles/settings.css` - Range input and form element styling fixes
- `src/styles/design-tokens.css` - Verify token values work cross-platform

## Integration Points

- **Provides**: Cross-platform CSS compatibility
- **Consumes**: None
- **Conflicts**: Coordinate with any other CSS changes
