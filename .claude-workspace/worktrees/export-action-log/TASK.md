---
id: export-action-log
name: Export Action Log - Save session history as JSON/text for debugging
wave: 1
priority: 2
dependencies: []
estimated_hours: 5
tags: [backend, frontend, logging]
---

## Objective

Implement session action logging with export functionality, allowing users to save their agent session history as JSON or text files for debugging, sharing, or analysis.

## Context

Currently, Pia only tracks the last action in memory. This feature adds comprehensive action history tracking during a session and export capabilities. Users need this for debugging agent behavior, sharing workflows, and understanding what actions were taken.

## Implementation

### Backend (Rust)

1. **Create Action History Module** (`src-tauri/src/agent/history.rs`)
   - `ActionEntry` struct: timestamp, action_type, action_details, screenshot_base64 (optional), llm_response, success, error_message
   - `SessionHistory` struct: session_id, instruction, started_at, ended_at, entries: Vec<ActionEntry>, metrics
   - Thread-safe history manager using Arc<RwLock<SessionHistory>>

2. **Integrate with Loop Runner** (`src-tauri/src/agent/loop_runner.rs`)
   - Record each action to history after execution
   - Capture action details, success/failure, timestamps
   - Store LLM response text for each iteration

3. **Add Export Commands** (`src-tauri/src/lib.rs`)
   - `export_session_json(include_screenshots: bool)` - Returns JSON string
   - `export_session_text()` - Returns human-readable text format
   - `save_session_to_file(path: String, format: String)` - Save to disk
   - `clear_session_history()` - Reset history for new session

4. **Update State** (`src-tauri/src/agent/state.rs`)
   - Add `history: Arc<RwLock<SessionHistory>>` to AgentStateManager
   - Clear history on new agent start

### Frontend (JavaScript)

5. **Export UI** (`index.html`)
   - Add "Export" button in main modal (visible when session has history)
   - Export format selection (JSON/Text) in dropdown or modal

6. **Export Logic** (`src/main.js`)
   - `exportSession(format)` - Trigger backend export and download
   - File download using Blob and anchor element
   - Show export button only when history exists

## Acceptance Criteria

- [ ] Every action during agent execution is logged with timestamp
- [ ] History includes: action type, coordinates/text, success/failure, LLM reasoning
- [ ] JSON export contains full session data with proper structure
- [ ] Text export is human-readable with clear formatting
- [ ] Users can save export to file via browser download
- [ ] History is cleared when starting a new agent session
- [ ] Export button only appears when there's history to export
- [ ] Screenshots can be optionally included in JSON export (large file warning)
- [ ] Metrics (tokens, iterations, duration) included in export

## Files to Create/Modify

- `src-tauri/src/agent/history.rs` - NEW: Action history tracking module
- `src-tauri/src/agent/mod.rs` - Export history module
- `src-tauri/src/agent/loop_runner.rs` - Integrate history recording
- `src-tauri/src/agent/state.rs` - Add history field to state manager
- `src-tauri/src/lib.rs` - Add export commands
- `index.html` - Add export button and UI
- `src/main.js` - Add export handling and file download

## Integration Points

- **Provides**: Session history infrastructure, export APIs
- **Consumes**: AgentLoop actions, AgentState metrics
- **Conflicts**: Modifies loop_runner.rs (coordinate with other features)

## Technical Notes

- Use chrono for timestamps (already in dependencies)
- JSON serialization via serde_json (already available)
- Consider memory limits - optionally exclude screenshots from history
- Text format example:
  ```
  Session: 2024-01-15 14:30:00
  Instruction: "Open Chrome and search for weather"
  
  [1] 14:30:01 - Click at (150, 200)
      LLM: "I see the Chrome icon, clicking to open..."
      Result: Success
  
  [2] 14:30:03 - Type "weather"
      LLM: "Browser opened, typing search query..."
      Result: Success
  ```
