---
id: task-templates
name: Task Templates - Save common instructions as reusable presets
wave: 1
priority: 1
dependencies: []
estimated_hours: 4
tags: [frontend, backend, config]
---

## Objective

Implement a task template system that allows users to save frequently-used instructions as named presets, and quickly load them for reuse.

## Context

Users often repeat similar computer-use tasks. This feature allows saving instructions as templates that can be selected from a dropdown, reducing repetitive typing and improving workflow efficiency.

## Implementation

### Backend (Rust)

1. **Extend Config** (`src-tauri/src/config/settings.rs`)
   - Add `templates: Vec<TaskTemplate>` to Config struct
   - Create `TaskTemplate` struct with fields: `id`, `name`, `instruction`, `created_at`
   
2. **Add Tauri Commands** (`src-tauri/src/lib.rs`)
   - `get_templates()` - Returns all saved templates
   - `save_template(name: String, instruction: String)` - Creates new template
   - `delete_template(id: String)` - Removes a template
   - `update_template(id: String, name: String, instruction: String)` - Edits existing

### Frontend (JavaScript)

3. **Template UI** (`index.html`)
   - Add template dropdown above instruction input
   - Add "Save as Template" button next to submit
   - Add template management section in settings panel
   
4. **Template Logic** (`src/main.js`)
   - `loadTemplates()` - Fetch templates from backend on startup
   - `selectTemplate(id)` - Populate input with selected template
   - `saveAsTemplate()` - Show name prompt, save current instruction
   - `deleteTemplate(id)` - Remove with confirmation
   - Render template list in settings

## Acceptance Criteria

- [ ] Users can save current instruction as a named template
- [ ] Templates persist across app restarts (stored in config.toml)
- [ ] Templates appear in a dropdown for quick selection
- [ ] Selecting a template fills the instruction input
- [ ] Users can delete templates from settings
- [ ] Templates have unique IDs (UUID)
- [ ] Empty/whitespace-only instructions cannot be saved as templates
- [ ] Template names are limited to 50 characters

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add TaskTemplate struct and Vec<TaskTemplate> field
- `src-tauri/src/config/mod.rs` - Export TaskTemplate type
- `src-tauri/src/lib.rs` - Add template CRUD commands
- `index.html` - Add template dropdown, save button, management UI
- `src/main.js` - Add template loading, selection, and management functions

## Integration Points

- **Provides**: Reusable instruction presets stored in config
- **Consumes**: Existing config load/save infrastructure
- **Conflicts**: None - self-contained feature

## Technical Notes

- Use UUID v4 for template IDs (add `uuid` crate to Cargo.toml)
- Templates stored in `~/.config/pia/config.toml` alongside other settings
- Frontend should cache templates to avoid repeated backend calls
- Consider alphabetical sorting of template dropdown
