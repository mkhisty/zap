use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Entry, EventControllerKey, Label, Notebook};
use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks;
use crate::todo::{Todo, TodoList};
use super::actions::parse_task_input;
use super::calendar_view::{create_calendar_view, refresh_calendar_view};
use super::list_view::refresh_list_with_settings;
use super::tab::TabContent;
use super::types::{DisplaySettings, InputMode, ViewType};

pub(crate) fn setup_entry_handler(
    command_entry: &Entry,
    todos: &Rc<RefCell<TodoList>>,
    tabs: &Rc<RefCell<Vec<TabContent>>>,
    notebook: &Notebook,
    mode_label: &Label,
    notification_label: &Label,
    input_mode: &Rc<RefCell<InputMode>>,
    display_settings: &Rc<RefCell<DisplaySettings>>,
) {
    let shared_todos = todos.clone();
    let tabs = tabs.clone();
    let notebook = notebook.clone();
    let mode_label = mode_label.clone();
    let notification_label = notification_label.clone();
    let input_mode = input_mode.clone();
    let display_settings = display_settings.clone();

    command_entry.connect_activate(move |e| {
        let text = e.text().to_string();
        let mode = input_mode.borrow().clone();

        notification_label.set_visible(false);

        let current_page = match notebook.current_page() {
            Some(p) => p as usize,
            None => {
                e.set_text("");
                e.set_sensitive(false);
                *input_mode.borrow_mut() = InputMode::Normal;
                mode_label.set_text("NORMAL");
                return;
            }
        };
        let tabs_ref = tabs.borrow();
        let tab = match tabs_ref.get(current_page) {
            Some(t) => t,
            None => {
                e.set_text("");
                e.set_sensitive(false);
                *input_mode.borrow_mut() = InputMode::Normal;
                mode_label.set_text("NORMAL");
                return;
            }
        };

        let todos = shared_todos.clone();
        let list_box = tab.list_box.clone();
        let flat_todos = tab.flat_todos.clone();
        drop(tabs_ref);

        match mode {
            InputMode::Command => {
                let cmd = text.trim();
                if cmd == ":display_start" {
                    let mut settings = display_settings.borrow_mut();
                    settings.show_start_date = !settings.show_start_date;
                    drop(settings);
                    refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                } else if let Some(view_name) = cmd.strip_prefix(":e ") {
                    match view_name.trim() {
                        "calendar" | "cal" => {
                            let mut tabs_mut = tabs.borrow_mut();
                            let tab = &mut tabs_mut[current_page];
                            *tab.view_type.borrow_mut() = ViewType::Calendar;
                            if tab.calendar_state.borrow().is_none() {
                                create_calendar_view(&tab.scrolled_calendar, &tab.calendar_state);
                            } else {
                                refresh_calendar_view(&tab.calendar_state);
                            }
                            tab.content_stack.set_visible_child_name("calendar");
                            tab.tab_label_widget.set_text("[cal]");
                        }
                        "list" => {
                            let mut tabs_mut = tabs.borrow_mut();
                            let tab = &mut tabs_mut[current_page];
                            *tab.view_type.borrow_mut() = ViewType::List;
                            tab.content_stack.set_visible_child_name("list");
                            tab.tab_label_widget.set_text(&format!("{}", current_page + 1));
                            tab.list_box.grab_focus();
                        }
                        name => {
                            // Extension point: to add a new view, implement TabView and add a match
                            // arm above this. This fallback activates an already-loaded plugin view.
                            let mut tabs_mut = tabs.borrow_mut();
                            let tab = &mut tabs_mut[current_page];
                            let has_view = tab.plugin_view.borrow().as_ref()
                                .map(|v| v.view_name() == name)
                                .unwrap_or(false);
                            if has_view {
                                *tab.view_type.borrow_mut() = ViewType::Plugin(name.to_string());
                                tab.plugin_view.borrow().as_ref().unwrap().refresh(&todos.borrow());
                                tab.content_stack.set_visible_child_name("plugin");
                                tab.tab_label_widget.set_text(&format!("[{}]", name));
                            }
                        }
                    }
                } else if cmd == ":sort" {
                    todos.borrow_mut().sort();
                    refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                    notification_label.set_text("Tasks sorted");
                    notification_label.remove_css_class("notification-error");
                    notification_label.set_visible(true);
                    let nl = notification_label.clone();
                    gtk4::glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
                        nl.set_visible(false);
                        gtk4::glib::ControlFlow::Break
                    });
                } else if cmd == ":flatten" {
                    let mut settings = display_settings.borrow_mut();
                    settings.flattened = !settings.flattened;
                    let is_flat = settings.flattened;
                    drop(settings);
                    refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                    notification_label.set_text(if is_flat { "Flattened view" } else { "Hierarchical view" });
                    notification_label.remove_css_class("notification-error");
                    notification_label.set_visible(true);
                    let nl = notification_label.clone();
                    gtk4::glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                        nl.set_visible(false);
                        gtk4::glib::ControlFlow::Break
                    });
                }
                // Extension point: external commands in ~/.config/zap/commands/<name>
                else if let Some(cmd_name) = cmd.strip_prefix(':') {
                    let parts: Vec<&str> = cmd_name.splitn(2, ' ').collect();
                    let name = parts[0];
                    let arg = parts.get(1).copied().unwrap_or("");
                    let script = dirs::config_dir()
                        .unwrap_or_default()
                        .join("zap/commands")
                        .join(name);
                    if script.exists() {
                        std::process::Command::new(&script)
                            .arg(arg)
                            .env("ZAP_CLUSTER", "main")
                            .spawn()
                            .ok();
                    }
                }
            }
            InputMode::Edit(ref path) => {
                if !text.trim().is_empty() {
                    let parsed = parse_task_input(&text);
                    if !parsed.text.trim().is_empty() {
                        let task_text = parsed.text.clone();
                        let task_id = flat_todos.borrow().iter()
                            .find(|ft| ft.path == *path)
                            .map(|ft| ft.todo.id.clone());
                        todos.borrow_mut().update_at_path(
                            path,
                            parsed.text,
                            parsed.due_date,
                            parsed.priority,
                            parsed.raw_text,
                            parsed.color,
                        );
                        hooks::fire(hooks::HookEvent::TaskEdit, task_id.as_deref(), Some(&task_text));
                        refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                    }
                }
            }
            InputMode::CalendarInsert(date) => {
                if !text.trim().is_empty() {
                    let parsed = parse_task_input(&text);
                    if !parsed.text.trim().is_empty() {
                        // Use the calendar date, ignore any parsed date from text
                        let todo = Todo::new(
                            parsed.text,
                            Some(date),
                            parsed.priority,
                            parsed.raw_text,
                            parsed.color,
                        );
                        todos.borrow_mut().add(todo);
                        let tabs_ref = tabs.borrow();
                        if let Some(tab) = tabs_ref.get(current_page) {
                            if *tab.view_type.borrow() == ViewType::Calendar {
                                refresh_calendar_view(&tab.calendar_state);
                            }
                        }
                    }
                }
            }
            // Insert modes are handled by inline entries, not this handler
            InputMode::Insert | InputMode::InsertSubtask(_) | InputMode::Normal => {}
        }

        e.set_text("");
        e.set_sensitive(false);
        *input_mode.borrow_mut() = InputMode::Normal;
        mode_label.set_text("NORMAL");
        list_box.grab_focus();
    });
}

pub(crate) fn setup_entry_autocomplete(
    command_entry: &Entry,
    input_mode: &Rc<RefCell<InputMode>>,
) {
    let command_entry_c = command_entry.clone();
    let input_mode = input_mode.clone();

    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key != gdk::Key::Tab {
            return gdk::glib::Propagation::Proceed;
        }

        let mode = input_mode.borrow().clone();
        if mode != InputMode::Command {
            return gdk::glib::Propagation::Proceed;
        }

        let text = command_entry_c.text().to_string();
        if let Some(completed) = autocomplete_command(&text) {
            command_entry_c.set_text(&completed);
            command_entry_c.set_position(-1);
        }

        gdk::glib::Propagation::Stop
    });

    command_entry.add_controller(key_controller);
}

fn autocomplete_command(input: &str) -> Option<String> {
    let commands = [":e calendar", ":e list", ":sort", ":flatten", ":display_start"];
    for cmd in &commands {
        if cmd.starts_with(input) && *cmd != input {
            return Some(cmd.to_string());
        }
    }
    None
}
