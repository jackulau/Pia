---
id: coordinate-overlay
name: Coordinate Overlay - Debug Mode for X/Y Targets
wave: 1
priority: 3
dependencies: []
estimated_hours: 4
tags: [frontend, backend, debug, overlay]
---

## Objective

Add an optional debug mode that displays a visual overlay showing the X/Y coordinates being targeted by mouse actions (click, double-click, move, scroll), making it easier to understand and debug agent behavior.

## Context

When the agent performs mouse actions, users cannot easily see exactly where on screen the agent is targeting. A debug overlay would show a crosshair or indicator at the target coordinates, helping users understand agent behavior and identify when coordinates might be wrong. This should be toggleable via settings.

## Implementation

### 1. Add Debug Setting to Config (`src-tauri/src/config/settings.rs`)

Add to the `GeneralSettings` struct:

```rust
#[serde(default)]
pub show_coordinate_overlay: bool,
```

### 2. Create Overlay Window (Tauri)

Create a new transparent, click-through overlay window that covers the entire screen:

**In `src-tauri/src/lib.rs` or separate module:**

```rust
// Create overlay window (transparent, click-through, always on top)
pub fn create_overlay_window(app: &tauri::AppHandle) -> Result<(), tauri::Error> {
    let overlay = tauri::WebviewWindowBuilder::new(
        app,
        "overlay",
        tauri::WebviewUrl::App("overlay.html".into())
    )
    .title("Coordinate Overlay")
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .fullscreen(true)
    .focused(false)
    .build()?;

    // Make click-through (platform-specific)
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_macos_click_through::WebviewWindowExt;
        overlay.set_click_through(true)?;
    }

    Ok(())
}
```

### 3. Create Overlay HTML (`overlay.html`)

```html
<!DOCTYPE html>
<html>
<head>
  <style>
    * { margin: 0; padding: 0; }
    html, body {
      width: 100vw;
      height: 100vh;
      background: transparent;
      overflow: hidden;
      pointer-events: none;
    }

    .crosshair {
      position: absolute;
      pointer-events: none;
      opacity: 0;
      transition: opacity 0.15s;
    }

    .crosshair.visible {
      opacity: 1;
    }

    .crosshair-h, .crosshair-v {
      position: absolute;
      background: rgba(10, 132, 255, 0.8);
    }

    .crosshair-h {
      width: 40px;
      height: 2px;
      left: -20px;
      top: -1px;
    }

    .crosshair-v {
      width: 2px;
      height: 40px;
      left: -1px;
      top: -20px;
    }

    .crosshair-dot {
      position: absolute;
      width: 8px;
      height: 8px;
      background: #0a84ff;
      border-radius: 50%;
      left: -4px;
      top: -4px;
      box-shadow: 0 0 10px rgba(10, 132, 255, 0.5);
    }

    .crosshair-label {
      position: absolute;
      top: 12px;
      left: 12px;
      font-family: -apple-system, monospace;
      font-size: 11px;
      color: #0a84ff;
      background: rgba(0, 0, 0, 0.7);
      padding: 2px 6px;
      border-radius: 4px;
      white-space: nowrap;
    }

    .click-ripple {
      position: absolute;
      width: 30px;
      height: 30px;
      border: 2px solid #0a84ff;
      border-radius: 50%;
      pointer-events: none;
      animation: ripple 0.4s ease-out forwards;
    }

    @keyframes ripple {
      0% {
        transform: translate(-50%, -50%) scale(0.5);
        opacity: 1;
      }
      100% {
        transform: translate(-50%, -50%) scale(2);
        opacity: 0;
      }
    }
  </style>
</head>
<body>
  <div class="crosshair" id="crosshair">
    <div class="crosshair-h"></div>
    <div class="crosshair-v"></div>
    <div class="crosshair-dot"></div>
    <div class="crosshair-label" id="coord-label">(0, 0)</div>
  </div>

  <script type="module">
    import { listen } from '@tauri-apps/api/event';

    const crosshair = document.getElementById('crosshair');
    const coordLabel = document.getElementById('coord-label');
    let hideTimeout;

    // Listen for coordinate updates from backend
    await listen('show-coordinate', (event) => {
      const { x, y, action_type } = event.payload;

      // Position crosshair
      crosshair.style.left = `${x}px`;
      crosshair.style.top = `${y}px`;
      crosshair.classList.add('visible');

      // Update label
      coordLabel.textContent = `(${x}, ${y})`;

      // Show click ripple for click actions
      if (action_type === 'click' || action_type === 'double_click') {
        showClickRipple(x, y);
      }

      // Hide after delay
      clearTimeout(hideTimeout);
      hideTimeout = setTimeout(() => {
        crosshair.classList.remove('visible');
      }, 1500);
    });

    function showClickRipple(x, y) {
      const ripple = document.createElement('div');
      ripple.className = 'click-ripple';
      ripple.style.left = `${x}px`;
      ripple.style.top = `${y}px`;
      document.body.appendChild(ripple);

      setTimeout(() => ripple.remove(), 400);
    }
  </script>
</body>
</html>
```

### 4. Emit Coordinate Events from Action Execution

**In `src-tauri/src/agent/action.rs` (execute_action function):**

```rust
// Before executing mouse actions, emit coordinate event
async fn emit_coordinate_if_enabled(
    app_handle: &tauri::AppHandle,
    x: i32,
    y: i32,
    action_type: &str
) {
    // Check if overlay is enabled in config
    if let Ok(config) = get_config() {
        if config.general.show_coordinate_overlay {
            let _ = app_handle.emit("show-coordinate", serde_json::json!({
                "x": x,
                "y": y,
                "action_type": action_type
            }));
        }
    }
}

// Call before click/move/scroll actions
emit_coordinate_if_enabled(&app_handle, x, y, "click").await;
```

### 5. Add Toggle in Settings UI (`index.html`)

Add checkbox in settings panel:

```html
<div class="setting-group">
  <label class="setting-checkbox">
    <input type="checkbox" id="show-overlay">
    <span>Show coordinate overlay</span>
  </label>
  <p class="setting-hint">Display crosshair at target coordinates (debug)</p>
</div>
```

### 6. Update Settings JavaScript (`src/main.js`)

```javascript
// Add to DOM elements
const showOverlay = document.getElementById('show-overlay');

// In updateSettingsUI():
showOverlay.checked = currentConfig.general.show_coordinate_overlay || false;

// In saveSettings():
general: {
  // ... existing fields
  show_coordinate_overlay: showOverlay.checked,
}
```

### 7. Manage Overlay Window Lifecycle

- Create overlay window when setting is enabled
- Destroy overlay window when setting is disabled
- Handle app quit to clean up overlay

## Acceptance Criteria

- [ ] Crosshair displays at exact X/Y coordinates for mouse actions
- [ ] Coordinate label shows (X, Y) values next to crosshair
- [ ] Click actions show expanding ripple animation
- [ ] Overlay is fully click-through (doesn't interfere with agent)
- [ ] Toggle in settings enables/disables overlay
- [ ] Overlay covers entire screen, even multi-monitor setups
- [ ] Performance is acceptable (no lag during rapid actions)
- [ ] Overlay hides automatically after 1.5s of inactivity
- [ ] Default is OFF (must be explicitly enabled)

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add show_coordinate_overlay field
- `overlay.html` - New overlay window HTML/CSS/JS
- `src-tauri/src/lib.rs` - Overlay window creation
- `src-tauri/src/agent/action.rs` - Emit coordinate events
- `index.html` - Add settings toggle
- `src/main.js` - Handle settings toggle

## Integration Points

- **Provides**: Visual debug overlay for coordinate targeting
- **Consumes**: Config setting, action coordinates from backend
- **Conflicts**: May need click-through plugin (tauri-plugin-macos-click-through) for macOS

## Platform Notes

- **macOS**: Requires click-through capability (plugin or native code)
- **Windows**: Use WS_EX_LAYERED and WS_EX_TRANSPARENT window styles
- **Linux**: May have compositor-specific requirements for transparency
