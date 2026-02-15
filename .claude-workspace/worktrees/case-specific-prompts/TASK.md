---
id: case-specific-prompts
name: Task-Type Detection and Case-Specific System Prompts
wave: 1
priority: 1
dependencies: []
estimated_hours: 5
tags: [backend, agent, prompts]
---

## Objective

Add task-type classification that detects what kind of computer use task the user wants, then inject specialized system prompt guidance to dramatically improve agent performance on case-specific tasks.

## Context

Currently the system prompt in `src-tauri/src/llm/provider.rs` is completely generic — the same prompt is used whether the user says "fill out this form", "navigate to a website", "extract data from a spreadsheet", or "write an email". This means the agent gets no task-specific guidance about best practices, common patterns, or strategies for different use cases.

By detecting the task type from the user's instruction and injecting specialized prompt sections, we can significantly improve the agent's performance on common computer use scenarios.

## Implementation

1. **Create task-type classifier** — Add a new file `src-tauri/src/agent/task_classifier.rs`:
   - Define a `TaskType` enum with common categories:
     - `FormFilling` — filling out web forms, registration, checkout
     - `WebNavigation` — browsing, searching, clicking through pages
     - `DataEntry` — entering data into spreadsheets, tables, databases
     - `DataExtraction` — reading/copying data from screen
     - `FileManagement` — opening, saving, moving, renaming files
     - `TextEditing` — writing, editing documents, emails
     - `AppInteraction` — general application interaction (menus, settings, dialogs)
     - `General` — fallback for unclassified tasks
   - Implement `classify_instruction(instruction: &str) -> TaskType` using keyword matching:
     - `FormFilling`: "fill", "form", "submit", "register", "sign up", "checkout", "input field"
     - `WebNavigation`: "navigate", "go to", "open", "browse", "search for", "click on", "url"
     - `DataEntry`: "enter data", "spreadsheet", "table", "cell", "row", "column", "excel"
     - `DataExtraction`: "extract", "copy", "read", "scrape", "get the", "find the value"
     - `FileManagement`: "file", "folder", "save as", "rename", "move", "delete file", "download"
     - `TextEditing`: "write", "type", "compose", "draft", "email", "document", "edit text"
     - `AppInteraction`: "settings", "preferences", "menu", "dialog", "configure", "toggle"
   - Return `General` if no category matches with sufficient confidence
   - Allow multiple matches, return the highest-confidence one

2. **Create task-specific prompt sections** — Add a method `get_task_specific_guidance(task_type: &TaskType) -> String`:
   Each task type gets specialized guidance. Examples:

   **FormFilling**:
   - "When filling forms: Tab between fields rather than clicking each one. Check for required field indicators (*). After filling, scroll to review all fields before submitting. Look for validation errors after submission. For dropdowns, click to open then select the option."

   **WebNavigation**:
   - "When navigating: Wait for pages to fully load before acting. Look for loading indicators. Use the address bar for URLs. For search, type the query and press Enter. Check the current URL to verify you're on the right page."

   **DataEntry**:
   - "When entering data into cells: Click the target cell, type the value, then press Tab or Enter to confirm and move to the next cell. Double-check values after entry. Use keyboard shortcuts (Ctrl+C/V) for copy/paste operations."

   **DataExtraction**:
   - "When extracting data: Triple-click to select entire lines. Use Ctrl+A to select all if needed. Use Ctrl+C to copy. Scroll methodically through the content. Report extracted data in your Complete message."

   **TextEditing**:
   - "When editing text: Use Ctrl+A to select all, Ctrl+Home/End for navigation. Use Shift+arrow keys for precise selection. Check spelling and formatting. Use Ctrl+Z to undo mistakes."

3. **Integrate with system prompt builders** in `src-tauri/src/llm/provider.rs`:
   - Modify `build_system_prompt_for_tools()` to accept the user instruction, classify it, and append task-specific guidance
   - Modify `build_system_prompt()` similarly
   - Pass the instruction through from `loop_runner.rs` when creating the provider/prompts

4. **Update loop_runner.rs** minimally:
   - Pass the user instruction to the system prompt builder so it can classify and customize
   - This should be a minimal change — just threading the instruction parameter through

5. **Add tests**:
   - Test classification of various instructions into correct categories
   - Test that task-specific guidance is non-empty for each type
   - Test that the system prompt includes task-specific content when instruction matches a category

## Acceptance Criteria

- [ ] `classify_instruction()` correctly categorizes at least 3 example instructions per task type
- [ ] Each task type has meaningful specialized guidance (at least 3 tips each)
- [ ] System prompts include task-specific guidance when instruction matches a category
- [ ] `General` fallback works for ambiguous instructions (no extra guidance added)
- [ ] Existing tests still pass
- [ ] New tests cover classification and prompt generation

## Files to Create/Modify

- `src-tauri/src/agent/task_classifier.rs` - NEW: Task type enum, classification logic, guidance text
- `src-tauri/src/agent/mod.rs` - Add `pub mod task_classifier;`
- `src-tauri/src/llm/provider.rs` - Modify system prompt builders to accept instruction and inject task-specific guidance
- `src-tauri/src/agent/loop_runner.rs` - Pass instruction to prompt builders (minimal change)

## Integration Points

- **Provides**: Task-type classification that builtin-templates can leverage for categorization
- **Consumes**: User instruction text
- **Conflicts**: Minimal changes to `loop_runner.rs` — coordinate with other tasks that touch it. Main work is in new file + `provider.rs`
