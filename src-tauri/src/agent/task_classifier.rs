/// Task type classification for injecting specialized system prompt guidance.
///
/// Detects the kind of computer use task from the user's instruction and
/// returns task-specific tips that improve agent performance.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    FormFilling,
    WebNavigation,
    DataEntry,
    DataExtraction,
    FileManagement,
    TextEditing,
    AppInteraction,
    General,
}

/// Keyword group: each entry is (keyword, weight).
/// Weight lets multi-word phrases count more than single words.
struct KeywordGroup {
    task_type: TaskType,
    keywords: &'static [(&'static str, u32)],
}

const KEYWORD_GROUPS: &[KeywordGroup] = &[
    KeywordGroup {
        task_type: TaskType::FormFilling,
        keywords: &[
            ("fill out", 3),
            ("fill in", 3),
            ("input field", 3),
            ("sign up", 3),
            ("checkout", 2),
            ("register", 2),
            ("submit", 2),
            ("form", 2),
            ("fill", 1),
        ],
    },
    KeywordGroup {
        task_type: TaskType::WebNavigation,
        keywords: &[
            ("navigate to", 3),
            ("go to", 3),
            ("search for", 3),
            ("click on", 2),
            ("browse", 2),
            ("open", 1),
            ("url", 2),
            ("website", 2),
            ("web page", 3),
        ],
    },
    KeywordGroup {
        task_type: TaskType::DataEntry,
        keywords: &[
            ("enter data", 3),
            ("spreadsheet", 3),
            ("excel", 3),
            ("table", 2),
            ("cell", 1),
            ("row", 1),
            ("column", 1),
        ],
    },
    KeywordGroup {
        task_type: TaskType::DataExtraction,
        keywords: &[
            ("find the value", 4),
            ("extract", 2),
            ("scrape", 2),
            ("copy", 1),
            ("read the", 2),
            ("get the", 2),
        ],
    },
    KeywordGroup {
        task_type: TaskType::FileManagement,
        keywords: &[
            ("save as", 3),
            ("delete file", 3),
            ("rename", 2),
            ("move file", 3),
            ("download", 2),
            ("folder", 2),
            ("file", 1),
        ],
    },
    KeywordGroup {
        task_type: TaskType::TextEditing,
        keywords: &[
            ("edit text", 3),
            ("compose", 2),
            ("draft", 2),
            ("email", 2),
            ("document", 2),
            ("write", 1),
            ("type", 1),
        ],
    },
    KeywordGroup {
        task_type: TaskType::AppInteraction,
        keywords: &[
            ("settings", 2),
            ("preferences", 2),
            ("menu", 2),
            ("dialog", 2),
            ("configure", 2),
            ("toggle", 2),
        ],
    },
];

/// Minimum total score required to classify as a specific task type.
const MIN_SCORE: u32 = 2;

/// Classify a user instruction into a task type using weighted keyword matching.
pub fn classify_instruction(instruction: &str) -> TaskType {
    let lower = instruction.to_lowercase();

    let mut best_type = TaskType::General;
    let mut best_score: u32 = 0;

    for group in KEYWORD_GROUPS {
        let score: u32 = group
            .keywords
            .iter()
            .filter(|(kw, _)| lower.contains(kw))
            .map(|(_, weight)| weight)
            .sum();

        if score > best_score {
            best_score = score;
            best_type = group.task_type.clone();
        }
    }

    if best_score >= MIN_SCORE {
        best_type
    } else {
        TaskType::General
    }
}

/// Return task-specific guidance text to append to the system prompt.
/// Returns an empty string for `General` so no extra content is injected.
pub fn get_task_specific_guidance(task_type: &TaskType) -> &'static str {
    match task_type {
        TaskType::FormFilling => {
            "Task-specific guidance (form filling):\n\
             - Tab between fields rather than clicking each one for faster entry.\n\
             - Check for required field indicators (*) before submitting.\n\
             - After filling all fields, scroll to review them before clicking Submit.\n\
             - Look for validation errors after submission and correct them.\n\
             - For dropdowns, click to open, then select the desired option."
        }
        TaskType::WebNavigation => {
            "Task-specific guidance (web navigation):\n\
             - Wait for pages to fully load before acting; look for loading spinners.\n\
             - Use the address bar to enter URLs directly when possible.\n\
             - For search, type the query and press Enter.\n\
             - Check the current URL to verify you are on the correct page.\n\
             - If a link is not visible, scroll down to find it before retrying."
        }
        TaskType::DataEntry => {
            "Task-specific guidance (data entry):\n\
             - Click the target cell, type the value, then press Tab or Enter to confirm.\n\
             - Double-check values after entry for accuracy.\n\
             - Use keyboard shortcuts (Ctrl+C/V) for copy/paste operations.\n\
             - Navigate between cells with Tab (right) and Enter (down).\n\
             - If a cell is not editable, try double-clicking it first."
        }
        TaskType::DataExtraction => {
            "Task-specific guidance (data extraction):\n\
             - Triple-click to select entire lines of text.\n\
             - Use Ctrl+A to select all content if needed.\n\
             - Use Ctrl+C to copy selected content.\n\
             - Scroll methodically through the content to ensure nothing is missed.\n\
             - Report extracted data in your Complete message."
        }
        TaskType::FileManagement => {
            "Task-specific guidance (file management):\n\
             - Right-click for context menus when renaming, deleting, or moving files.\n\
             - Use keyboard shortcuts (F2 to rename, Delete to remove) when available.\n\
             - Verify the destination folder before moving or saving files.\n\
             - Wait for file operations to complete before proceeding.\n\
             - Check the file list or folder after the operation to confirm success."
        }
        TaskType::TextEditing => {
            "Task-specific guidance (text editing):\n\
             - Use Ctrl+A to select all, Ctrl+Home/End for navigation.\n\
             - Use Shift+arrow keys for precise text selection.\n\
             - Check spelling and formatting before completing.\n\
             - Use Ctrl+Z to undo mistakes immediately.\n\
             - Place the cursor at the correct position before typing."
        }
        TaskType::AppInteraction => {
            "Task-specific guidance (application interaction):\n\
             - Look for menus at the top of the window (File, Edit, View, etc.).\n\
             - Check for toggle switches, checkboxes, or radio buttons in settings.\n\
             - Dialogs often have OK/Cancel buttons at the bottom; scroll if needed.\n\
             - After changing a setting, look for a Save or Apply button.\n\
             - If a menu item is grayed out, check if a prerequisite action is needed."
        }
        TaskType::General => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- FormFilling tests --
    #[test]
    fn test_classify_form_filling_fill_out() {
        assert_eq!(classify_instruction("Fill out the registration form"), TaskType::FormFilling);
    }

    #[test]
    fn test_classify_form_filling_sign_up() {
        assert_eq!(classify_instruction("Sign up for a new account"), TaskType::FormFilling);
    }

    #[test]
    fn test_classify_form_filling_checkout() {
        assert_eq!(classify_instruction("Complete the checkout form and submit"), TaskType::FormFilling);
    }

    // -- WebNavigation tests --
    #[test]
    fn test_classify_web_navigation_navigate() {
        assert_eq!(classify_instruction("Navigate to google.com"), TaskType::WebNavigation);
    }

    #[test]
    fn test_classify_web_navigation_search() {
        assert_eq!(classify_instruction("Search for rust tutorials on the web"), TaskType::WebNavigation);
    }

    #[test]
    fn test_classify_web_navigation_go_to() {
        assert_eq!(classify_instruction("Go to the settings page on the website"), TaskType::WebNavigation);
    }

    // -- DataEntry tests --
    #[test]
    fn test_classify_data_entry_spreadsheet() {
        assert_eq!(classify_instruction("Enter data into the spreadsheet"), TaskType::DataEntry);
    }

    #[test]
    fn test_classify_data_entry_excel() {
        assert_eq!(classify_instruction("Put the numbers into the Excel table"), TaskType::DataEntry);
    }

    #[test]
    fn test_classify_data_entry_cells() {
        assert_eq!(classify_instruction("Enter data into row 5 column B"), TaskType::DataEntry);
    }

    // -- DataExtraction tests --
    #[test]
    fn test_classify_data_extraction_extract() {
        assert_eq!(classify_instruction("Extract the phone numbers from the page"), TaskType::DataExtraction);
    }

    #[test]
    fn test_classify_data_extraction_find_value() {
        assert_eq!(classify_instruction("Find the value of the total in the report"), TaskType::DataExtraction);
    }

    #[test]
    fn test_classify_data_extraction_scrape() {
        assert_eq!(classify_instruction("Scrape the prices from this page"), TaskType::DataExtraction);
    }

    // -- FileManagement tests --
    #[test]
    fn test_classify_file_management_save_as() {
        assert_eq!(classify_instruction("Save as a PDF in the Downloads folder"), TaskType::FileManagement);
    }

    #[test]
    fn test_classify_file_management_rename() {
        assert_eq!(classify_instruction("Rename the file to report_final.docx"), TaskType::FileManagement);
    }

    #[test]
    fn test_classify_file_management_delete() {
        assert_eq!(classify_instruction("Delete file old_backup.zip from Desktop"), TaskType::FileManagement);
    }

    // -- TextEditing tests --
    #[test]
    fn test_classify_text_editing_compose() {
        assert_eq!(classify_instruction("Compose an email to the team"), TaskType::TextEditing);
    }

    #[test]
    fn test_classify_text_editing_draft() {
        assert_eq!(classify_instruction("Draft a document summarizing the meeting"), TaskType::TextEditing);
    }

    #[test]
    fn test_classify_text_editing_edit() {
        assert_eq!(classify_instruction("Edit text in the document header"), TaskType::TextEditing);
    }

    // -- AppInteraction tests --
    #[test]
    fn test_classify_app_interaction_settings() {
        assert_eq!(classify_instruction("Open settings and toggle dark mode"), TaskType::AppInteraction);
    }

    #[test]
    fn test_classify_app_interaction_preferences() {
        assert_eq!(classify_instruction("Change preferences in the settings dialog"), TaskType::AppInteraction);
    }

    #[test]
    fn test_classify_app_interaction_configure() {
        assert_eq!(classify_instruction("Configure the proxy settings in the dialog"), TaskType::AppInteraction);
    }

    // -- General fallback --
    #[test]
    fn test_classify_general_ambiguous() {
        assert_eq!(classify_instruction("Do the thing"), TaskType::General);
    }

    #[test]
    fn test_classify_general_empty() {
        assert_eq!(classify_instruction(""), TaskType::General);
    }

    #[test]
    fn test_classify_general_random() {
        assert_eq!(classify_instruction("hello world"), TaskType::General);
    }

    // -- Guidance tests --
    #[test]
    fn test_guidance_non_empty_for_all_specific_types() {
        let types = [
            TaskType::FormFilling,
            TaskType::WebNavigation,
            TaskType::DataEntry,
            TaskType::DataExtraction,
            TaskType::FileManagement,
            TaskType::TextEditing,
            TaskType::AppInteraction,
        ];
        for t in &types {
            let guidance = get_task_specific_guidance(t);
            assert!(!guidance.is_empty(), "Guidance for {:?} should not be empty", t);
            // At least 3 tips (lines starting with "- ")
            let tip_count = guidance.lines().filter(|l| l.trim_start().starts_with("- ")).count();
            assert!(tip_count >= 3, "Guidance for {:?} should have at least 3 tips, found {}", t, tip_count);
        }
    }

    #[test]
    fn test_guidance_empty_for_general() {
        assert_eq!(get_task_specific_guidance(&TaskType::General), "");
    }

    // -- Case insensitivity --
    #[test]
    fn test_classify_case_insensitive() {
        assert_eq!(classify_instruction("FILL OUT THE FORM"), TaskType::FormFilling);
        assert_eq!(classify_instruction("Navigate To Google"), TaskType::WebNavigation);
    }
}
