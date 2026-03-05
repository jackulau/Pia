---
id: keyboard-navigation
name: Full Keyboard Navigation Support
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [frontend, accessibility, ux]
---

## Objective

Implement comprehensive keyboard navigation allowing users to operate the entire UI without a mouse.

## Context

Currently the app only has Enter-to-submit functionality. Users need full keyboard navigation including tab order, escape to close modals, and keyboard shortcuts for common actions.

## Implementation

1. **Tab order and focus management** (`index.html`, `src/modal.css`)
   - Define logical tab order: input -> submit -> settings -> close
   - Add visible focus indicators (focus-visible styles)
   - Style focus rings with accent color (#0a84ff)
   - Ensure all interactive elements are focusable

2. **Keyboard shortcuts** (`src/main.js`)
   - `Escape` - Close settings panel / Cancel confirmation dialog
   - `Cmd/Ctrl + Enter` - Force submit even when button disabled
   - `Cmd/Ctrl + ,` - Open settings
   - `Cmd/Ctrl + .` - Stop agent (when running)
   - `Tab` / `Shift+Tab` - Navigate between elements

3. **Settings panel keyboard support**
   - Arrow keys to navigate between providers in dropdown
   - Enter to select provider
   - Escape to close settings and return to main view

4. **Confirmation dialog keyboard support**
   - Focus trap within dialog when open
   - Tab between Cancel and Confirm buttons
   - Enter on focused button to activate
   - Escape to cancel

5. **Focus restoration**
   - Return focus to previous element when closing panels
   - Auto-focus input field on app start
   - Focus submit button after successful submission

## Acceptance Criteria

- [ ] All interactive elements reachable via Tab key
- [ ] Visible focus indicators on all focusable elements
- [ ] Escape closes settings panel and confirmation dialog
- [ ] Cmd/Ctrl+, opens settings
- [ ] Cmd/Ctrl+. stops running agent
- [ ] Focus trapped in confirmation dialog when open
- [ ] Focus returns to previous element after closing panels

## Files to Create/Modify

- `index.html` - Add tabindex attributes where needed
- `src/modal.css` - Add :focus-visible styles
- `src/main.js` - Add keyboard event listeners and focus management

## Integration Points

- **Provides**: Keyboard navigation infrastructure
- **Consumes**: Existing DOM structure
- **Conflicts**: None
