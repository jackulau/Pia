---
id: extended-actions
name: Add Extended Action Types (Drag, Select, Window)
wave: 1
priority: 4
dependencies: []
estimated_hours: 5
tags: [backend, input, features]
---

## Objective

Add missing action types to expand the agent's capabilities beyond basic mouse/keyboard operations.

## Context

Current actions are limited to:
- Click, DoubleClick, Move
- Type, Key
- Scroll
- Complete, Error

Missing capabilities that users need:
- Drag and drop operations
- Text selection/highlighting
- Right-click context menus (partially supported)
- Window management
- Triple-click for line selection

## Implementation

1. Modify `/src-tauri/src/agent/action.rs`:
   - Add `Drag` action: `{action: "drag", from_x, from_y, to_x, to_y}`
   - Add `Select` action: `{action: "select", start_x, start_y, end_x, end_y}`
   - Add `TripleClick` action for line selection
   - Add `RightClick` action (explicit, currently via button param)
   - Add `Wait` action: `{action: "wait", duration_ms: 1000}`

2. Modify `/src-tauri/src/input/mouse.rs`:
   - Implement drag operation (mouse_down, move, mouse_up)
   - Add mouse button state tracking
   - Implement select operation

3. Modify `/src-tauri/src/llm/provider.rs`:
   - Update system prompt with new action documentation
   - Add tool definitions for new actions (if native tool_use)

4. Ensure `enigo` supports required operations:
   - Check drag support
   - Check mouse button hold capability

## Acceptance Criteria

- [ ] Drag action moves items between locations
- [ ] Select action highlights text regions
- [ ] TripleClick selects entire lines
- [ ] Wait action pauses execution
- [ ] All new actions documented in system prompt
- [ ] Existing actions unaffected

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Add new action variants
- `src-tauri/src/input/mouse.rs` - Implement drag/select
- `src-tauri/src/llm/provider.rs` - Update system prompt

## Integration Points

- **Provides**: Extended action capabilities
- **Consumes**: Existing input infrastructure
- **Conflicts**: Coordinate with native-tool-use for tool definitions
