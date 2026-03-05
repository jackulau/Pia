---
id: action-timeline
name: Action Timeline - Scrollable History List
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [frontend, ui, state-management]
---

## Objective

Replace the single "Last Action" display with a scrollable timeline showing all actions taken during the agent session, including timestamps.

## Context

Currently, the action display (index.html:306-309) only shows the most recent action. Users need visibility into the full action history to understand what the agent has done, debug issues, and track progress. This timeline will provide chronological visibility into all agent actions with timestamps.

## Implementation

### 1. Modify Backend State (`src-tauri/src/agent/state.rs`)

Add action history storage to `AgentState`:

```rust
// Add to AgentState struct
pub action_history: Vec<ActionHistoryEntry>,

// New struct
#[derive(Clone, Serialize, Debug)]
pub struct ActionHistoryEntry {
    pub action: String,        // JSON action string
    pub timestamp: String,     // ISO 8601 timestamp
    pub is_error: bool,        // Whether this was an error
}
```

Add method to `AgentStateManager`:
- `add_action_to_history(action: String, is_error: bool)` - Adds action with timestamp

Modify `set_last_action` and `set_error` to also call `add_action_to_history`.

### 2. Update HTML Structure (`index.html`)

Replace the action-display section (lines 306-309):

```html
<div class="action-timeline">
  <div class="timeline-header">
    <span class="action-label">Action History</span>
    <span class="action-count" id="action-count">0 actions</span>
  </div>
  <div class="timeline-list" id="timeline-list">
    <div class="timeline-empty">Waiting for instruction...</div>
  </div>
</div>
```

### 3. Add CSS Styles (inline in `index.html` or `src/styles/modal.css`)

```css
.action-timeline {
  padding: 10px 12px;
  max-height: 120px;
  display: flex;
  flex-direction: column;
}

.timeline-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.action-count {
  font-size: 9px;
  color: rgba(255, 255, 255, 0.4);
}

.timeline-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.timeline-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 11px;
  padding: 4px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.timeline-item:last-child {
  border-bottom: none;
}

.timeline-time {
  font-size: 9px;
  color: rgba(255, 255, 255, 0.4);
  white-space: nowrap;
  min-width: 45px;
}

.timeline-action {
  color: rgba(255, 255, 255, 0.6);
  line-height: 1.3;
  flex: 1;
}

.timeline-item.error .timeline-action {
  color: var(--error, #ff453a);
}

.timeline-empty {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.4);
  text-align: center;
  padding: 8px 0;
}
```

### 4. Update JavaScript (`src/main.js`)

Modify `updateAgentState()` function:

```javascript
// Add DOM element reference at top
const timelineList = document.getElementById('timeline-list');
const actionCount = document.getElementById('action-count');

// In updateAgentState():
if (state.action_history && state.action_history.length > 0) {
  actionCount.textContent = `${state.action_history.length} action${state.action_history.length === 1 ? '' : 's'}`;

  // Clear and rebuild timeline
  timelineList.innerHTML = '';

  // Show most recent first (reverse order)
  const recentActions = [...state.action_history].reverse();

  for (const entry of recentActions) {
    const item = document.createElement('div');
    item.className = `timeline-item${entry.is_error ? ' error' : ''}`;

    const time = document.createElement('span');
    time.className = 'timeline-time';
    time.textContent = formatTimestamp(entry.timestamp);

    const action = document.createElement('span');
    action.className = 'timeline-action';
    try {
      const parsed = JSON.parse(entry.action);
      action.textContent = formatAction(parsed);
    } catch {
      action.textContent = entry.action;
    }

    item.appendChild(time);
    item.appendChild(action);
    timelineList.appendChild(item);
  }

  // Auto-scroll to show newest (at top)
  timelineList.scrollTop = 0;
} else {
  timelineList.innerHTML = '<div class="timeline-empty">Waiting for instruction...</div>';
  actionCount.textContent = '0 actions';
}

// Add helper function
function formatTimestamp(isoString) {
  const date = new Date(isoString);
  return date.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
}
```

### 5. Reset Timeline on New Task

In `submitInstruction()`, reset the history display when starting a new task.

## Acceptance Criteria

- [ ] All actions are stored in chronological order with timestamps
- [ ] Timeline displays newest actions at top with scrollable list
- [ ] Timestamps show HH:MM:SS format
- [ ] Error actions are visually distinguished (red text)
- [ ] Action count badge shows total number of actions
- [ ] Timeline clears when starting a new instruction
- [ ] Scrollbar appears only when content overflows
- [ ] Performance is acceptable with 100+ actions

## Files to Create/Modify

- `src-tauri/src/agent/state.rs` - Add action_history field and ActionHistoryEntry struct
- `index.html` - Replace action-display with action-timeline HTML and CSS
- `src/main.js` - Update updateAgentState() and add formatTimestamp()

## Integration Points

- **Provides**: Scrollable action history UI for timeline display
- **Consumes**: Agent state from backend (action_history array)
- **Conflicts**: Avoid modifying formatAction() signature (used by action-icons task)
