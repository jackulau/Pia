---
id: action-batching
name: Action Batching for Reduced Round-Trips
wave: 1
priority: 1
dependencies: []
estimated_hours: 6
tags: [backend, performance, llm]
---

## Objective

Enable the LLM to return multiple actions in a single response, executing them sequentially without additional API round-trips.

## Context

Currently, Pia executes one action per LLM call, requiring a screenshot and API round-trip for every single action. This is inefficient for sequences like "type text, press tab, type more text" which could be batched. Action batching will significantly reduce latency and API costs for multi-step tasks.

## Implementation

### 1. Update Action Schema (`src-tauri/src/agent/action.rs`)

Add a new `Batch` action variant:
```rust
pub enum Action {
    // ... existing variants ...
    Batch { actions: Vec<Action> },
}
```

Implement serialization/deserialization for nested actions.

### 2. Update System Prompt (`src-tauri/src/llm/provider.rs`)

Modify `get_system_prompt()` to document the batch action:
```
- {"action": "batch", "actions": [{"action": "type", "text": "hello"}, {"action": "key", "key": "tab"}]}
  Execute multiple actions in sequence without taking a new screenshot between them.
  Use for predictable action sequences. Max 5 actions per batch recommended.
```

### 3. Update Action Execution (`src-tauri/src/agent/action.rs`)

In `execute_action()`, handle the Batch variant:
```rust
Action::Batch { actions } => {
    for action in actions {
        let result = execute_action(action, confirm_dangerous)?;
        if !result.success {
            return Ok(result); // Stop batch on first failure
        }
        if result.completed {
            return Ok(result); // Complete action ends batch
        }
        // Small delay between batched actions
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(ActionResult { success: true, completed: false, message: Some("Batch completed".into()) })
}
```

### 4. Update Agent Loop (`src-tauri/src/agent/loop_runner.rs`)

Ensure batch actions are properly logged and state is updated after each sub-action execution. Format batch actions nicely for the UI.

### 5. Update Frontend Display (`src/main.js`)

Format batch actions in the action display:
```javascript
function formatAction(action) {
    if (action.action === 'batch') {
        return `Batch (${action.actions.length} actions): ${action.actions.map(formatSingleAction).join(' → ')}`;
    }
    // ... existing formatting
}
```

## Acceptance Criteria

- [ ] LLM can return `{"action": "batch", "actions": [...]}` and all sub-actions execute
- [ ] Batch execution stops on first failure and reports which action failed
- [ ] Dangerous actions within batches still trigger confirmation
- [ ] UI displays batch actions with all sub-actions visible
- [ ] System prompt documents batch action with usage guidelines
- [ ] Max batch size is enforced (recommend 5-10 actions)
- [ ] Inter-action delay is configurable (default 100ms)

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Add Batch variant, update execute_action
- `src-tauri/src/llm/provider.rs` - Update system prompt
- `src-tauri/src/agent/loop_runner.rs` - Update state emission for batches
- `src/main.js` - Format batch actions in UI

## Integration Points

- **Provides**: Batch action capability for other features
- **Consumes**: Existing action execution infrastructure
- **Conflicts**: None - additive change

## Testing Notes

- Test with 2-action batch (type + key)
- Test with batch containing dangerous action
- Test batch with failing action in middle
- Test batch size limits
- Verify token savings with batch vs individual calls
