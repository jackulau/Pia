---
id: setup-auto-memory
name: Populate auto-memory with key project patterns
wave: 1
priority: 2
dependencies: []
estimated_hours: 1
tags: [memory, configuration]
---

## Objective

Populate Claude Code's auto-memory `MEMORY.md` with key patterns, conventions, and debugging insights for the Pia project.

## Context

Claude Code has an auto-memory directory at `~/.claude/projects/-Users-jacklau-Pia/memory/` that persists across conversations. The `MEMORY.md` file is loaded into the system prompt each session. It is currently empty. This should contain concise, high-value notes that complement the `CLAUDE.md` file (which covers project structure) with operational knowledge.

## Implementation

Update `/Users/jacklau/.claude/projects/-Users-jacklau-Pia/memory/MEMORY.md` with:

1. **Build & Dev**:
   - `cargo tauri dev` starts both Vite dev server and Tauri app
   - Rust changes trigger hot-reload; frontend changes are instant via Vite HMR
   - Config file location: `~/Library/Application Support/pia/config.toml` (macOS)
   - `cargo test` runs all Rust tests (inline modules)

2. **Architecture Quick Reference**:
   - Frontend communicates with Rust via `window.__TAURI__.invoke("command_name", {args})`
   - New Tauri commands: add `#[tauri::command]` fn in `lib.rs`, register in `invoke_handler![]`
   - New LLM provider: implement `LlmProvider` trait in `src-tauri/src/llm/`, add to `mod.rs`, add config struct in `settings.rs`, add to `create_provider_from_config()` in `lib.rs`
   - New agent action: add variant to `Action` enum in `action.rs`, add execution in `execute_action()`, update system prompt in `provider.rs`

3. **Common Patterns**:
   - State sharing: `Arc<RwLock<T>>` with Tauri's `State<'_, AppState>`
   - Error pattern: `Result<T, String>` for commands, custom `thiserror` enums internally
   - Frontend events: `app_handle.emit("event-name", payload)` → JS listens via `listen("event-name", callback)`
   - Config changes: modify struct → call `config.save()` to persist

4. **Gotchas**:
   - `lib.rs` is the main entry point (not `main.rs` which just calls `pia_lib::run()`)
   - Three separate HTML entry points: `index.html`, `overlay.html`, `cursor-overlay.html`
   - `src/main.js` is very large (~92k lines) - single-file vanilla JS frontend
   - No JS test framework - only Rust tests exist
   - Window is `alwaysOnTop: true` and `decorations: false` (custom titlebar)

## Acceptance Criteria

- [ ] `MEMORY.md` contains concise, accurate project knowledge
- [ ] Content is under 200 lines (system prompt limit)
- [ ] No duplicate information that's better served by CLAUDE.md
- [ ] Focuses on operational knowledge (how to do things, gotchas, patterns)

## Files to Create/Modify

- `~/.claude/projects/-Users-jacklau-Pia/memory/MEMORY.md` - Update auto-memory

## Integration Points

- **Provides**: Persistent operational knowledge for Claude Code sessions
- **Consumes**: None
- **Conflicts**: None (different file from CLAUDE.md task)
