# Pia

Cross-platform desktop agent that uses LLMs with vision to see a user's screen and control their mouse and keyboard to complete tasks.

## Tech Stack

- **Backend**: Rust (edition 2021, MSRV 1.77.2)
- **Frontend**: Vanilla JS/HTML/CSS (no framework)
- **Build**: Tauri 2.x + Vite 7.x
- **Package Manager**: npm

## Commands

```bash
npm run dev              # Start Vite dev server (port 1420)
npm run build            # Production frontend build
cargo tauri dev          # Run full app in development
cargo tauri build        # Production build with bundling
cargo test               # Run Rust unit tests
cargo test -p pia        # Run tests for pia crate specifically
cargo clippy             # Lint Rust code
cargo fmt                # Format Rust code
```

## Project Structure

```
/
├── index.html                  # Main window entry point
├── overlay.html                # Coordinate overlay window
├── cursor-overlay.html         # Cursor indicator overlay window
├── vite.config.js              # Vite config (multi-page: main, overlay, cursor-overlay)
├── package.json                # Frontend deps (@tauri-apps/api, @tauri-apps/cli, vite)
├── src/
│   ├── main.js                 # Main window JS (UI, settings, agent control)
│   ├── overlay.js              # Coordinate overlay logic
│   ├── cursor-overlay.js       # Cursor indicator animation
│   ├── icons/action-icons.js   # SVG icon definitions
│   └── styles/
│       ├── design-tokens.css   # CSS custom properties (colors, spacing, etc.)
│       ├── modal.css           # Modal and dialog styles
│       ├── settings.css        # Settings panel styles
│       └── cursor-overlay.css  # Cursor overlay styles
├── src-tauri/
│   ├── Cargo.toml              # Rust dependencies
│   ├── tauri.conf.json         # Tauri window/app configuration
│   └── src/
│       ├── main.rs             # Rust entry point
│       ├── lib.rs              # App setup, AppState, all #[tauri::command] handlers
│       ├── agent/
│       │   ├── mod.rs          # Module exports
│       │   ├── action.rs       # Action enum, parsing, execution (click, type, key, etc.)
│       │   ├── loop_runner.rs  # AgentLoop: screenshot -> LLM -> parse -> execute cycle
│       │   ├── state.rs        # AgentStateManager: shared agent state (status, metrics)
│       │   ├── conversation.rs # ConversationHistory: multi-turn message tracking
│       │   ├── queue.rs        # QueueManager: sequential instruction queue
│       │   ├── history.rs      # ActionHistory: undo support and action recording
│       │   ├── delay.rs        # DelayController: speed multiplier, adaptive delays
│       │   ├── recovery.rs     # Error classification and retry policies
│       │   └── retry.rs        # RetryContext: screenshot-based action verification
│       ├── capture/
│       │   ├── mod.rs          # Module exports
│       │   └── screenshot.rs   # Screen capture via xcap, JPEG encoding, downsampling
│       ├── config/
│       │   ├── mod.rs          # Module exports
│       │   ├── settings.rs     # Config struct, TOML load/save, provider configs
│       │   └── credentials.rs  # Auto-detect API keys from env, dotenv, CLI configs
│       ├── input/
│       │   ├── mod.rs          # Module exports
│       │   ├── mouse.rs        # MouseController via enigo (click, drag, scroll)
│       │   └── keyboard.rs     # KeyboardController via enigo (type, key combos)
│       ├── history/
│       │   └── mod.rs          # InstructionHistory: persisted instruction log
│       └── llm/
│           ├── mod.rs          # Module exports
│           ├── provider.rs     # LlmProvider trait, Tool/ToolUse/ToolResult, system prompts
│           ├── anthropic.rs    # Anthropic Claude provider (native tool_use support)
│           ├── openai.rs       # OpenAI provider (JSON action in prompt)
│           ├── ollama.rs       # Ollama local provider
│           ├── openrouter.rs   # OpenRouter provider
│           ├── glm.rs          # GLM (ZhipuAI) provider
│           ├── openai_compatible.rs # Generic OpenAI-compatible provider
│           └── sse.rs          # Server-Sent Events streaming parser
└── public/                     # Static assets
```

## Architecture

### Core Loop (`agent/loop_runner.rs`)

The agent runs an iterative loop:
1. Capture screenshot of the primary monitor
2. Send screenshot + instruction + conversation history to LLM
3. Parse LLM response into an Action (JSON text or native tool_use)
4. Execute the action (mouse/keyboard input)
5. Repeat until LLM sends `complete` or `error`, or max iterations reached

### AppState (`lib.rs`)

Tauri managed state holding:
- `AgentStateManager` - agent status, metrics, kill switch (atomic flags + RwLock)
- `Config` - TOML-based configuration (Arc<RwLock<Config>>)
- `InstructionHistory` - persisted history of past instructions
- `QueueManager` - instruction queue for batch execution
- `ActionHistory` - undo stack for reversible actions

### Tauri Commands

All frontend-backend communication goes through `#[tauri::command]` functions in `lib.rs`. The frontend calls them via `@tauri-apps/api`. Key categories:
- Agent control: `start_agent`, `stop_agent`, `pause_agent`, `resume_agent`
- State: `get_agent_state`, `get_config`, `save_config`
- Queue: `add_to_queue`, `start_queue`, `clear_queue`
- Templates: `get_templates`, `save_template`, `delete_template`
- History: `get_instruction_history`, `export_session_json`
- Overlays: `show_cursor_indicator`, `hide_cursor_indicator`
- Providers: `check_provider_health`, `list_provider_models`
- Credentials: `detect_credentials`, `apply_detected_credential`

### Events (Backend -> Frontend)

The backend emits Tauri events that the frontend listens to:
- `agent-state` - full agent state updates (debounced at 50ms)
- `llm-chunk` - streaming LLM response chunks
- `confirmation-required` - dangerous action needs user approval
- `kill-switch-triggered` - emergency stop activated
- `queue-item-started/completed/failed` - queue progress
- `show-coordinate`, `show-action-indicator` - visual feedback

### LLM Providers (`llm/`)

All providers implement the `LlmProvider` trait:
- `send_with_history()` - send conversation with screenshots to LLM
- `health_check()` / `list_models()` - provider diagnostics
- `supports_tools()` - whether provider uses native tool calling

Two response modes:
- **Native tool_use** (Anthropic): LLM returns structured `ToolUse` with tool name + input
- **JSON-in-prompt** (all others): LLM returns JSON action string, parsed via `extract_json()`

### Actions (`agent/action.rs`)

The `Action` enum covers all computer-use operations:
- Mouse: `Click`, `DoubleClick`, `RightClick`, `TripleClick`, `Move`, `Drag`, `Scroll`
- Keyboard: `Type`, `Key` (with modifiers)
- Control: `Wait`, `WaitForElement`, `Batch`, `Complete`, `Error`

Actions are executed via `enigo` for input simulation and `xcap` for screen capture.

### Config (`config/settings.rs`)

TOML config stored at `~/.config/pia/config.toml` (via `dirs` crate). Key sections:
- `general` - default provider, max iterations, speed multiplier, hotkeys, timeouts
- `providers` - per-provider configs (API keys, models, hosts)
- `templates` - saved task templates

### Windows

Three Tauri windows defined in `tauri.conf.json`:
- `main` - primary UI (500x450, always-on-top, transparent, frameless)
- `overlay` - coordinate grid overlay (fullscreen, click-through)
- `cursor-overlay` - visual cursor indicator during actions

## Coding Conventions

- Rust: standard formatting (`cargo fmt`), clippy-clean, `thiserror` for error types, `anyhow` for ad-hoc errors
- Async: `tokio` runtime, `async-trait` for trait objects, `spawn_blocking` for input simulation (enigo is not Send)
- Serialization: `serde` + `serde_json` for API payloads, `toml` for config files
- State sharing: `Arc<RwLock<T>>` for shared mutable state, atomic types for hot-path flags
- Frontend: vanilla JS with `@tauri-apps/api` invoke/listen, CSS custom properties in `design-tokens.css`
- No frontend framework - direct DOM manipulation in `main.js`

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tauri` 2.5 | Desktop app framework |
| `reqwest` 0.12 | HTTP client for LLM APIs |
| `xcap` 0.8 | Cross-platform screen capture |
| `enigo` 0.3 | Cross-platform input simulation |
| `image` 0.25 | Screenshot processing (resize, JPEG encode) |
| `tokio` 1 | Async runtime |
| `serde` / `serde_json` | Serialization |
| `eventsource-stream` 0.2 | SSE parsing for streaming LLM responses |

## Safety Features

- **Kill switch**: Cmd+Shift+Escape (macOS) / Ctrl+Shift+Escape (Windows/Linux) stops the agent immediately
- **Dangerous action confirmation**: key combos like Ctrl+W, Cmd+Q require user approval
- **Preview mode**: see what the agent would do without executing
- **Max iterations**: configurable cap (default 150)
- **Consecutive error limit**: stops after 3 consecutive failures
- **Action retry with verification**: retries actions that don't produce visible screen changes
