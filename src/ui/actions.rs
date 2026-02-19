use chrono::NaiveDate;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Entry, Label, ListBox, ListBoxRow};
use std::cell::RefCell;
use std::rc::Rc;

use crate::date_parser::{parse_color, parse_date, parse_priority};
use crate::keybindings::Action;
use crate::todo::{FlatTodo, Priority, Todo, TodoList};
use super::list_view::{create_inline_entry_row, get_entry_from_row, move_selection, refresh_list_with_settings};
use super::types::{DisplaySettings, InputMode};

pub(crate) struct ParsedInput {
    pub text: String,
    pub raw_text: String,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub color: Option<String>,
}

pub(crate) fn parse_task_input(raw: &str) -> ParsedInput {
    let raw_text = raw.to_string();
    let (text_after_priority, priority) = parse_priority(raw);
    let (text_after_date, due_date) = parse_date(&text_after_priority);
    let (text, color_raw) = parse_color(&text_after_date);
    let color = color_raw.and_then(|c| if c == "none" { None } else { Some(c) });
    ParsedInput { text, raw_text, priority, due_date, color }
}

pub(crate) struct ActionContext {
    pub todos: Rc<RefCell<TodoList>>,
    pub list_box: ListBox,
    pub flat_todos: Rc<RefCell<Vec<FlatTodo>>>,
    pub display_settings: Rc<RefCell<DisplaySettings>>,
    pub inline_entry_row: Rc<RefCell<Option<ListBoxRow>>>,
    pub input_mode: Rc<RefCell<InputMode>>,
    pub mode_label: Label,
    pub command_entry: Entry,
}

pub(crate) fn execute_action(action: Action, ctx: &ActionContext) -> gdk::glib::Propagation {
    match action {
        Action::MoveDown => {
            move_selection(&ctx.list_box, 1);
        }
        Action::MoveUp => {
            move_selection(&ctx.list_box, -1);
        }
        Action::JumpToFirst => {
            if let Some(first) = ctx.list_box.row_at_index(0) {
                ctx.list_box.select_row(Some(&first));
            }
        }
        Action::JumpToLast => {
            let count = ctx.flat_todos.borrow().len() as i32;
            if count > 0 {
                if let Some(last) = ctx.list_box.row_at_index(count - 1) {
                    ctx.list_box.select_row(Some(&last));
                }
            }
        }
        Action::ToggleComplete => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let task_id = flat_todo.todo.id.clone();
                    drop(flat);
                    ctx.todos.borrow_mut().toggle_at_path(&path);
                    refresh_list_with_settings(&ctx.todos, &ctx.list_box, &ctx.flat_todos, &ctx.display_settings);
                    let new_flat = ctx.flat_todos.borrow();
                    let new_index = new_flat.iter().position(|ft| ft.todo.id == task_id).unwrap_or(index);
                    drop(new_flat);
                    if let Some(new_row) = ctx.list_box.row_at_index(new_index as i32) {
                        ctx.list_box.select_row(Some(&new_row));
                    }
                }
            }
        }
        Action::Abandon => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let task_id = flat_todo.todo.id.clone();
                    drop(flat);
                    ctx.todos.borrow_mut().abandon_at_path(&path);
                    refresh_list_with_settings(&ctx.todos, &ctx.list_box, &ctx.flat_todos, &ctx.display_settings);
                    let new_flat = ctx.flat_todos.borrow();
                    let new_index = new_flat.iter().position(|ft| ft.todo.id == task_id).unwrap_or(index);
                    drop(new_flat);
                    if let Some(new_row) = ctx.list_box.row_at_index(new_index as i32) {
                        ctx.list_box.select_row(Some(&new_row));
                    }
                }
            }
        }
        Action::Delete => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    drop(flat);
                    ctx.todos.borrow_mut().remove_at_path(&path);
                    refresh_list_with_settings(&ctx.todos, &ctx.list_box, &ctx.flat_todos, &ctx.display_settings);
                    let new_count = ctx.flat_todos.borrow().len() as i32;
                    if new_count > 0 {
                        let new_index = (index as i32).min(new_count - 1);
                        if let Some(new_row) = ctx.list_box.row_at_index(new_index) {
                            ctx.list_box.select_row(Some(&new_row));
                        }
                    }
                }
            }
        }
        Action::MoveTaskDown => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    drop(flat);
                    if ctx.todos.borrow_mut().move_down(&path) {
                        refresh_list_with_settings(&ctx.todos, &ctx.list_box, &ctx.flat_todos, &ctx.display_settings);
                        let new_flat = ctx.flat_todos.borrow();
                        for (i, ft) in new_flat.iter().enumerate() {
                            if ft.path.len() == path.len() {
                                let mut new_path = path.clone();
                                if let Some(last) = new_path.last_mut() {
                                    *last += 1;
                                }
                                if ft.path == new_path {
                                    if let Some(new_row) = ctx.list_box.row_at_index(i as i32) {
                                        ctx.list_box.select_row(Some(&new_row));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        Action::MoveTaskUp => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    drop(flat);
                    if ctx.todos.borrow_mut().move_up(&path) {
                        refresh_list_with_settings(&ctx.todos, &ctx.list_box, &ctx.flat_todos, &ctx.display_settings);
                        let new_flat = ctx.flat_todos.borrow();
                        for (i, ft) in new_flat.iter().enumerate() {
                            if ft.path.len() == path.len() {
                                let mut new_path = path.clone();
                                if let Some(last) = new_path.last_mut() {
                                    if *last > 0 {
                                        *last -= 1;
                                    }
                                }
                                if ft.path == new_path {
                                    if let Some(new_row) = ctx.list_box.row_at_index(i as i32) {
                                        ctx.list_box.select_row(Some(&new_row));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        Action::ToggleFold => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let id = flat_todo.todo.id.clone();
                    drop(flat);
                    ctx.todos.borrow_mut().toggle_fold(&id);
                    refresh_list_with_settings(&ctx.todos, &ctx.list_box, &ctx.flat_todos, &ctx.display_settings);
                    if let Some(new_row) = ctx.list_box.row_at_index(index as i32) {
                        ctx.list_box.select_row(Some(&new_row));
                    }
                }
            }
        }
        Action::Insert => {
            *ctx.input_mode.borrow_mut() = InputMode::Insert;
            ctx.mode_label.set_text("INSERT");
            setup_inline_insert(ctx, 0, None, None);
        }
        Action::InsertSubtask => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let depth = flat_todo.depth + 1;
                    drop(flat);
                    *ctx.input_mode.borrow_mut() = InputMode::InsertSubtask(path.clone());
                    ctx.mode_label.set_text("INSERT (subtask)");
                    setup_inline_insert(ctx, depth, Some(index as i32), Some(path));
                }
            }
        }
        Action::Edit => {
            if let Some(row) = ctx.list_box.selected_row() {
                let index = row.index() as usize;
                let flat = ctx.flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let current_text = if flat_todo.todo.raw_text.is_empty() {
                        flat_todo.todo.text.clone()
                    } else {
                        flat_todo.todo.raw_text.clone()
                    };
                    drop(flat);
                    *ctx.input_mode.borrow_mut() = InputMode::Edit(path);
                    ctx.mode_label.set_text("EDIT");
                    ctx.command_entry.set_placeholder_text(Some(""));
                    ctx.command_entry.set_text(&current_text);
                    ctx.command_entry.set_sensitive(true);
                    ctx.command_entry.grab_focus();
                    ctx.command_entry.set_position(-1);
                }
            }
        }
        Action::CommandMode => {
            *ctx.input_mode.borrow_mut() = InputMode::Command;
            ctx.mode_label.set_text("COMMAND");
            ctx.command_entry.set_placeholder_text(Some(""));
            ctx.command_entry.set_text(":");
            ctx.command_entry.set_sensitive(true);
            ctx.command_entry.grab_focus();
            ctx.command_entry.set_position(-1);
        }
        Action::Cancel => {
            // Handled in the main key handler
        }
    }
    gdk::glib::Propagation::Stop
}

fn setup_inline_insert(
    ctx: &ActionContext,
    depth: usize,
    insert_after: Option<i32>,
    parent_path: Option<Vec<usize>>,
) {
    let placeholder = if parent_path.is_some() { "New subtask..." } else { "New task..." };
    let entry_row = create_inline_entry_row(depth, placeholder);

    match insert_after {
        Some(idx) => ctx.list_box.insert(&entry_row, idx + 1),
        None => ctx.list_box.append(&entry_row),
    }
    *ctx.inline_entry_row.borrow_mut() = Some(entry_row.clone());

    let Some(entry) = get_entry_from_row(&entry_row) else { return };

    let todos_c = ctx.todos.clone();
    let list_box_c = ctx.list_box.clone();
    let flat_todos_c = ctx.flat_todos.clone();
    let display_settings_c = ctx.display_settings.clone();
    let input_mode_c = ctx.input_mode.clone();
    let mode_label_c = ctx.mode_label.clone();
    let inline_entry_row_c = ctx.inline_entry_row.clone();
    let is_subtask = parent_path.is_some();

    entry.connect_activate(move |e| {
        let text = e.text().to_string();
        if !text.trim().is_empty() {
            if let Some(section_name) = text.trim().strip_prefix("/section ") {
                if !section_name.trim().is_empty() {
                    let todo = Todo::new_section(section_name.trim().to_string());
                    if let Some(ref path) = parent_path {
                        todos_c.borrow_mut().add_subtask(path, todo);
                    } else {
                        todos_c.borrow_mut().add(todo);
                    }
                }
            } else {
                let parsed = parse_task_input(&text);
                if !parsed.text.trim().is_empty() {
                    let todo = Todo::new(parsed.text, parsed.due_date, parsed.priority, parsed.raw_text, parsed.color);
                    if let Some(ref path) = parent_path {
                        todos_c.borrow_mut().add_subtask(path, todo);
                    } else {
                        todos_c.borrow_mut().add(todo);
                    }
                }
            }
        }
        if let Some(row) = inline_entry_row_c.borrow_mut().take() {
            list_box_c.remove(&row);
        }
        refresh_list_with_settings(&todos_c, &list_box_c, &flat_todos_c, &display_settings_c);
        *input_mode_c.borrow_mut() = InputMode::Normal;
        mode_label_c.set_text("NORMAL");
        list_box_c.grab_focus();
        if !is_subtask {
            let count = flat_todos_c.borrow().len() as i32;
            if count > 0 {
                if let Some(last) = list_box_c.row_at_index(count - 1) {
                    list_box_c.select_row(Some(&last));
                }
            }
        }
    });

    if is_subtask {
        let entry_for_focus = entry.clone();
        gdk::glib::idle_add_local_once(move || {
            entry_for_focus.grab_focus();
        });
    } else {
        entry.grab_focus();
    }
}
