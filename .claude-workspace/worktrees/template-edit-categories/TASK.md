---
id: template-edit-categories
name: Template Editing UI and Category System
wave: 1
priority: 2
dependencies: []
estimated_hours: 4
tags: [backend, frontend, templates]
---

## Objective

Wire up the existing `update_template` backend command to the frontend, add inline template editing, and introduce a category/tag system for organizing templates by task type.

## Context

The backend already has an `update_template` Tauri command (`lib.rs` line 369) that accepts id, name, and instruction updates, but the frontend has no way to trigger it — templates can only be deleted and recreated. Additionally, as users accumulate templates, there's no way to organize or filter them. Adding categories aligned with common computer use task types makes templates more discoverable.

## Implementation

### Backend Changes

1. **Add `category` field to `TaskTemplate`** in `src-tauri/src/config/settings.rs`:
   - Add `category: Option<String>` with `#[serde(default)]` for backwards compatibility
   - Predefined categories: "Form Filling", "Web Navigation", "Data Entry", "Data Extraction", "File Management", "Text Editing", "App Interaction", "General"
   - Category is optional — uncategorized templates default to "General"

2. **Update `update_template` command** in `src-tauri/src/lib.rs`:
   - Ensure it accepts optional `category` parameter
   - Add same validation as save_template (name <= 50 chars, instruction non-empty)

3. **Update `save_template` command**:
   - Accept optional `category` parameter

### Frontend Changes

4. **Add template editing** in `src/main.js`:
   - In the template list (settings section), add an "Edit" button alongside existing "Delete" button
   - Clicking Edit opens the save template dialog pre-filled with current name + instruction
   - Dialog title changes to "Edit Template" instead of "Save Template"
   - Submit calls `invoke('update_template', ...)` instead of `save_template`
   - After successful update, refresh the template list

5. **Add category selector** in `src/main.js` and `index.html`:
   - In save/edit template dialog: add a category dropdown with predefined categories
   - In template dropdown (main UI): add category filter — group templates by category with optgroup elements, or add a filter dropdown above the template selector
   - In settings template list: show category badge next to each template name
   - Allow filtering templates by category in settings

6. **Update template dropdown** (`updateTemplateDropdown()` in main.js):
   - Group templates by category using `<optgroup>` elements
   - Within each group, sort alphabetically
   - Uncategorized templates go under "General"

7. **Add CSS** for edit button, category badges, category filter:
   - Style the edit button to match existing delete button
   - Style category badges (small colored tags)
   - Style optgroup labels in dropdown
   - Keep consistent with existing UI design

### Tests

8. **Add Rust tests** in `settings.rs`:
   - Test template CRUD with category field
   - Test backwards compatibility (loading templates without category)
   - Test category defaults to None/General

## Acceptance Criteria

- [ ] Templates can be edited in-place (name, instruction, category) without delete + recreate
- [ ] Category dropdown appears in save/edit template dialog
- [ ] Template dropdown groups templates by category
- [ ] Settings template list shows category badges
- [ ] Existing templates without categories load correctly (backwards compatible)
- [ ] Edit triggers `update_template` backend command (not save + delete)
- [ ] All existing tests still pass

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add category field to TaskTemplate
- `src-tauri/src/lib.rs` - Update save_template and update_template to handle category
- `src/main.js` - Add edit flow, category selector, template grouping, category filter
- `index.html` - Add edit button HTML, category dropdown HTML, category badge CSS

## Integration Points

- **Provides**: Category system that builtin-templates will use to organize pre-built templates
- **Consumes**: Existing template CRUD system and update_template backend
- **Conflicts**: Both this task and template-variables modify template UI in main.js and index.html — this task focuses on editing and categories; template-variables focuses on the variable fill-in flow. Keep changes in separate UI areas.
