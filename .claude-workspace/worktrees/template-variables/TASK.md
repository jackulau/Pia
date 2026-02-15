---
id: template-variables
name: Template Parameter Variables with Fill-In UI
wave: 1
priority: 2
dependencies: []
estimated_hours: 4
tags: [backend, frontend, templates]
---

## Objective

Add support for `{{variable}}` placeholders in template instructions, with a fill-in form UI that prompts users to supply values when selecting a template.

## Context

Currently templates are static instruction strings — a template like "Fill out the registration form at example.com with email test@test.com" can't be reused with different values. By adding `{{variable}}` placeholder support, templates become reusable: "Fill out the registration form at {{url}} with email {{email}}".

When a user selects such a template, a small form appears asking them to fill in each variable before the instruction is populated.

## Implementation

### Backend Changes

1. **Update `TaskTemplate` struct** in `src-tauri/src/config/settings.rs`:
   - Add `variables: Vec<TemplateVariable>` field (with serde default for backwards compatibility)
   - Define `TemplateVariable { name: String, description: Option<String>, default_value: Option<String> }`
   - Add `extract_variables(instruction: &str) -> Vec<TemplateVariable>` function that parses `{{name}}` patterns from instruction text using regex
   - Add `render_instruction(instruction: &str, values: &HashMap<String, String>) -> String` that replaces `{{name}}` with provided values

2. **Update template Tauri commands** in `src-tauri/src/lib.rs`:
   - Modify `save_template` to auto-extract variables from instruction text when saving
   - Ensure `get_templates` returns templates with their variable definitions
   - Ensure `update_template` also re-extracts variables when instruction changes

3. **Add migration handling** in `settings.rs`:
   - When loading existing templates that lack `variables` field, auto-extract from instruction text
   - Use `#[serde(default)]` on the variables field for TOML backwards compatibility

### Frontend Changes

4. **Add variable fill-in UI** in `src/main.js`:
   - When `selectTemplate(id)` is called and template has variables:
     - Show a modal/form with input fields for each variable
     - Pre-fill defaults if available
     - On submit: render the instruction with provided values and populate the text area
   - If template has NO variables, behave as before (directly set instruction text)

5. **Add variable highlighting in template management** in `index.html` / `main.js`:
   - In the template list (settings section), show detected variables as tags/badges
   - In the save template dialog, show a preview of detected variables
   - When typing instruction in save dialog, live-detect and display `{{variables}}`

6. **Add CSS for variable UI** in `index.html` or `src/styles/modal.css`:
   - Style the variable fill-in form/modal
   - Style variable tags in template list
   - Keep consistent with existing modal styles (save template dialog, confirmation dialog)

### Tests

7. **Add Rust tests**:
   - Test `extract_variables()` with various patterns: `{{name}}`, `{{url}}`, multiple variables, no variables, nested braces, empty names
   - Test `render_instruction()` with complete values, missing values (leave placeholder), extra values (ignore)
   - Test backwards compatibility: templates without variables field load correctly

## Acceptance Criteria

- [ ] `{{variable_name}}` patterns detected in template instruction text
- [ ] Variables extracted and stored with template on save
- [ ] Fill-in form appears when selecting a template with variables
- [ ] Instruction rendered with user-provided values replaces text area content
- [ ] Templates without variables work exactly as before (backwards compatible)
- [ ] Existing templates load correctly after upgrade (serde default)
- [ ] Variable preview shown in save template dialog
- [ ] All new Rust tests pass

## Files to Create/Modify

- `src-tauri/src/config/settings.rs` - Add TemplateVariable struct, extract_variables(), render_instruction(), update TaskTemplate
- `src-tauri/src/lib.rs` - Update save_template and update_template to extract variables
- `src/main.js` - Add variable fill-in UI, modify selectTemplate(), update save dialog
- `index.html` - Add variable fill-in modal HTML and CSS (or `src/styles/modal.css`)

## Integration Points

- **Provides**: Variable system that builtin-templates will leverage for reusable templates
- **Consumes**: Existing template CRUD system
- **Conflicts**: Both this task and template-edit-categories modify template UI in main.js and index.html — coordinate carefully. This task focuses on the variable fill-in flow; template-edit-categories focuses on editing and categories.
