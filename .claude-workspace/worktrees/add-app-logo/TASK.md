---
id: add-app-logo
name: Add Pia Mascot Logo to UI
wave: 1
priority: 2
dependencies: []
estimated_hours: 2
tags: [frontend, ui, branding]
---

## Objective

Display the Pia mascot logo in the application UI to strengthen brand identity.

## Context

The cute robot mascot (icon.svg) is only used as the app icon but doesn't appear anywhere in the actual UI. Adding it to the interface would:
- Strengthen brand recognition
- Make the app feel more personalized
- Utilize the custom character design

## Implementation

1. Determine best placement for logo (options: header area, status indicator, splash/welcome state)
2. Inline the SVG or reference it properly for Tauri bundling
3. Add appropriate sizing and styling
4. Consider animation possibilities (subtle idle animation when ready)

## Acceptance Criteria

- [ ] Pia mascot logo visible in the app UI
- [ ] Logo properly sized and positioned (not overwhelming the compact UI)
- [ ] Logo respects the existing dark theme
- [ ] No layout issues on different window sizes
- [ ] Optional: subtle animation or interaction

## Files to Create/Modify

- `index.html` - Add logo markup and potentially inline SVG
- Potentially add CSS for logo styling

## Integration Points

- **Provides**: Visual branding in UI
- **Consumes**: icon.svg design/colors
- **Conflicts**: Modifies modal-header section of index.html
