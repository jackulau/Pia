---
id: screenshot-preview
name: Live Screenshot Preview Thumbnail
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [frontend, backend, ui]
---

## Objective

Add a small thumbnail preview showing what the agent "sees" (the captured screenshot) to help users understand context and debug issues.

## Context

Currently, the agent captures screenshots before each LLM call but users cannot see what the agent sees. This creates a black-box experience where users don't understand why the agent took certain actions. A live preview thumbnail will provide transparency and help debug issues when the agent misinterprets the screen.

## Implementation

### Backend Changes (src-tauri/src/)

1. **Modify `agent/state.rs`**:
   - Add `last_screenshot: Option<String>` field to `AgentState` struct (stores base64 PNG)
   - Update `AgentStatePayload` to include the screenshot data

2. **Modify `agent/loop_runner.rs`**:
   - After `capture_primary_screen()`, store the screenshot base64 in state
   - Emit the screenshot with `agent-state` events

### Frontend Changes (src/, index.html)

3. **Update `index.html`**:
   - Add a thumbnail container in the modal:
   ```html
   <div class="screenshot-preview">
     <div class="preview-label">Agent View</div>
     <img id="screenshot-thumbnail" class="preview-image" />
   </div>
   ```

4. **Update `src/styles/modal.css`**:
   - Style the preview container (small thumbnail, border, aspect ratio)
   - Add hover-to-enlarge functionality (CSS transform scale)
   - Ensure it doesn't take too much space (~100px width)

5. **Update `src/main.js`**:
   - In `updateAgentState()`, update the thumbnail src with base64 data
   - Handle missing screenshot gracefully (show placeholder)

## Acceptance Criteria

- [ ] Thumbnail displays the most recent screenshot captured by the agent
- [ ] Thumbnail updates in real-time as the agent takes new screenshots
- [ ] Thumbnail is small (~100px wide) and doesn't clutter the UI
- [ ] Hovering over thumbnail shows larger preview (optional enhancement)
- [ ] Placeholder shown when no screenshot is available
- [ ] No significant performance impact from base64 image display

## Files to Create/Modify

- `src-tauri/src/agent/state.rs` - Add screenshot field to state
- `src-tauri/src/agent/loop_runner.rs` - Include screenshot in state updates
- `index.html` - Add thumbnail HTML structure
- `src/styles/modal.css` - Style the preview container
- `src/main.js` - Update thumbnail on state changes

## Integration Points

- **Provides**: Visual screenshot feedback for users
- **Consumes**: Screenshot data from capture module (already captured)
- **Conflicts**: None - additive changes only

## Technical Notes

- Screenshots are already captured as base64 PNG in `capture/screenshot.rs`
- Current screenshot dimensions are captured (width, height) - use for aspect ratio
- Consider compressing/resizing for thumbnail to reduce payload size (optional optimization)
- Base64 images can be displayed directly: `<img src="data:image/png;base64,..." />`
