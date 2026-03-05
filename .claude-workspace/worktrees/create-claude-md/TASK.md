---
id: create-claude-md
name: Create CLAUDE.md project memory file
wave: 1
priority: 1
dependencies: []
estimated_hours: 1
tags: [memory, configuration]
---

## Objective

Create a comprehensive `CLAUDE.md` file at the project root that documents the Pia codebase for Claude Code's project memory system.

## Context

Pia is a cross-platform computer use agent built with Tauri 2.x (Rust backend + vanilla JS/HTML/CSS frontend). It uses LLM providers to analyze screenshots and execute mouse/keyboard actions on the user's computer. Currently there is no `CLAUDE.md` file, meaning Claude Code has no persistent project memory across sessions.

## Implementation

Create `/CLAUDE.md` in the project root with the following sections:

### Content to include:

1. **Project Overview** - Pia is a Tauri 2.x desktop app for LLM-driven computer use automation
2. **Tech Stack** - Rust (backend), vanilla JS/HTML/CSS (frontend), Vite (bundler), Tauri 2.x (framework)
3. **Key Commands**:
   - `npm run dev` / `cargo tauri dev` - Development server
   - `npm run build` / `cargo tauri build` - Production build
   - `cargo test` - Run Rust tests (inline `#[cfg(test)]` modules)
   - No JS test framework configured
4. **Project Structure**:
   - `src-tauri/src/lib.rs` - Main Tauri app setup, command handlers, app state
   - `src-tauri/src/agent/` - Agent loop, actions, state, queue, retry, recovery, conversation history
   - `src-tauri/src/llm/` - LLM providers (Anthropic, OpenAI, OpenRouter, Ollama, GLM, OpenAI-compatible), provider trait, SSE streaming
   - `src-tauri/src/config/` - TOML config (settings.rs), credential detection (credentials.rs)
   - `src-tauri/src/capture/` - Screen capture via xcap
   - `src-tauri/src/input/` - Mouse/keyboard simulation via enigo
   - `src-tauri/src/history/` - Instruction history persistence
   - `src/main.js` - Frontend logic (vanilla JS, ~92k lines)
   - `src/styles/` - CSS (design-tokens.css, modal.css, settings.css, cursor-overlay.css)
   - `index.html` - Main window
   - `src/overlay.html` + `src/overlay.js` - Coordinate overlay
   - `cursor-overlay.html` + `src/cursor-overlay.js` - Visual feedback overlay
5. **Architecture Patterns**:
   - Tauri commands in `lib.rs` bridge frontend ↔ backend via `#[tauri::command]`
   - `AppState` struct holds `AgentStateManager`, `Config`, `InstructionHistory`, `QueueManager`, `ActionHistory`
   - Config stored as TOML at platform config dir (`dirs::config_dir()/pia/config.toml`)
   - LLM providers implement `LlmProvider` trait (`async_trait`) with `send_with_history()`, `health_check()`, `list_models()`
   - Agent actions: click, double_click, type, key, scroll, move, drag, triple_click, right_click, wait, wait_for_element, batch, complete, error
   - Three Tauri windows: main (500x450, always-on-top), overlay (coordinate grid), cursor-overlay (visual feedback)
6. **Coding Conventions**:
   - Rust: snake_case, `thiserror` for error types, `serde` derive for serialization, `tokio` async runtime
   - Frontend: Vanilla JS (no framework), CSS custom properties in design-tokens.css
   - Tests: Rust inline `#[cfg(test)]` modules, `tempfile` for test fixtures
   - Error handling: `Result<T, String>` for Tauri commands, `thiserror` enums for internal errors
7. **LLM Providers** - Anthropic (native tool_use), OpenAI, OpenRouter, Ollama, GLM, OpenAI-compatible
8. **Key Dependencies** - tauri 2.5, reqwest, tokio, xcap, enigo, image, base64, serde/serde_json/toml

## Acceptance Criteria

- [ ] `CLAUDE.md` exists at project root
- [ ] Contains accurate project overview, commands, structure, and conventions
- [ ] Information is concise and scannable (not overly verbose)
- [ ] No sensitive data (API keys, paths) included

## Files to Create/Modify

- `CLAUDE.md` - Create new project memory file

## Integration Points

- **Provides**: Project context for all future Claude Code sessions
- **Consumes**: None
- **Conflicts**: None
