// SVG icon templates for each action type
// Icons are 12x12px with stroke-based styling for consistency with the UI

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
    <line x1="6" y1="8" x2="6" y2="8.01"/>
    <line x1="10" y1="8" x2="10" y2="8.01"/>
    <line x1="14" y1="8" x2="14" y2="8.01"/>
    <line x1="18" y1="8" x2="18" y2="8.01"/>
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
    <line x1="12" y1="16" x2="12" y2="16.01"/>
  </svg>`,

  default: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="12" cy="12" r="10"/>
    <circle cx="12" cy="12" r="3"/>
  </svg>`
};

/**
 * Get the SVG icon for a given action type
 * @param {string} actionType - The action type (click, double_click, move, type, key, scroll, complete, error)
 * @returns {string} The SVG markup for the icon
 */
export function getActionIcon(actionType) {
  return actionIcons[actionType] || actionIcons.default;
}
