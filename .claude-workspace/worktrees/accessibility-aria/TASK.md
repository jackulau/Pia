---
id: accessibility-aria
name: ARIA Labels and Screen Reader Support
wave: 2
priority: 1
dependencies: [keyboard-navigation]
estimated_hours: 3
tags: [frontend, accessibility, a11y]
---

## Objective

Add comprehensive ARIA attributes and screen reader support to make the app fully accessible to users with visual impairments.

## Context

The app currently lacks ARIA labels, roles, and screen reader support. Icon-only buttons have no accessible names, status changes aren't announced, and the semantic structure is minimal.

## Implementation

1. **Add ARIA roles and labels** (`index.html`)
   - `role="main"` on modal container
   - `role="status"` on status indicator with `aria-live="polite"`
   - `role="button"` on icon buttons with `aria-label`
   - `aria-label` on settings button: "Open settings"
   - `aria-label` on close button: "Hide window"
   - `aria-label` on stop button: "Stop agent"
   - `aria-label` on submit button: "Send instruction"

2. **Form accessibility**
   - Add `<label>` element for instruction input (visually hidden)
   - `aria-describedby` linking input to metrics
   - `aria-invalid` for error states
   - `aria-disabled` sync with disabled attribute

3. **Live regions for dynamic content**
   - `aria-live="polite"` on action display
   - `aria-live="assertive"` on error messages
   - Announce status changes: "Agent running", "Agent stopped"
   - Announce iteration progress: "Iteration 5 of 50"

4. **Dialog accessibility**
   - `role="dialog"` on confirmation dialog
   - `aria-modal="true"` when dialog is open
   - `aria-labelledby` pointing to dialog title
   - `aria-describedby` pointing to dialog message

5. **Settings panel accessibility**
   - `role="region"` with `aria-label="Settings"`
   - `aria-expanded` on settings toggle
   - Proper labeling of form controls

6. **SVG icon accessibility**
   - Add `<title>` elements to inline SVGs
   - `aria-hidden="true"` on decorative icons
   - Ensure icon buttons have text alternatives

## Acceptance Criteria

- [ ] All buttons have accessible names (ARIA labels)
- [ ] Status changes announced to screen readers
- [ ] Action updates announced via live region
- [ ] Confirmation dialog properly identified as modal
- [ ] Form controls have associated labels
- [ ] SVG icons have proper accessibility attributes
- [ ] No accessibility errors in axe-core audit

## Files to Create/Modify

- `index.html` - Add ARIA attributes, roles, labels, live regions
- `src/main.js` - Update aria-* attributes dynamically
- `src/modal.css` - Add visually-hidden class for labels

## Integration Points

- **Provides**: Full screen reader support
- **Consumes**: keyboard-navigation (focus management)
- **Conflicts**: None
