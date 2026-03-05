---
id: action-icons
name: Action Icons - Visual Action Type Indicators
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [frontend, ui, icons]
---

## Objective

Add visual SVG icons for each action type (click, double-click, move, type, key, scroll, complete, error) to make the action display more scannable and visually informative.

## Context

Currently, actions are displayed as text-only (e.g., "Click left at (100, 200)"). Adding icons will make it easier to quickly scan and identify action types at a glance, especially in the action timeline. Icons should follow the existing design language (14px inline SVGs with stroke-based styling).

## Implementation

### 1. Create Action Icons Module (`src/icons/action-icons.js`)

```javascript
// SVG icon templates for each action type
export const actionIcons = {
  click: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M5 3l14 9-6 2 4 7-3 1-4-7-5 4z"/>
  </svg>`,

  double_click: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M5 3l14 9-6 2 4 7-3 1-4-7-5 4z"/>
    <circle cx="18" cy="6" r="3" fill="currentColor"/>
  </svg>`,

  move: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M5 9l-3 3 3 3"/>
    <path d="M19 9l3 3-3 3"/>
    <path d="M9 5l3-3 3 3"/>
    <path d="M9 19l3 3 3-3"/>
    <line x1="2" y1="12" x2="22" y2="12"/>
    <line x1="12" y1="2" x2="12" y2="22"/>
  </svg>`,

  type: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <rect x="2" y="4" width="20" height="16" rx="2"/>
    <line x1="6" y1="8" x2="6" y2="8"/>
    <line x1="10" y1="8" x2="10" y2="8"/>
    <line x1="14" y1="8" x2="14" y2="8"/>
    <line x1="18" y1="8" x2="18" y2="8"/>
    <line x1="8" y1="12" x2="16" y2="12"/>
    <line x1="6" y1="16" x2="18" y2="16"/>
  </svg>`,

  key: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <rect x="3" y="6" width="18" height="12" rx="2"/>
    <rect x="6" y="9" width="3" height="3" rx="0.5"/>
    <rect x="10.5" y="9" width="3" height="3" rx="0.5"/>
    <rect x="15" y="9" width="3" height="3" rx="0.5"/>
    <rect x="7" y="13" width="10" height="2" rx="0.5"/>
  </svg>`,

  scroll: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <rect x="8" y="2" width="8" height="20" rx="4"/>
    <line x1="12" y1="6" x2="12" y2="10"/>
    <path d="M10 8l2-2 2 2"/>
    <path d="M10 16l2 2 2-2"/>
  </svg>`,

  complete: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="12" cy="12" r="10"/>
    <path d="M8 12l3 3 5-6"/>
  </svg>`,

  error: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="12" cy="12" r="10"/>
    <line x1="12" y1="8" x2="12" y2="12"/>
    <line x1="12" y1="16" x2="12" y2="16"/>
  </svg>`,

  default: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="12" cy="12" r="10"/>
    <line x1="12" y1="8" x2="12" y2="12"/>
    <line x1="12" y1="16" x2="12" y2="16"/>
  </svg>`
};

export function getActionIcon(actionType) {
  return actionIcons[actionType] || actionIcons.default;
}
```

### 2. Update CSS Styles (inline in `index.html`)

Add icon-specific styles:

```css
.action-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: rgba(255, 255, 255, 0.5);
  flex-shrink: 0;
}

.action-icon svg {
  width: 12px;
  height: 12px;
}

.action-icon.click { color: #0a84ff; }
.action-icon.double_click { color: #5e5ce6; }
.action-icon.move { color: #64d2ff; }
.action-icon.type { color: #30d158; }
.action-icon.key { color: #ffd60a; }
.action-icon.scroll { color: #bf5af2; }
.action-icon.complete { color: #30d158; }
.action-icon.error { color: #ff453a; }
```

### 3. Update Action Display in JavaScript (`src/main.js`)

Modify `formatAction()` to return both icon and text, or create a new function:

```javascript
import { getActionIcon } from './icons/action-icons.js';

// Create a function that returns { icon, text } for an action
function formatActionWithIcon(action) {
  const actionType = action.action || 'default';
  const icon = getActionIcon(actionType);
  const text = formatAction(action); // Existing function

  return { icon, text, type: actionType };
}

// Update the display rendering to include icon
function renderActionItem(entry) {
  const item = document.createElement('div');
  item.className = `timeline-item${entry.is_error ? ' error' : ''}`;

  try {
    const parsed = JSON.parse(entry.action);
    const { icon, text, type } = formatActionWithIcon(parsed);

    // Create icon element
    const iconEl = document.createElement('span');
    iconEl.className = `action-icon ${type}`;
    iconEl.innerHTML = icon;

    // Create text element
    const textEl = document.createElement('span');
    textEl.className = 'timeline-action';
    textEl.textContent = text;

    item.appendChild(iconEl);
    item.appendChild(textEl);
  } catch {
    const textEl = document.createElement('span');
    textEl.className = 'timeline-action';
    textEl.textContent = entry.action;
    item.appendChild(textEl);
  }

  return item;
}
```

### 4. Alternative: Inline Icons Without Module

If ES modules are problematic, embed icons directly in main.js:

```javascript
const ACTION_ICONS = {
  click: '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 3l14 9-6 2 4 7-3 1-4-7-5 4z"/></svg>',
  // ... other icons
};

function getActionIcon(type) {
  return ACTION_ICONS[type] || ACTION_ICONS.default;
}
```

## Acceptance Criteria

- [ ] Each action type has a distinct, recognizable SVG icon
- [ ] Icons are 12x12px with stroke-based styling matching existing UI
- [ ] Icon colors correspond to action type (blue for click, green for type, etc.)
- [ ] Icons display inline before the action text
- [ ] Icons are accessible (appropriate contrast ratios)
- [ ] Error actions use red icon color
- [ ] Unknown action types show a default icon
- [ ] Icons render correctly at all modal sizes

## Files to Create/Modify

- `src/icons/action-icons.js` - New file with icon definitions (optional, can inline)
- `index.html` - Add icon CSS styles
- `src/main.js` - Add icon rendering logic to action display

## Integration Points

- **Provides**: Visual icons for action types
- **Consumes**: Action type string from parsed actions
- **Conflicts**: If action-timeline is also being built, coordinate on where icons are rendered
