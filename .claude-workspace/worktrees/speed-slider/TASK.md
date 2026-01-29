---
id: speed-slider
name: Speed Slider - Action Delay Control
wave: 1
priority: 2
dependencies: []
estimated_hours: 5
tags: [backend, frontend, control]
---

## Objective

Add a speed slider that controls the delay between agent actions, allowing users to slow down execution to observe behavior or speed it up for efficiency.

## Context

Currently, delays are hardcoded:
- 50ms before mouse clicks (`src-tauri/src/input/mouse.rs:101`)
- 500ms between iterations (`src-tauri/src/agent/loop_runner.rs:194`)

Users need control over execution speed for:
- Watching the agent work step-by-step (slower)
- Debugging unexpected behavior (very slow)
- Efficient task completion (faster)
- Accessibility (some users need slower visual feedback)

## Implementation

### Backend (Rust)

1. **Add speed_multiplier to config** - `src-tauri/src/config/settings.rs`
   - Add `speed_multiplier: f32` field to `GeneralConfig` (range: 0.25 to 3.0)
   - Default to `1.0` (normal speed)

2. **Create DelayController** - `src-tauri/src/agent/delay.rs` (new file)
   ```rust
   pub struct DelayController {
       speed_multiplier: f32,
   }
   
   impl DelayController {
       pub fn new(speed_multiplier: f32) -> Self { ... }
       pub fn calculate_delay(&self, base_ms: u64) -> Duration { ... }
       pub fn iteration_delay(&self) -> Duration { ... }  // base: 500ms
       pub fn click_delay(&self) -> Duration { ... }      // base: 50ms
   }
   ```

3. **Update mouse.rs** - `src-tauri/src/input/mouse.rs`
   - Pass delay value to click functions instead of hardcoded 50ms
   - Or accept a DelayController reference

4. **Update loop_runner.rs** - `src-tauri/src/agent/loop_runner.rs`
   - Create DelayController from config at start
   - Use `delay_controller.iteration_delay()` instead of hardcoded 500ms
   - Pass delay info to action execution

5. **Add Tauri command** - `src-tauri/src/lib.rs`
   - `set_speed_multiplier(multiplier: f32)` command
   - Validate range (0.25 to 3.0)
   - Update running agent's delay if possible

### Frontend (JavaScript)

6. **Add Speed Slider to settings** - `src/main.js`
   - Range slider input (0.25x to 3.0x)
   - Labels: "0.25x (Slow)" ... "1x (Normal)" ... "3x (Fast)"
   - Live value display showing current multiplier
   - Save to config on change

7. **Add speed indicator to main UI** - `src/main.js`
   - Show current speed near metrics bar
   - Quick-access buttons: 0.5x, 1x, 2x (optional)

8. **Update settings panel** - `index.html`
   - Add slider styling
   - Show speed value with unit (e.g., "1.5x")

## Speed Presets

| Preset | Multiplier | Iteration Delay | Click Delay |
|--------|------------|-----------------|-------------|
| Very Slow | 0.25x | 2000ms | 200ms |
| Slow | 0.5x | 1000ms | 100ms |
| Normal | 1.0x | 500ms | 50ms |
| Fast | 2.0x | 250ms | 25ms |
| Very Fast | 3.0x | 167ms | 17ms |

## Acceptance Criteria

- [ ] Speed slider visible in settings panel
- [ ] Slider range: 0.25x to 3.0x with reasonable steps (0.25)
- [ ] Current speed value displayed
- [ ] Speed changes apply to running agent (or on next iteration)
- [ ] Speed persists in config across sessions
- [ ] Iteration delay scales with multiplier
- [ ] Click delay scales with multiplier
- [ ] Speed indicator visible during agent execution

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add speed_multiplier config
- `src-tauri/src/agent/delay.rs` - NEW: DelayController struct
- `src-tauri/src/agent/mod.rs` - Export delay module
- `src-tauri/src/agent/loop_runner.rs` - Use DelayController
- `src-tauri/src/input/mouse.rs` - Accept configurable delay
- `src-tauri/src/lib.rs` - Add set_speed_multiplier command
- `src/main.js` - Speed slider UI
- `index.html` - Slider styling

## Integration Points

- **Provides**: Configurable timing for all agent operations
- **Consumes**: None (independent feature)
- **Conflicts**: Avoid modifying action execution logic (handled by preview-mode task)
