---
id: multi-step-queuing
name: Multi-step Queuing - Chain multiple instructions sequentially
wave: 2
priority: 1
dependencies: [export-action-log]
estimated_hours: 6
tags: [backend, frontend, state-management]
---

## Objective

Implement instruction queuing that allows users to chain multiple instructions together, executing them sequentially: "First do X, then do Y, finally do Z".

## Context

Complex workflows often require multiple distinct tasks. Currently users must wait for one task to complete before entering the next. This feature allows queueing instructions upfront, with the agent processing them in order automatically.

## Implementation

### Backend (Rust)

1. **Create Queue Manager** (`src-tauri/src/agent/queue.rs`)
   - `InstructionQueue` struct with VecDeque<QueuedInstruction>
   - `QueuedInstruction`: id, instruction, status (Pending/Running/Completed/Failed), result
   - Thread-safe queue using Arc<RwLock<InstructionQueue>>
   - Methods: add, remove, get_next, clear, reorder

2. **Modify AgentState** (`src-tauri/src/agent/state.rs`)
   - Add `queue: InstructionQueue` field
   - Track current_queue_index
   - Add queue state to serialized payload

3. **Update Loop Runner** (`src-tauri/src/agent/loop_runner.rs`)
   - On instruction completion, check queue for next
   - Auto-start next queued instruction
   - Emit queue progress events
   - Handle queue item failure (continue or stop option)

4. **Add Tauri Commands** (`src-tauri/src/lib.rs`)
   - `add_to_queue(instruction: String)` - Add instruction to queue
   - `add_multiple_to_queue(instructions: Vec<String>)` - Batch add
   - `remove_from_queue(id: String)` - Remove specific item
   - `clear_queue()` - Clear all queued instructions
   - `reorder_queue(order: Vec<String>)` - Reorder by IDs
   - `get_queue()` - Get current queue state
   - `start_queue()` - Begin processing queue
   - `set_queue_failure_mode(mode: String)` - "stop" or "continue"

### Frontend (JavaScript)

5. **Queue UI** (`index.html`)
   - Queue panel showing pending/completed instructions
   - "Add to Queue" button next to Submit
   - Queue item reordering (drag-drop or up/down buttons)
   - Queue item removal (X button)
   - Start Queue / Clear Queue buttons
   - Progress indicator showing current item / total
   - Failure mode toggle in settings

6. **Queue Logic** (`src/main.js`)
   - `addToQueue()` - Add current input to queue
   - `removeFromQueue(id)` - Remove specific item
   - `startQueue()` - Begin processing
   - `renderQueue()` - Update queue display
   - Handle queue events from backend
   - Parse multi-step input (split on "then" keyword)

## Acceptance Criteria

- [ ] Users can add multiple instructions to a queue
- [ ] Queue displays pending/completed/running status for each item
- [ ] Agent automatically processes queue items in order
- [ ] Users can remove items from queue before execution
- [ ] Users can reorder queue items
- [ ] Queue progress is visible (1/5, 2/5, etc.)
- [ ] Failed instructions can stop or skip based on setting
- [ ] Users can clear the entire queue
- [ ] Natural language parsing: "do X, then Y" splits into queue items
- [ ] Queue persists if app is closed mid-execution (stored in config)
- [ ] History integration: each queue item recorded separately

## Files to Create/Modify

- `src-tauri/src/agent/queue.rs` - NEW: Queue management module
- `src-tauri/src/agent/mod.rs` - Export queue module
- `src-tauri/src/agent/state.rs` - Add queue field to state
- `src-tauri/src/agent/loop_runner.rs` - Queue progression logic
- `src-tauri/src/lib.rs` - Add queue commands
- `src-tauri/src/config/settings.rs` - Add queue persistence and failure mode
- `index.html` - Add queue UI panel and controls
- `src/main.js` - Add queue management functions

## Integration Points

- **Provides**: Multi-step workflow execution
- **Consumes**: AgentLoop, history infrastructure (from export-action-log)
- **Depends On**: export-action-log (for per-instruction history recording)
- **Conflicts**: Modifies loop_runner.rs and state.rs significantly

## Technical Notes

- Natural language parsing for "then"/"after that"/"next":
  ```
  Input: "Open Chrome, then search for weather, then click first result"
  Queue: ["Open Chrome", "search for weather", "click first result"]
  ```
- Queue item IDs use UUID
- Consider adding delay between queue items (configurable)
- Failure modes:
  - "stop" - Halt queue on any failure
  - "continue" - Skip failed item, proceed with next
- Queue state persisted to allow resume after restart
- Each queue item gets its own history entry (uses export-action-log infrastructure)
