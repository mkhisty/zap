use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Color configuration for the application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    // Main backgrounds
    pub main_bg: String,
    pub todo_row_bg: String,
    pub todo_row_selected: String,

    // Priority colors
    pub priority_low: String,
    pub priority_medium: String,
    pub priority_high: String,
    pub priority_max: String,
    pub priority_max_bg: String,
    pub priority_none: String,

    // Text colors
    pub text_primary: String,
    pub text_secondary: String,
    pub text_completed: String,

    // UI element colors
    pub cluster_title: String,
    pub mode_indicator: String,
    pub notification: String,
    pub notification_error: String,
    pub help_text: String,

    // Entry/command bar colors
    pub command_bar_bg: String,
    pub command_bar_text: String,
    pub command_bar_border: String,
    pub command_bar_disabled_bg: String,
    pub command_bar_disabled_text: String,

    // Task indicator colors
    pub checkbox_color: String,
    pub due_date_color: String,
    pub start_date_color: String,
    pub subtask_indicator: String,
    pub fold_chevron: String,

    // Section colors
    pub section_bg: String,
    pub section_border: String,
    pub section_text: String,

    // Insert mode colors
    pub insert_indicator: String,

    // Abandoned task colors
    #[serde(default = "default_abandoned_marker")]
    pub abandoned_marker: String,
    #[serde(default = "default_abandoned_text")]
    pub abandoned_text: String,
}

fn default_abandoned_marker() -> String {
    "#e06c75".to_string()
}

fn default_abandoned_text() -> String {
    "#5c6370".to_string()
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            // Main backgrounds
            main_bg: "#1e1e1e".to_string(),
            todo_row_bg: "#2d2d2d".to_string(),
            todo_row_selected: "#3e4451".to_string(),

            // Priority colors
            priority_low: "#56b6c2".to_string(),
            priority_medium: "#e5c07b".to_string(),
            priority_high: "#e06c75".to_string(),
            priority_max: "#e06c75".to_string(),
            priority_max_bg: "#5c2f2f".to_string(),
            priority_none: "#3e3e3e".to_string(),

            // Text colors
            text_primary: "#abb2bf".to_string(),
            text_secondary: "#5c6370".to_string(),
            text_completed: "#5c6370".to_string(),

            // UI element colors
            cluster_title: "#c678dd".to_string(),
            mode_indicator: "#98c379".to_string(),
            notification: "#e5c07b".to_string(),
            notification_error: "#e06c75".to_string(),
            help_text: "#5c6370".to_string(),

            // Entry/command bar colors
            command_bar_bg: "#2d2d2d".to_string(),
            command_bar_text: "#abb2bf".to_string(),
            command_bar_border: "#3e3e3e".to_string(),
            command_bar_disabled_bg: "#252525".to_string(),
            command_bar_disabled_text: "#5c6370".to_string(),

            // Task indicator colors
            checkbox_color: "#61afef".to_string(),
            due_date_color: "#e5c07b".to_string(),
            start_date_color: "#56b6c2".to_string(),
            subtask_indicator: "#5c6370".to_string(),
            fold_chevron: "#61afef".to_string(),

            // Section colors
            section_bg: "#252525".to_string(),
            section_border: "#c678dd".to_string(),
            section_text: "#c678dd".to_string(),

            // Insert mode colors
            insert_indicator: "#98c379".to_string(),

            // Abandoned task colors
            abandoned_marker: "#e06c75".to_string(),
            abandoned_text: "#5c6370".to_string(),
        }
    }
}

impl ColorConfig {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }

        // Create default config
        let config = Self::default();
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            fs::write(&path, json).ok();
        }
        config
    }

    fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("zap");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("colors.json")
    }

    /// Generate CSS from the color configuration
    pub fn generate_css(&self) -> String {
        format!(
            r#"
            .main-container {{
                background-color: {main_bg};
            }}

            .cluster-title {{
                color: {cluster_title};
                font-family: monospace;
                font-weight: bold;
                font-size: 14px;
            }}

            .mode-indicator {{
                color: {mode_indicator};
                font-family: monospace;
                font-weight: bold;
                font-size: 12px;
            }}

            .notification {{
                color: {notification};
                font-family: monospace;
                font-size: 12px;
                margin-bottom: 4px;
            }}

            .notification-error {{
                color: {notification_error};
            }}

            .command-bar {{
                background-color: {command_bar_bg};
                color: {command_bar_text};
                border: 1px solid {command_bar_border};
                border-radius: 4px;
                padding: 8px;
                font-family: monospace;
                font-size: 14px;
            }}

            .command-bar:disabled {{
                background-color: {command_bar_disabled_bg};
                color: {command_bar_disabled_text};
            }}

            .todo-list {{
                background-color: transparent;
            }}

            .todo-row {{
                background-color: {todo_row_bg};
                border-radius: 4px;
                margin-bottom: 4px;
            }}

            .todo-row:selected {{
                background-color: {todo_row_selected};
            }}

            .priority-max-row {{
                background-color: {priority_max_bg};
            }}

            .priority-max-row:selected {{
                background-color: {priority_max_bg};
            }}

            .todo-check {{
                color: {checkbox_color};
                font-family: monospace;
                font-size: 14px;
            }}

            .completed {{
                color: {text_completed};
                text-decoration: line-through;
            }}

            .due-date {{
                color: {due_date_color};
                font-size: 12px;
                font-family: monospace;
            }}

            .start-date {{
                color: {start_date_color};
                font-size: 12px;
                font-family: monospace;
            }}

            .help-text {{
                color: {help_text};
                font-size: 11px;
                font-family: monospace;
            }}

            .subtask-indicator {{
                color: {subtask_indicator};
                font-family: monospace;
            }}

            .fold-chevron {{
                color: {fold_chevron};
                font-family: monospace;
                font-size: 10px;
                min-width: 12px;
            }}

            .fold-spacer {{
                min-width: 12px;
            }}

            .priority-high {{
                color: {priority_high};
                font-size: 12px;
            }}

            .priority-medium {{
                color: {priority_medium};
                font-size: 12px;
            }}

            .priority-low {{
                color: {priority_low};
                font-size: 12px;
            }}

            .priority-max {{
                color: {priority_max};
                font-size: 12px;
            }}

            .priority-none {{
                color: {priority_none};
                font-size: 12px;
            }}

            .section-row {{
                background-color: {section_bg};
                border-left: 3px solid {section_border};
            }}

            .section-marker {{
                color: {section_text};
                font-family: monospace;
                font-weight: bold;
                font-size: 14px;
            }}

            .section-text {{
                color: {section_text};
                font-family: monospace;
                font-weight: bold;
                font-size: 14px;
            }}

            .inline-entry-row {{
                background-color: {todo_row_bg};
                border-radius: 4px;
                margin-bottom: 4px;
            }}

            .insert-indicator {{
                color: {insert_indicator};
                font-family: monospace;
                font-weight: bold;
            }}

            .inline-entry {{
                background-color: transparent;
                border: none;
                color: {text_primary};
                font-family: monospace;
                font-size: 14px;
            }}

            .hierarchy-path {{
                color: {text_secondary};
                font-family: monospace;
                font-size: 13px;
                font-style: italic;
            }}

            /* Abandoned task styles */
            .abandoned-marker {{
                color: {abandoned_marker};
                font-family: monospace;
                font-weight: bold;
                font-size: 14px;
            }}

            .abandoned-text {{
                color: {abandoned_text};
            }}

            .abandoned-row {{
                opacity: 0.7;
            }}

            /* Calendar styles */
            .calendar-header {{
                color: {cluster_title};
                font-family: monospace;
                font-weight: bold;
                font-size: 16px;
                margin-bottom: 8px;
            }}

            .calendar-day-header {{
                color: {text_secondary};
                font-family: monospace;
                font-size: 12px;
                padding: 4px;
            }}

            .calendar-grid {{
                background-color: transparent;
            }}

            .calendar-day {{
                background-color: {todo_row_bg};
                border-radius: 4px;
                min-height: 80px;
                min-width: 60px;
            }}

            .calendar-day-number {{
                color: {text_primary};
                font-family: monospace;
                font-weight: bold;
                font-size: 12px;
            }}

            .calendar-today {{
                border: 2px solid {mode_indicator};
            }}

            .calendar-selected {{
                background-color: {todo_row_selected};
            }}

            .calendar-task {{
                color: {text_primary};
                font-family: monospace;
                font-size: 10px;
                padding: 1px 0;
            }}

            .calendar-task-completed {{
                color: {text_completed};
                text-decoration: line-through;
            }}

            .calendar-task-more {{
                color: {text_secondary};
                font-family: monospace;
                font-size: 10px;
                font-style: italic;
            }}

            .calendar-task-max {{
                color: {priority_max};
            }}

            .calendar-task-high {{
                color: {priority_high};
            }}

            .calendar-task-medium {{
                color: {priority_medium};
            }}

            .calendar-nav-btn {{
                background-color: {todo_row_bg};
                color: {cluster_title};
                border: 1px solid {command_bar_border};
                border-radius: 4px;
                padding: 4px 12px;
                font-family: monospace;
                font-size: 14px;
                min-width: 32px;
            }}

            .calendar-nav-btn:hover {{
                background-color: {todo_row_selected};
            }}
        "#,
            main_bg = self.main_bg,
            cluster_title = self.cluster_title,
            mode_indicator = self.mode_indicator,
            notification = self.notification,
            notification_error = self.notification_error,
            command_bar_bg = self.command_bar_bg,
            command_bar_text = self.command_bar_text,
            command_bar_border = self.command_bar_border,
            command_bar_disabled_bg = self.command_bar_disabled_bg,
            command_bar_disabled_text = self.command_bar_disabled_text,
            todo_row_bg = self.todo_row_bg,
            todo_row_selected = self.todo_row_selected,
            priority_max_bg = self.priority_max_bg,
            checkbox_color = self.checkbox_color,
            text_completed = self.text_completed,
            due_date_color = self.due_date_color,
            start_date_color = self.start_date_color,
            help_text = self.help_text,
            subtask_indicator = self.subtask_indicator,
            fold_chevron = self.fold_chevron,
            priority_high = self.priority_high,
            priority_medium = self.priority_medium,
            priority_low = self.priority_low,
            priority_max = self.priority_max,
            priority_none = self.priority_none,
            section_bg = self.section_bg,
            section_border = self.section_border,
            section_text = self.section_text,
            insert_indicator = self.insert_indicator,
            text_primary = self.text_primary,
            text_secondary = self.text_secondary,
            abandoned_marker = self.abandoned_marker,
            abandoned_text = self.abandoned_text,
        )
    }
}
