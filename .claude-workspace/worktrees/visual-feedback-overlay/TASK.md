---
id: visual-feedback-overlay
name: Visual Feedback Overlay
wave: 1
priority: 3
dependencies: []
estimated_hours: 6
tags: [frontend, ux, debugging]
---

## Objective

Create a transparent overlay window that shows visual indicators of where the agent is clicking/typing, building user trust and enabling debugging.

## Context

Users currently can't see where the agent is clicking or what it's about to do. This makes the agent feel like a "black box" and makes debugging failures difficult. A visual overlay showing click targets, mouse movements, and action indicators will build trust and aid troubleshooting.

## Implementation

### 1. Create Overlay Window (`src-tauri/src/lib.rs`)

Add a new transparent overlay window in the Tauri builder:
```rust
.setup(|app| {
    // Existing main window setup...

    // Create overlay window
    let overlay = tauri::WebviewWindowBuilder::new(
        app,
        "overlay",
        tauri::WebviewUrl::App("overlay.html".into()),
    )
    .title("Pia Overlay")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .fullscreen(true)  // Cover entire screen
    .focused(false)
    .build()?;

    // Make window click-through
    #[cfg(target_os = "macos")]
    overlay.set_ignore_cursor_events(true)?;

    Ok(())
})
```

### 2. Create Overlay HTML (`src/overlay.html`)

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            margin: 0;
            overflow: hidden;
            pointer-events: none;
            background: transparent;
        }
        .click-indicator {
            position: absolute;
            width: 40px;
            height: 40px;
            border: 3px solid #4CAF50;
            border-radius: 50%;
            transform: translate(-50%, -50%);
            animation: pulse 0.5s ease-out forwards;
        }
        @keyframes pulse {
            0% { transform: translate(-50%, -50%) scale(0.5); opacity: 1; }
            100% { transform: translate(-50%, -50%) scale(2); opacity: 0; }
        }
        .action-label {
            position: absolute;
            background: rgba(0, 0, 0, 0.8);
            color: white;
            padding: 4px 8px;
            border-radius: 4px;
            font-family: monospace;
            font-size: 12px;
            transform: translate(-50%, 30px);
        }
    </style>
</head>
<body>
    <div id="overlay-container"></div>
    <script src="overlay.js"></script>
</body>
</html>
```

### 3. Create Overlay JavaScript (`src/overlay.js`)

```javascript
const { listen } = window.__TAURI__.event;

const container = document.getElementById('overlay-container');

listen('show-action-indicator', (event) => {
    const { action, x, y, label } = event.payload;

    // Remove old indicators
    container.innerHTML = '';

    if (action === 'click' || action === 'double_click') {
        const indicator = document.createElement('div');
        indicator.className = 'click-indicator';
        indicator.style.left = `${x}px`;
        indicator.style.top = `${y}px`;
        container.appendChild(indicator);

        const labelEl = document.createElement('div');
        labelEl.className = 'action-label';
        labelEl.textContent = label;
        labelEl.style.left = `${x}px`;
        labelEl.style.top = `${y}px`;
        container.appendChild(labelEl);

        // Auto-remove after animation
        setTimeout(() => container.innerHTML = '', 600);
    }
    // Add more indicator types as needed
});
```

### 4. Emit Action Indicators (`src-tauri/src/agent/action.rs`)

Before executing actions, emit overlay events:
```rust
fn emit_action_indicator(app_handle: &AppHandle, action: &Action) {
    let payload = match action {
        Action::Click { x, y, button } => serde_json::json!({
            "action": "click",
            "x": x,
            "y": y,
            "label": format!("{} click", button)
        }),
        Action::DoubleClick { x, y } => serde_json::json!({
            "action": "double_click",
            "x": x,
            "y": y,
            "label": "double click"
        }),
        // ... other action types
        _ => return,
    };
    let _ = app_handle.emit("show-action-indicator", payload);
}
```

### 5. Add Configuration Option (`src-tauri/src/config/settings.rs`)

```rust
pub struct GeneralConfig {
    // ... existing fields
    pub show_visual_feedback: bool,  // Default: true
}
```

### 6. Update Frontend Settings (`src/main.js`)

Add toggle in settings:
```javascript
const visualFeedbackToggle = document.createElement('input');
visualFeedbackToggle.type = 'checkbox';
visualFeedbackToggle.checked = config.general.show_visual_feedback;
visualFeedbackToggle.onchange = async () => {
    config.general.show_visual_feedback = visualFeedbackToggle.checked;
    await invoke('save_config', { config });
};
```

## Acceptance Criteria

- [ ] Overlay window is created on app startup, transparent and click-through
- [ ] Click actions show a pulsing circle at click coordinates
- [ ] Double-click actions show distinct indicator
- [ ] Action labels appear briefly showing what action occurred
- [ ] Overlay can be toggled on/off in settings
- [ ] Overlay doesn't interfere with mouse/keyboard operations
- [ ] Overlay works correctly on primary monitor
- [ ] Visual feedback configuration persists across restarts

## Files to Create/Modify

- `src-tauri/src/lib.rs` - Create overlay window
- `src/overlay.html` - Overlay HTML structure
- `src/overlay.js` - Overlay event handling
- `src-tauri/src/agent/action.rs` - Emit action indicators
- `src-tauri/src/config/settings.rs` - Add config option
- `src/main.js` - Add settings toggle
- `src-tauri/tauri.conf.json` - Register overlay.html as a window

## Integration Points

- **Provides**: Visual debugging capability, user feedback
- **Consumes**: Action execution events
- **Conflicts**: None - separate window system

## Testing Notes

- Test overlay appears for click actions
- Test overlay is click-through (doesn't block mouse)
- Test overlay toggle in settings
- Test with multi-monitor setup
- Verify overlay doesn't slow down action execution
