---
id: add-minimize-mode
name: Add Minimize Mode Showing Only Prompting Menu
wave: 1
priority: 1
dependencies: []
estimated_hours: 3
tags: [frontend, ui, feature]
---

## Objective

Add a minimize/restore toggle so the user can collapse the window to show only the prompting input area (and optionally a small status + control buttons if the agent is running).

## Context

The user wants the ability to "minimize" the app to see only the prompting menu — hiding the history, action timeline, metrics, screenshot preview, and other panels. This is NOT a window-manager minimize (to taskbar); it's an in-app compact mode that shrinks the window and hides non-essential sections. The full mode should display everything; the minimized mode should show only what's needed to type and submit instructions.

The existing codebase has patterns for this:
- `.hidden` class for toggling visibility (`display: none !important`)
- `.expanded-only` pattern for conditional display based on modal state
- `appWindow.setSize(new LogicalSize(w, h))` for programmatic window resize
- `localStorage` for persisting UI state
- The `size-mini` CSS preset is a rough analog but hides the input area (opposite of what we want)

All CSS must be added to the inline `<style>` block in `index.html` (not separate CSS files) for transparent window support.

## Implementation

### 1. Add Minimize Mode CSS (`index.html` inline `<style>`)

Add a `.modal.minimized` CSS class that hides everything except:
- **KEEP visible**: `.drag-handle` (shorter, ~20px), minimal header with status + minimize/restore button, `.input-area` (the prompting menu with instruction input, history btn, record btn, queue btn, submit btn), `.control-buttons` (if agent is running — pause/resume/stop)
- **HIDE**: `.preview-toggle-container`, `.template-row`, `.metrics-bar`, `.screenshot-preview`, `.thinking-display`, `.action-display`, `.action-timeline`, `.action-history`, `.queue-panel`, `.recording-panel`, `.export-btn`, `.undo-btn`

The header in minimized mode should be simplified — keep the status indicator (logo + status dot) and the minimize/restore toggle button and close button. Hide other header action buttons (position menu, settings, kill switch, etc.) to save space.

Add smooth transition: `transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1)` consistent with existing modal transitions.

Respect `@media (prefers-reduced-motion: reduce)` — disable transitions in that case.

### 2. Add Minimize Toggle Button (`index.html` HTML)

Add a minimize/restore icon button to `.header-actions` in the modal header. Follow the existing `.icon-btn` pattern (12px font size, title attribute, aria-label). Use a simple icon:
- Minimized state: show a "maximize/expand" icon (e.g., `⬜` or `▢` or SVG)
- Full state: show a "minimize" icon (e.g., `▁` or `─` or `_`)

Place it near the close button for discoverability.

### 3. Add Minimize Mode JavaScript (`src/main.js`)

Add a `toggleMinimizeMode()` function following the pattern of `toggleExpandedMode()`:

```javascript
const FULL_SIZE = { width: 420, height: 450 };  // Match the fixed size from tauri.conf.json
const MINIMIZED_SIZE = { width: 420, height: 130 }; // Just enough for drag handle + input area

let isMinimized = localStorage.getItem('pia-minimized') === 'true';

function toggleMinimizeMode() {
  isMinimized = !isMinimized;
  applyMinimizedState();
  localStorage.setItem('pia-minimized', isMinimized);
}

async function applyMinimizedState() {
  const mainModal = document.getElementById('main-modal');
  const minimizeBtn = document.querySelector('.minimize-btn');

  if (isMinimized) {
    mainModal.classList.add('minimized');
    await appWindow.setSize(new LogicalSize(MINIMIZED_SIZE.width, MINIMIZED_SIZE.height));
  } else {
    mainModal.classList.remove('minimized');
    await appWindow.setSize(new LogicalSize(FULL_SIZE.width, FULL_SIZE.height));
  }

  // Update button icon
  if (minimizeBtn) {
    minimizeBtn.textContent = isMinimized ? '⬜' : '▁';
    minimizeBtn.title = isMinimized ? 'Restore full view' : 'Minimize to prompt only';
  }
}
```

### 4. Initialize on Startup

In `init()`, after other UI state restoration, call `applyMinimizedState()` to restore the last minimize state from localStorage. This should run AFTER the window is visible.

### 5. Add Keyboard Shortcut (Optional)

Consider adding `Cmd+M` (macOS) as a keyboard shortcut for toggling minimize mode, consistent with macOS conventions. Add it to the existing keyboard event handler.

### 6. Handle Edge Cases
- When minimized and user opens settings: restore to full size first, then open settings
- When agent starts/stops running: if minimized, keep minimized but show/hide control buttons
- Double-click on drag handle could toggle minimize (nice UX touch)
- Ensure the input area still has full width in minimized mode

## Acceptance Criteria

- [ ] A minimize/restore button is visible in the modal header
- [ ] Clicking minimize hides all non-essential UI, showing only the prompt input
- [ ] Clicking restore brings back the full UI
- [ ] Window resizes programmatically to fit the minimized/full content
- [ ] Minimize state persists across app restarts (localStorage)
- [ ] Smooth animation/transition when toggling (respects prefers-reduced-motion)
- [ ] Control buttons (pause/resume/stop) remain visible when minimized if agent is running
- [ ] Settings panel interaction works correctly from minimized state
- [ ] No layout overflow or visual glitches in either mode
- [ ] Input area is fully functional in minimized mode (can type, submit, use history/record/queue buttons)

## Files to Create/Modify

- `index.html` - Add `.minimized` CSS rules to inline styles, add minimize button to header HTML
- `src/main.js` - Add minimize toggle function, state management, localStorage persistence, init restore

## Integration Points

- **Provides**: Minimize/restore UI toggle for compact workflow
- **Consumes**: Fixed window size values (should match the full-size dimensions set in tauri.conf.json by the lock-window-size task)
- **Conflicts**: Do NOT modify the `.resize-handle`, S/M/L selector, or expanded mode code — the lock-window-size task handles removing those. Focus ONLY on adding the new minimize feature. If S/M/L or expanded mode code still exists, work around it; if it's been removed, that's fine too.
