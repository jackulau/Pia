use super::settings::TaskTemplate;
use chrono::Utc;

/// Returns the complete set of built-in templates that ship with Pia.
/// Each template has a stable ID (prefixed with "builtin-") so it can be
/// identified across app updates and never duplicated.
pub fn get_builtin_templates() -> Vec<TaskTemplate> {
    let now = Utc::now();

    vec![
        // ── Form Filling ──────────────────────────────────────────────
        TaskTemplate {
            id: "builtin-fill-web-form".to_string(),
            name: "Fill Web Form".to_string(),
            instruction: "Go to {{url}} and fill out the form with the following information: {{form_data}}. Review all fields before clicking Submit.".to_string(),
            category: "Form Filling".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-create-account".to_string(),
            name: "Create Account".to_string(),
            instruction: "Navigate to {{url}} and create a new account using email {{email}} and the provided details: {{details}}. Complete all required fields marked with *.".to_string(),
            category: "Form Filling".to_string(),
            is_builtin: true,
            created_at: now,
        },

        // ── Web Navigation ────────────────────────────────────────────
        TaskTemplate {
            id: "builtin-search-and-navigate".to_string(),
            name: "Search and Navigate".to_string(),
            instruction: "Open {{browser}} and search for \"{{search_query}}\". Click on the most relevant result and summarize what you find.".to_string(),
            category: "Web Navigation".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-download-file".to_string(),
            name: "Download File".to_string(),
            instruction: "Navigate to {{url}} and download {{file_description}}. Save it to the default downloads folder.".to_string(),
            category: "Web Navigation".to_string(),
            is_builtin: true,
            created_at: now,
        },

        // ── Data Entry ────────────────────────────────────────────────
        TaskTemplate {
            id: "builtin-spreadsheet-data-entry".to_string(),
            name: "Spreadsheet Data Entry".to_string(),
            instruction: "Open the spreadsheet at {{file_path}} and enter the following data starting at cell {{start_cell}}: {{data}}. Verify each entry after typing.".to_string(),
            category: "Data Entry".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-fill-database-form".to_string(),
            name: "Fill Database Form".to_string(),
            instruction: "In the application, navigate to {{section}} and enter these records: {{records}}. Confirm each entry is saved.".to_string(),
            category: "Data Entry".to_string(),
            is_builtin: true,
            created_at: now,
        },

        // ── Data Extraction ───────────────────────────────────────────
        TaskTemplate {
            id: "builtin-extract-table-data".to_string(),
            name: "Extract Table Data".to_string(),
            instruction: "Go to {{url}} and extract all data from the {{table_description}} table. Report the data in a structured format in your completion message.".to_string(),
            category: "Data Extraction".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-read-and-report".to_string(),
            name: "Read and Report".to_string(),
            instruction: "Open {{file_or_url}} and find {{information_needed}}. Report your findings when complete.".to_string(),
            category: "Data Extraction".to_string(),
            is_builtin: true,
            created_at: now,
        },

        // ── Text Editing ──────────────────────────────────────────────
        TaskTemplate {
            id: "builtin-compose-email".to_string(),
            name: "Compose Email".to_string(),
            instruction: "Open {{email_app}} and compose a new email to {{recipient}} with subject \"{{subject}}\". Write: {{message_content}}. Review before sending.".to_string(),
            category: "Text Editing".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-edit-document".to_string(),
            name: "Edit Document".to_string(),
            instruction: "Open {{file_path}} and make the following changes: {{changes}}. Save the file when done.".to_string(),
            category: "Text Editing".to_string(),
            is_builtin: true,
            created_at: now,
        },

        // ── File Management ───────────────────────────────────────────
        TaskTemplate {
            id: "builtin-organize-files".to_string(),
            name: "Organize Files".to_string(),
            instruction: "In {{folder_path}}, organize files by {{criteria}}. Create subfolders as needed and move files accordingly.".to_string(),
            category: "File Management".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-batch-rename".to_string(),
            name: "Batch Rename".to_string(),
            instruction: "In {{folder_path}}, rename all {{file_pattern}} files using the pattern {{new_pattern}}.".to_string(),
            category: "File Management".to_string(),
            is_builtin: true,
            created_at: now,
        },

        // ── App Interaction ───────────────────────────────────────────
        TaskTemplate {
            id: "builtin-change-app-settings".to_string(),
            name: "Change App Settings".to_string(),
            instruction: "Open {{application}} settings/preferences and change {{setting_name}} to {{setting_value}}. Confirm the change was saved.".to_string(),
            category: "App Interaction".to_string(),
            is_builtin: true,
            created_at: now,
        },
        TaskTemplate {
            id: "builtin-install-application".to_string(),
            name: "Install Application".to_string(),
            instruction: "Download and install {{application_name}} from {{source}}. Follow the installation wizard with default settings.".to_string(),
            category: "App Interaction".to_string(),
            is_builtin: true,
            created_at: now,
        },
    ]
}

/// Returns the set of built-in template IDs for quick lookup.
pub fn builtin_template_ids() -> Vec<String> {
    get_builtin_templates()
        .iter()
        .map(|t| t.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_builtin_template_count() {
        let templates = get_builtin_templates();
        assert!(
            templates.len() >= 12,
            "Expected at least 12 built-in templates, got {}",
            templates.len()
        );
    }

    #[test]
    fn test_builtin_templates_have_stable_ids() {
        let templates = get_builtin_templates();
        for t in &templates {
            assert!(
                t.id.starts_with("builtin-"),
                "Built-in template '{}' should have id starting with 'builtin-', got '{}'",
                t.name,
                t.id
            );
        }
    }

    #[test]
    fn test_builtin_templates_unique_ids() {
        let templates = get_builtin_templates();
        let mut ids = HashSet::new();
        for t in &templates {
            assert!(
                ids.insert(&t.id),
                "Duplicate built-in template ID: {}",
                t.id
            );
        }
    }

    #[test]
    fn test_builtin_templates_have_categories() {
        let templates = get_builtin_templates();
        let expected_categories: HashSet<&str> = [
            "Form Filling",
            "Web Navigation",
            "Data Entry",
            "Data Extraction",
            "Text Editing",
            "File Management",
            "App Interaction",
        ]
        .iter()
        .copied()
        .collect();

        let actual_categories: HashSet<&str> =
            templates.iter().map(|t| t.category.as_str()).collect();

        for cat in &expected_categories {
            assert!(
                actual_categories.contains(cat),
                "Missing expected category: {}",
                cat
            );
        }
    }

    /// Simple helper to find {{variable}} occurrences without regex.
    fn count_template_variables(instruction: &str) -> usize {
        let mut count = 0;
        let bytes = instruction.as_bytes();
        let mut i = 0;
        while i + 3 < bytes.len() {
            if bytes[i] == b'{' && bytes[i + 1] == b'{' {
                // Look for closing }}
                if let Some(end) = instruction[i + 2..].find("}}") {
                    let var_name = &instruction[i + 2..i + 2 + end];
                    if !var_name.is_empty()
                        && var_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                    {
                        count += 1;
                        i = i + 2 + end + 2;
                        continue;
                    }
                }
            }
            i += 1;
        }
        count
    }

    #[test]
    fn test_builtin_templates_have_valid_variables() {
        let templates = get_builtin_templates();

        for t in &templates {
            let var_count = count_template_variables(&t.instruction);
            assert!(
                var_count > 0,
                "Built-in template '{}' has no {{{{variables}}}} in its instruction",
                t.name
            );
            // Check no malformed variable syntax (e.g., {{{triple}}})
            assert!(
                !t.instruction.contains("{{{"),
                "Built-in template '{}' has malformed triple-brace variable",
                t.name
            );
        }
    }

    #[test]
    fn test_all_builtin_templates_marked_builtin() {
        let templates = get_builtin_templates();
        for t in &templates {
            assert!(
                t.is_builtin,
                "Built-in template '{}' should have is_builtin=true",
                t.name
            );
        }
    }

    #[test]
    fn test_builtin_templates_have_non_empty_names() {
        let templates = get_builtin_templates();
        for t in &templates {
            assert!(!t.name.is_empty(), "Built-in template has empty name");
            assert!(
                !t.instruction.is_empty(),
                "Built-in template '{}' has empty instruction",
                t.name
            );
        }
    }

    #[test]
    fn test_builtin_template_ids_helper() {
        let ids = builtin_template_ids();
        let templates = get_builtin_templates();
        assert_eq!(ids.len(), templates.len());
        for id in &ids {
            assert!(id.starts_with("builtin-"));
        }
    }
}
