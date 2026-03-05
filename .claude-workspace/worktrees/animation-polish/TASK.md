---
id: animation-polish
name: Animation and Transition Polish
wave: 2
priority: 2
dependencies: [modal-drag-enhance]
estimated_hours: 3
tags: [frontend, ux, animation]
---

## Objective

Refine all animations and transitions for a polished, professional feel with smooth micro-interactions.

## Context

The app has basic transitions but could benefit from more refined animations for state changes, panel transitions, and user interactions. Animations should feel responsive but not distracting.

## Implementation

1. **Button interaction animations** (`src/modal.css`)
   - Subtle scale on press (0.97)
   - Smooth color transitions (150ms ease)
   - Ripple effect on click (optional)
   - Disabled state fade transition

2. **Status indicator animations**
   - Smooth color transitions between states
   - Pulse animation refinement (softer, less distracting)
   - Spin animation for loading states
   - Success/error state flash

3. **Panel transition animations**
   - Settings panel slide-in from top
   - Fade + scale for confirmation dialog
   - Smooth opacity transitions
   - Staggered animation for settings options

4. **Input field animations**
   - Focus ring expansion
   - Placeholder fade on focus
   - Shake animation for validation errors
   - Subtle border glow on focus

5. **Metrics and action display**
   - Counter animation for metrics changes
   - Smooth text content transitions
   - Action text slide-in effect
   - Progress indication animations

6. **Loading and processing states**
   - Skeleton loading for async content
   - Submit button loading spinner
   - Action processing indicator

7. **Toast notifications**
   - Refined slide-up animation
   - Smooth exit animation
   - Stack animation for multiple toasts

8. **Respect reduced motion preference**
   - `@media (prefers-reduced-motion: reduce)` query
   - Disable animations for users who prefer reduced motion
   - Keep essential state changes visible

## Acceptance Criteria

- [ ] All buttons have press feedback
- [ ] Status transitions are smooth
- [ ] Panel open/close animations feel natural
- [ ] Input focus has visual animation
- [ ] Animations respect reduced motion preference
- [ ] No animation jank or stutter
- [ ] Animations complete within 300ms (no sluggishness)

## Files to Create/Modify

- `src/modal.css` - Animation keyframes, transitions, reduced motion query
- `index.html` - Add animation trigger classes
- `src/main.js` - Toggle animation classes for state changes

## Integration Points

- **Provides**: Polished visual feedback
- **Consumes**: modal-drag-enhance (drag state classes)
- **Conflicts**: None
