---
id: confirmation-flow
name: Fix Dangerous Action Confirmation Flow
wave: 1
priority: 3
dependencies: []
estimated_hours: 3
tags: [backend, frontend, safety]
---

## Objective

Fix the incomplete confirmation flow for dangerous actions so users can actually approve or deny them.

## Context

Currently, when a dangerous action is detected:
1. Backend emits `confirmation-required` event
2. Frontend shows a dialog
3. Backend waits 5 seconds (hardcoded!)
4. Backend continues regardless of user response

This is broken - the user's response is never communicated back to the backend.

## Implementation

1. Modify `/src-tauri/src/lib.rs`:
   - Add `confirm_action` Tauri command
   - Add `deny_action` Tauri command
   - Create channel for confirmation responses

2. Modify `/src-tauri/src/agent/loop_runner.rs`:
   - Replace hardcoded 5-second sleep with channel wait
   - Handle confirmation/denial response
   - Add timeout for no response (default to deny)
   - Resume or abort based on response

3. Modify `/src-tauri/src/agent/state.rs`:
   - Add `AwaitingConfirmation` status
   - Add pending_action field for display

4. Modify `/Users/jacklau/Pia/src/main.js`:
   - Wire up confirmation dialog buttons to invoke commands
   - Call `confirm_action` or `deny_action` commands
   - Update UI state during confirmation

## Acceptance Criteria

- [ ] Confirmation dialog waits for user response
- [ ] User can approve dangerous action
- [ ] User can deny dangerous action
- [ ] Timeout after 30 seconds defaults to deny
- [ ] UI shows pending state during confirmation
- [ ] Agent properly resumes or aborts after response

## Files to Create/Modify

- `src-tauri/src/lib.rs` - Add confirmation commands
- `src-tauri/src/agent/loop_runner.rs` - Implement channel-based waiting
- `src-tauri/src/agent/state.rs` - Add confirmation status
- `src/main.js` - Wire up frontend buttons

## Integration Points

- **Provides**: Working confirmation flow
- **Consumes**: Existing event system
- **Conflicts**: None
