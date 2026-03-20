use crate::todo::FlatTodo;
use chrono::{Datelike, Local, NaiveDate};

/// Result of a filter operation
pub struct FilterResult {
    pub visible: bool,
    pub error: Option<String>,
}

impl FilterResult {
    fn success(visible: bool) -> Self {
        Self { visible, error: None }
    }

    #[allow(dead_code)]
    fn error(msg: String) -> Self {
        Self { visible: true, error: Some(msg) }
    }
}

/// Determines if a task should be visible based on a filter string.
/// Filter string format: property1-param1;property2-param2
pub fn is_task_visible(flat_todo: &FlatTodo, filter_str: &str) -> FilterResult {
    if filter_str.trim().is_empty() {
        return FilterResult::success(true);
    }

    let parts: Vec<&str> = filter_str.split(';').collect();
    for part in parts {
        let part = part.trim();
        if part.is_empty() { continue; }

        let (property, param) = if let Some(idx) = part.find('-') {
            (part[..idx].trim().to_lowercase(), part[idx + 1..].trim())
        } else {
            (part.to_lowercase(), "")
        };

        let result = match property.as_str() {
            "in" => filter_in(flat_todo, param),
            "date" => filter_date(flat_todo, param),
            _ => FilterResult::success(true), // Unknown property, default to visible
        };

        if !result.visible {
            return result;
        }
    }

    FilterResult::success(true)
}

/// "in" property: returns true if the task is in the specified section or any of its subtasks.
fn filter_in(flat_todo: &FlatTodo, section_name: &str) -> FilterResult {
    if section_name.is_empty() {
        return FilterResult::success(true);
    }

    // If it's a top-level task with no parents, we include it as per instruction:
    // "If the task object/struct does not contain parent task/section then include it."
    if flat_todo.hierarchy_path.is_empty() {
        return FilterResult::success(true);
    }

    // Check if any parent section matches the name
    let visible = flat_todo.hierarchy_path.iter().any(|p| p.to_lowercase() == section_name.to_lowercase());
    FilterResult::success(visible)
}

/// "date" property: handles date comparisons.
fn filter_date(flat_todo: &FlatTodo, param: &str) -> FilterResult {
    if param.is_empty() {
        return FilterResult::success(true);
    }

    // If it's a top-level task with no parents, we include it
    if flat_todo.hierarchy_path.is_empty() {
        return FilterResult::success(true);
    }

    let Some(due_date) = flat_todo.todo.due_date else {
        return FilterResult::success(false);
    };

    // Handle "=2/2" format
    if let Some(rest) = param.strip_prefix('=') {
        if let Some(target_date) = parse_simple_date(rest) {
            return FilterResult::success(due_date == target_date);
        }
    }

    // Handle other operators if needed later, for now just return true on parse error as per instruction
    FilterResult::success(true)
}

/// Parses "M/D" format for the current year.
fn parse_simple_date(s: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 { return None; }

    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year = Local::now().year();

    NaiveDate::from_ymd_opt(year, month, day)
}
