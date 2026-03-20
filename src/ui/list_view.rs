use chrono::{DateTime, Datelike, Local, Utc};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Entry, Label, ListBox, ListBoxRow, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::filters::is_task_visible;
use crate::todo::{FlatTodo, Priority, TodoList};
use super::types::DisplaySettings;

pub(crate) fn create_todo_row(flat_todo: &FlatTodo, settings: &DisplaySettings) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.add_css_class("todo-row");

    let outer_hbox = GtkBox::new(Orientation::Horizontal, 0);

    for color in &flat_todo.inherited_colors {
        let bar = GtkBox::new(Orientation::Vertical, 0);
        bar.set_width_request(4);
        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_data(&format!("box {{ background-color: {}; }}", color));
        bar.style_context().add_provider(&css_provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
        outer_hbox.append(&bar);
    }

    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    let indent = if settings.flattened { 0 } else { flat_todo.depth as i32 * 20 };
    hbox.set_margin_start(8 + indent);
    hbox.set_margin_end(8);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    if flat_todo.todo.is_section {
        row.add_css_class("section-row");

        if flat_todo.has_subtasks && !settings.flattened {
            let chevron = if flat_todo.is_folded { "▶" } else { "▼" };
            let chevron_label = Label::new(Some(chevron));
            chevron_label.add_css_class("fold-chevron");
            hbox.append(&chevron_label);
        }

        let section_color = flat_todo.inherited_colors.last();

        let marker = Label::new(Some("§"));
        if let Some(color) = section_color {
            let css_provider = gtk4::CssProvider::new();
            css_provider.load_from_data(&format!("label {{ color: {}; }}", color));
            marker.style_context().add_provider(&css_provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
        } else {
            marker.add_css_class("section-marker");
        }
        hbox.append(&marker);

        if settings.flattened && !flat_todo.hierarchy_path.is_empty() {
            let path_text = format!("{}/", flat_todo.hierarchy_path.join("/"));
            let path_label = Label::new(Some(&path_text));
            path_label.add_css_class("hierarchy-path");
            hbox.append(&path_label);
        }

        let text_label = Label::new(Some(&flat_todo.todo.text));
        text_label.set_hexpand(true);
        text_label.set_halign(gtk4::Align::Start);
        if let Some(color) = section_color {
            let css_provider = gtk4::CssProvider::new();
            css_provider.load_from_data(&format!(
                "label {{ color: {}; font-family: monospace; font-weight: bold; font-size: 14px; }}",
                color
            ));
            text_label.style_context().add_provider(&css_provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2);
        } else {
            text_label.add_css_class("section-text");
        }
        hbox.append(&text_label);

        hbox.set_hexpand(true);
        outer_hbox.append(&hbox);
        row.set_child(Some(&outer_hbox));
        return row;
    }

    if !settings.flattened {
        if flat_todo.has_subtasks {
            let chevron = if flat_todo.is_folded { "▶" } else { "▼" };
            let chevron_label = Label::new(Some(chevron));
            chevron_label.add_css_class("fold-chevron");
            hbox.append(&chevron_label);
        } else if flat_todo.depth > 0 {
            let ind = Label::new(Some("└"));
            ind.add_css_class("subtask-indicator");
            hbox.append(&ind);
        } else {
            let spacer = Label::new(Some(" "));
            spacer.add_css_class("fold-spacer");
            hbox.append(&spacer);
        }
    }

    if flat_todo.todo.priority == Priority::Max {
        row.add_css_class("priority-max-row");
    }

    let priority_label = Label::new(Some("●"));
    match flat_todo.todo.priority {
        Priority::Max => priority_label.add_css_class("priority-max"),
        Priority::High => priority_label.add_css_class("priority-high"),
        Priority::Medium => priority_label.add_css_class("priority-medium"),
        Priority::Low => priority_label.add_css_class("priority-low"),
        Priority::None => priority_label.add_css_class("priority-none"),
    }
    hbox.append(&priority_label);

    let check_label = if flat_todo.todo.abandoned {
        let label = Label::new(Some(" ! "));
        label.add_css_class("abandoned-marker");
        row.add_css_class("abandoned-row");
        label
    } else {
        let check = if flat_todo.todo.completed { "[x]" } else { "[ ]" };
        let label = Label::new(Some(check));
        label.add_css_class("todo-check");
        label
    };

    if settings.flattened && !flat_todo.hierarchy_path.is_empty() {
        let path_text = format!("{}/", flat_todo.hierarchy_path.join("/"));
        let path_label = Label::new(Some(&path_text));
        path_label.add_css_class("hierarchy-path");
        hbox.append(&path_label);
    }

    let text_label = Label::new(Some(&flat_todo.todo.text));
    text_label.set_hexpand(true);
    text_label.set_halign(gtk4::Align::Start);
    if flat_todo.todo.completed {
        text_label.add_css_class("completed");
    }
    if flat_todo.todo.abandoned {
        text_label.add_css_class("abandoned-text");
    }

    hbox.append(&check_label);
    hbox.append(&text_label);

    if settings.show_start_date {
        let created: DateTime<Utc> = DateTime::from_timestamp(flat_todo.todo.created_at, 0)
            .unwrap_or_else(Utc::now);
        let created_local = created.with_timezone(&Local);
        let current_year = Local::now().year();
        let start_str = if created_local.year() != current_year {
            created_local.format("%b %d, %Y").to_string()
        } else {
            created_local.format("%b %d").to_string()
        };
        let start_label = Label::new(Some(&format!("+ {}", start_str)));
        start_label.add_css_class("start-date");
        hbox.append(&start_label);
    }

    if let Some(due) = flat_todo.todo.due_date {
        let current_year = Local::now().year();
        let date_str = if due.year() != current_year {
            due.format("%b %d, %Y").to_string()
        } else {
            due.format("%b %d").to_string()
        };
        let date_label = Label::new(Some(&format!("→ {}", date_str)));
        date_label.add_css_class("due-date");
        hbox.append(&date_label);
    }

    hbox.set_hexpand(true);
    outer_hbox.append(&hbox);
    row.set_child(Some(&outer_hbox));
    row
}

pub(crate) fn move_selection(list_box: &ListBox, delta: i32) {
    if let Some(row) = list_box.selected_row() {
        let current = row.index();
        let next_index = current + delta;
        if next_index >= 0 {
            if let Some(next_row) = list_box.row_at_index(next_index) {
                list_box.select_row(Some(&next_row));
            }
        }
    } else if let Some(first) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first));
    }
}
pub(crate) fn refresh_list_with_settings(
    todos: &Rc<RefCell<TodoList>>,
    list_box: &ListBox,
    flat_todos: &Rc<RefCell<Vec<FlatTodo>>>,
    display_settings: &Rc<RefCell<DisplaySettings>>,
    tab_filter: &Option<String>,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let todos_ref = todos.borrow();
    let settings = display_settings.borrow();
    let flat = if settings.flattened { todos_ref.flatten_all() } else { todos_ref.flatten() };

    let today = Local::now().date_naive();
    let filtered_flat: Vec<FlatTodo> = flat.into_iter().filter(|ft| {
        // First check completed status
        if ft.todo.completed {
            if !ft.todo.due_date.map_or(true, |d| d >= today) {
                return false;
            }
        }

        // Then check filter if one exists for this tab
        if let Some(ref filter_str) = tab_filter {
            is_task_visible(ft, filter_str).visible
        } else {
            true
        }
    }).collect();
    let display_flat: Vec<FlatTodo> = if settings.flattened {
        let mut sorted: Vec<FlatTodo> = filtered_flat.into_iter().filter(|ft| !ft.todo.is_section).collect();
        sorted.sort_by(|a, b| {
            if a.todo.abandoned != b.todo.abandoned {
                return a.todo.abandoned.cmp(&b.todo.abandoned);
            }
            if a.todo.completed != b.todo.completed {
                return a.todo.completed.cmp(&b.todo.completed);
            }
            let priority_order = |p: &Priority| match p {
                Priority::Max => 0,
                Priority::High => 1,
                Priority::Medium => 2,
                Priority::Low => 3,
                Priority::None => 4,
            };
            let priority_cmp = priority_order(&a.todo.priority).cmp(&priority_order(&b.todo.priority));
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }
            let date_cmp = match (&a.todo.due_date, &b.todo.due_date) {
                (Some(da), Some(db)) => da.cmp(db),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };
            if date_cmp != std::cmp::Ordering::Equal {
                return date_cmp;
            }
            a.todo.text.to_lowercase().cmp(&b.todo.text.to_lowercase())
        });
        sorted
    } else {
        let mut non_abandoned: Vec<FlatTodo> = Vec::new();
        let mut abandoned: Vec<FlatTodo> = Vec::new();
        for ft in filtered_flat {
            if ft.todo.abandoned {
                abandoned.push(ft);
            } else {
                non_abandoned.push(ft);
            }
        }
        non_abandoned.extend(abandoned);
        non_abandoned
    };

    for flat_todo in &display_flat {
        let row = create_todo_row(flat_todo, &settings);
        list_box.append(&row);
    }

    *flat_todos.borrow_mut() = display_flat;
}

pub(crate) fn create_inline_entry_row(depth: usize, placeholder: &str) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.add_css_class("inline-entry-row");
    row.set_selectable(false);

    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_start(8 + (depth as i32 * 20));
    hbox.set_margin_end(8);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);

    let indicator = Label::new(Some(">"));
    indicator.add_css_class("insert-indicator");
    hbox.append(&indicator);

    let entry = Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.add_css_class("inline-entry");
    entry.set_hexpand(true);
    hbox.append(&entry);

    row.set_child(Some(&hbox));
    row
}

pub(crate) fn get_entry_from_row(row: &ListBoxRow) -> Option<Entry> {
    let hbox = row.child()?.downcast::<GtkBox>().ok()?;
    let mut child = hbox.first_child();
    child = child?.next_sibling();
    child?.downcast::<Entry>().ok()
}
