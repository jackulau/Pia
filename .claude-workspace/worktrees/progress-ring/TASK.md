---
id: progress-ring
name: Circular Progress Ring Indicator
wave: 1
priority: 4
dependencies: []
estimated_hours: 3
tags: [frontend, ui, css]
---

## Objective

Replace or augment the iteration counter with a circular progress indicator around the status dot, providing a more visual representation of progress.

## Context

Currently, progress is shown as text: "25/50" iterations. A circular progress ring around the status dot would provide at-a-glance visual feedback on how far along the agent is in its task, similar to loading indicators in modern apps.

## Implementation

### Frontend Only (Pure CSS/JS)

1. **Update `index.html`**:
   - Wrap the status dot in a progress ring container:
   ```html
   <div class="progress-ring-container">
     <svg class="progress-ring" viewBox="0 0 36 36">
       <circle class="progress-ring-bg" cx="18" cy="18" r="16" />
       <circle class="progress-ring-fill" cx="18" cy="18" r="16" 
               stroke-dasharray="100, 100" />
     </svg>
     <span class="status-dot"></span>
   </div>
   ```

2. **Update `src/styles/modal.css`**:
   - Style the SVG progress ring:
   ```css
   .progress-ring-container {
     position: relative;
     width: 36px;
     height: 36px;
   }
   .progress-ring {
     transform: rotate(-90deg);
   }
   .progress-ring-bg {
     fill: none;
     stroke: var(--bg-tertiary);
     stroke-width: 3;
   }
   .progress-ring-fill {
     fill: none;
     stroke: var(--accent);
     stroke-width: 3;
     stroke-linecap: round;
     transition: stroke-dasharray 0.3s ease;
   }
   .status-dot {
     position: absolute;
     top: 50%;
     left: 50%;
     transform: translate(-50%, -50%);
   }
   ```

3. **Update `src/main.js`**:
   - Calculate progress percentage from iteration/max_iterations
   - Update the `stroke-dasharray` to reflect progress:
   ```javascript
   const progress = (state.iteration / state.max_iterations) * 100;
   const circumference = 2 * Math.PI * 16; // r=16
   const dashArray = `${progress}, 100`;
   progressRingFill.style.strokeDasharray = dashArray;
   ```

4. **Color States**:
   - Running: Accent blue fill animating
   - Completed: Full green ring
   - Error: Full red ring
   - Idle: Empty ring (or hidden)

## Acceptance Criteria

- [ ] Progress ring displays around the status dot
- [ ] Ring fills proportionally based on iteration/max_iterations
- [ ] Smooth animation when progress updates
- [ ] Ring color matches status (blue=running, green=complete, red=error)
- [ ] Status dot remains visible and pulsing inside the ring
- [ ] Ring is appropriately sized (~36px diameter)
- [ ] Works alongside existing iteration text (complementary, not replacement)

## Files to Create/Modify

- `index.html` - Add SVG progress ring structure
- `src/styles/modal.css` - Style the progress ring
- `src/main.js` - Update ring progress on state changes

## Integration Points

- **Provides**: Visual progress indicator
- **Consumes**: iteration and max_iterations from agent state
- **Conflicts**: Modifies status dot area layout (coordinate with any status dot changes)

## Technical Notes

- SVG circle with `stroke-dasharray` is the standard approach for progress rings
- The ring should be behind the status dot (z-index layering)
- `stroke-dasharray: "X, 100"` where X is the percentage fills the ring
- Rotation `-90deg` starts the fill from the top instead of right side
- Consider using CSS custom properties for the progress value to enable smooth transitions
- The existing pulsing animation on the status dot should continue to work
