---
id: drag-action
name: Drag Action Support
wave: 1
priority: 4
dependencies: []
estimated_hours: 4
tags: [backend, input, actions]
---

## Objective

Add a drag action that allows click-and-drag operations from one point to another, enabling tasks like resizing windows, moving files, and slider adjustments.

## Context

Many desktop interactions require dragging: moving files to folders, resizing windows, adjusting sliders, selecting text with the mouse, drag-and-drop interfaces. Currently these tasks are impossible for Pia. Adding drag support unlocks a significant category of previously impossible automation tasks.

## Implementation

### 1. Add Drag Action (`src-tauri/src/agent/action.rs`)

Add new action variant:
```rust
pub enum Action {
    // ... existing variants ...
    Drag {
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        button: Option<String>,  // Default: "left"
        duration_ms: Option<u32>, // How long the drag takes (default: 500ms)
    },
}
```

### 2. Add Mouse Drag Method (`src-tauri/src/input/mouse.rs`)

```rust
impl MouseController {
    pub fn drag(
        &mut self,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        button: enigo::Button,
        duration_ms: u32,
    ) -> Result<(), MouseError> {
        // Move to start position
        self.move_to(start_x, start_y)?;
        std::thread::sleep(Duration::from_millis(50));

        // Press button
        self.enigo.button(button, enigo::Direction::Press)?;
        std::thread::sleep(Duration::from_millis(50));

        // Smooth movement to end position
        let steps = (duration_ms / 16).max(5); // ~60fps
        let dx = (end_x - start_x) as f32 / steps as f32;
        let dy = (end_y - start_y) as f32 / steps as f32;

        for i in 1..=steps {
            let x = start_x + (dx * i as f32) as i32;
            let y = start_y + (dy * i as f32) as i32;
            self.move_to(x, y)?;
            std::thread::sleep(Duration::from_millis(16));
        }

        // Ensure we're at exact end position
        self.move_to(end_x, end_y)?;
        std::thread::sleep(Duration::from_millis(50));

        // Release button
        self.enigo.button(button, enigo::Direction::Release)?;

        Ok(())
    }
}
```

### 3. Implement Drag Execution (`src-tauri/src/agent/action.rs`)

```rust
Action::Drag { start_x, start_y, end_x, end_y, button, duration_ms } => {
    let btn = parse_button(button.as_deref().unwrap_or("left"));
    let duration = duration_ms.unwrap_or(500).min(5000); // Cap at 5 seconds

    log::info!(
        "Drag {} from ({}, {}) to ({}, {}) over {}ms",
        button.as_deref().unwrap_or("left"), start_x, start_y, end_x, end_y, duration
    );

    let mut mouse = MouseController::new()?;
    mouse.drag(*start_x, *start_y, *end_x, *end_y, btn, duration)?;

    Ok(ActionResult {
        success: true,
        completed: false,
        message: Some(format!("Dragged from ({}, {}) to ({}, {})", start_x, start_y, end_x, end_y))
    })
}
```

### 4. Update System Prompt (`src-tauri/src/llm/provider.rs`)

Add drag action documentation:
```
- {"action": "drag", "start_x": 100, "start_y": 200, "end_x": 300, "end_y": 200}
  Click and drag from start position to end position.
  Optional: "button" (default "left"), "duration_ms" (default 500, max 5000)
  Use for: moving files, resizing windows, adjusting sliders, selecting text
```

### 5. Update Frontend Display (`src/main.js`)

```javascript
case 'drag':
    return `🖱️ Drag: (${action.start_x}, ${action.start_y}) → (${action.end_x}, ${action.end_y})`;
```

### 6. Add Visual Feedback for Drag (if overlay exists)

Emit drag events for visual overlay:
```javascript
// In overlay.js
if (action === 'drag') {
    // Draw line from start to end
    const line = document.createElement('div');
    line.className = 'drag-line';
    // ... position and style the line
}
```

## Acceptance Criteria

- [ ] `{"action": "drag", "start_x": 100, "start_y": 100, "end_x": 200, "end_y": 200}` works
- [ ] Drag movement is smooth (not instant teleportation)
- [ ] Button defaults to left when not specified
- [ ] Duration defaults to 500ms when not specified
- [ ] Maximum duration is capped at 5000ms
- [ ] Drag works for file operations (drag file to folder)
- [ ] Drag works for window resizing
- [ ] System prompt documents drag action with examples
- [ ] UI displays drag actions with start/end coordinates

## Files to Create/Modify

- `src-tauri/src/agent/action.rs` - Add Drag variant, implement execution
- `src-tauri/src/input/mouse.rs` - Add drag method with smooth movement
- `src-tauri/src/llm/provider.rs` - Update system prompt
- `src/main.js` - Format drag actions in UI

## Integration Points

- **Provides**: Drag-and-drop capability for automation
- **Consumes**: Mouse controller infrastructure
- **Conflicts**: None - additive change

## Testing Notes

- Test basic drag operation
- Test drag with custom duration
- Test drag with right button
- Test drag across long distances
- Test drag for window resize operation
- Verify smooth movement (not jerky)
