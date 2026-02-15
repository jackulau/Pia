---
id: builtin-templates
name: Ship Pre-Built Templates for Common Computer Use Tasks
wave: 2
priority: 1
dependencies: [template-variables, template-edit-categories, case-specific-prompts]
estimated_hours: 3
tags: [backend, frontend, templates, content]
---

## Objective

Create a curated set of pre-built templates that ship with Pia, covering the most common computer use scenarios with well-crafted instructions, proper categories, and useful variables.

## Context

Users currently start with zero templates and no examples of what good instructions look like. Shipping built-in templates serves two purposes: (1) immediately useful templates for common tasks, and (2) examples that teach users how to write effective computer use instructions. These templates should leverage the variable system (from template-variables task) and category system (from template-edit-categories task), and align with task-type classification (from case-specific-prompts task).

## Implementation

1. **Add built-in template definitions** — Create `src-tauri/src/config/builtin_templates.rs`:
   - Define a `get_builtin_templates() -> Vec<TaskTemplate>` function
   - Each template has: name, instruction (with {{variables}}), category, and is marked as `builtin: true`
   - Built-in templates should NOT be deletable by users (or at least restorable)

2. **Template Collection** — Create at least 2-3 templates per category:

   **Form Filling:**
   - "Fill Web Form" — `Go to {{url}} and fill out the form with the following information: {{form_data}}. Review all fields before clicking Submit.`
   - "Create Account" — `Navigate to {{url}} and create a new account using email {{email}} and the provided details: {{details}}. Complete all required fields marked with *.`

   **Web Navigation:**
   - "Search and Navigate" — `Open {{browser}} and search for "{{search_query}}". Click on the most relevant result and summarize what you find.`
   - "Download File" — `Navigate to {{url}} and download {{file_description}}. Save it to the default downloads folder.`

   **Data Entry:**
   - "Spreadsheet Data Entry" — `Open the spreadsheet at {{file_path}} and enter the following data starting at cell {{start_cell}}: {{data}}. Verify each entry after typing.`
   - "Fill Database Form" — `In the application, navigate to {{section}} and enter these records: {{records}}. Confirm each entry is saved.`

   **Data Extraction:**
   - "Extract Table Data" — `Go to {{url}} and extract all data from the {{table_description}} table. Report the data in a structured format in your completion message.`
   - "Read and Report" — `Open {{file_or_url}} and find {{information_needed}}. Report your findings when complete.`

   **Text Editing:**
   - "Compose Email" — `Open {{email_app}} and compose a new email to {{recipient}} with subject "{{subject}}". Write: {{message_content}}. Review before sending.`
   - "Edit Document" — `Open {{file_path}} and make the following changes: {{changes}}. Save the file when done.`

   **File Management:**
   - "Organize Files" — `In {{folder_path}}, organize files by {{criteria}}. Create subfolders as needed and move files accordingly.`
   - "Batch Rename" — `In {{folder_path}}, rename all {{file_pattern}} files using the pattern {{new_pattern}}.`

   **App Interaction:**
   - "Change App Settings" — `Open {{application}} settings/preferences and change {{setting_name}} to {{setting_value}}. Confirm the change was saved.`
   - "Install Application" — `Download and install {{application_name}} from {{source}}. Follow the installation wizard with default settings.`

3. **Integrate with template loading** in `src-tauri/src/config/settings.rs` or `lib.rs`:
   - On first launch (or when no user templates exist), populate with built-in templates
   - Add `is_builtin: bool` field to TaskTemplate (default false, serde default)
   - Built-in templates have `is_builtin: true` and a stable `id` (not random UUID) so they can be identified
   - On app update, new built-in templates can be added without duplicating existing ones (check by id)

4. **Add "Reset to Defaults" option** in frontend:
   - In settings template section, add a "Restore Default Templates" button
   - This re-adds any missing built-in templates without affecting user-created ones

5. **Frontend display** in `src/main.js`:
   - Show a small indicator (e.g., lock icon or "Built-in" badge) on built-in templates in the list
   - Built-in templates should not show a delete button (or show "Hide" instead of "Delete")
   - Built-in templates CAN be edited (creates a user copy, keeps original)

6. **Add tests**:
   - Test `get_builtin_templates()` returns expected count and categories
   - Test that all built-in templates have valid variables (parseable {{}} syntax)
   - Test first-launch population logic
   - Test that built-in templates survive config saves/loads

## Acceptance Criteria

- [ ] At least 12 built-in templates across all categories ship with the app
- [ ] Built-in templates have proper categories, variables, and well-written instructions
- [ ] First launch populates template list with built-in templates
- [ ] Built-in templates are marked and visually distinct from user templates
- [ ] "Restore Default Templates" button works
- [ ] Built-in templates use {{variables}} that prompt for user input
- [ ] All tests pass

## Files to Create/Modify

- `src-tauri/src/config/builtin_templates.rs` - NEW: Built-in template definitions
- `src-tauri/src/config/mod.rs` - Add `pub mod builtin_templates;`
- `src-tauri/src/config/settings.rs` - Add `is_builtin` field, first-launch population logic
- `src-tauri/src/lib.rs` - Add restore_defaults command, protect built-in templates from deletion
- `src/main.js` - Built-in template badges, hide instead of delete, restore defaults button
- `index.html` - Restore defaults button HTML, built-in badge CSS

## Integration Points

- **Provides**: Ready-to-use templates for users
- **Consumes**: Template variable system (template-variables), category system (template-edit-categories), task type alignment (case-specific-prompts)
- **Conflicts**: Touches template UI in main.js — coordinate with template-variables and template-edit-categories
