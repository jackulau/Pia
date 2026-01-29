---
id: state-batching
name: Batch State Updates and Reduce Lock Contention
wave: 1
priority: 2
dependencies: []
estimated_hours: 3
tags: [backend, performance, concurrency]
---

## Objective

Optimize state management by batching updates, reducing lock contention, and minimizing UI update frequency.

## Context

Current issues:
1. `emit_state_update()` called 7-8 times per iteration
2. Full state struct cloned on every `get_state()` call
3. `Arc<RwLock<AgentState>>` write locks on every metric update
4. Frontend receives and re-renders on every state change

Key locations:
- `src-tauri/src/agent/state.rs` line 67 - Full state clone on read
- `src-tauri/src/agent/loop_runner.rs` - Multiple state emissions per iteration
- `src-tauri/src/agent/loop_runner.rs` line 194 - Hard-coded 500ms delay

## Implementation

1. **Batch State Emissions**:
   - Instead of emitting state after every change, batch updates
   - Emit once at the end of each iteration phase (after capture, after LLM, after action)
   - Or emit on a timer (every 100-200ms) rather than every change

2. **Reduce State Cloning**:
   - Create specific getter methods for individual fields
   - Use `Arc<AtomicU32>` for numeric metrics that change frequently
   - Only clone full state when absolutely needed

3. **Optimize Lock Usage**:
   - Use atomic types for simple counters (iteration, tokens)
   - Consider using `parking_lot::RwLock` for faster locks (optional)
   - Batch multiple updates into single write lock acquisition

4. **Adaptive Delay**:
   - Replace hard-coded 500ms sleep with adaptive delay
   - If LLM response took >500ms, skip the delay
   - Make delay configurable

## Acceptance Criteria

- [ ] State emissions reduced to 2-3 per iteration (from 7-8)
- [ ] Numeric metrics use atomic types where appropriate
- [ ] No full state clones for simple metric reads
- [ ] Adaptive delay based on LLM response time
- [ ] Code compiles without errors
- [ ] UI still updates smoothly during agent execution

## Files to Create/Modify

- `src-tauri/src/agent/state.rs` - Add atomic metrics, optimize get_state
- `src-tauri/src/agent/loop_runner.rs` - Batch emissions, adaptive delay

## Integration Points

- **Provides**: Optimized state management with reduced overhead
- **Consumes**: AgentState struct
- **Conflicts**: Avoid editing LLM provider files (handled by string-allocation-optimize)
