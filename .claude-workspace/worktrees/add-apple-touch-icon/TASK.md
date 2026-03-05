---
id: add-apple-touch-icon
name: Add Apple Touch Icon for iOS/Safari
wave: 1
priority: 4
dependencies: []
estimated_hours: 1
tags: [frontend, html, mobile]
---

## Objective

Add Apple touch icon support for iOS home screen bookmarks and Safari.

## Context

If users access Pia via Safari or add it to their iOS home screen, having an apple-touch-icon ensures the Pia mascot appears instead of a generic screenshot. This is also good practice for web apps.

## Implementation

1. Create or use existing 180x180 PNG icon (Apple's recommended size)
2. Add `<link rel="apple-touch-icon">` to index.html
3. Ensure icon is accessible from web root or proper path

## Acceptance Criteria

- [ ] 180x180 PNG icon exists (or closest existing size is used)
- [ ] `<link rel="apple-touch-icon">` added to HTML head
- [ ] Icon displays when adding to iOS home screen (if testable)
- [ ] No console warnings about missing apple-touch-icon

## Files to Create/Modify

- `index.html` - Add apple-touch-icon link tag
- Potentially create `apple-touch-icon.png` from existing icon assets

## Integration Points

- **Provides**: iOS/Safari bookmark icon support
- **Consumes**: Existing icon PNG assets
- **Conflicts**: None - only adds to `<head>` section
