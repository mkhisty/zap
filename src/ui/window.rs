use chrono::{DateTime, Datelike, Local, Utc};
use gtk4::prelude::*;
use gtk4::{
    gdk, glib, Application, ApplicationWindow, Box as GtkBox, Entry, EventControllerKey, Label,
    ListBox, ListBoxRow, Orientation, ScrolledWindow, SelectionMode,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::colors::ColorConfig;
use crate::date_parser::{parse_date, parse_priority};
use crate::keybindings::{Action, Keybindings};
use crate::todo::{FlatTodo, Priority, Todo, TodoList};

#[derive(Clone, Debug, PartialEq)]
enum InputMode {
    Normal,                      // Not in input mode
    Insert,                      // Adding a new top-level task
    InsertSubtask(Vec<usize>),   // Adding a subtask under the path
    Edit(Vec<usize>),            // Editing task at path
    Command,                     // Command mode (started with :)
}

#[derive(Clone, Debug, Default)]
struct DisplaySettings {
    show_start_date: bool,
}

pub struct ZapWindow {
    pub window: ApplicationWindow,
    todos: Rc<RefCell<TodoList>>,
    list_box: ListBox,
    command_entry: Entry,  // Bottom entry for command mode only
    mode_label: Label,
    cluster_label: Label,
    notification_label: Label,
    input_mode: Rc<RefCell<InputMode>>,
    pending_key: Rc<RefCell<Option<String>>>,  // For key sequences like gg, dd, za
    flat_todos: Rc<RefCell<Vec<FlatTodo>>>,
    display_settings: Rc<RefCell<DisplaySettings>>,
    inline_entry_row: Rc<RefCell<Option<ListBoxRow>>>,  // For insert modes
    keybindings: Rc<Keybindings>,
    color_config: Rc<ColorConfig>,
}

impl ZapWindow {
    pub fn new(app: &Application) -> Self {
        let todos = Rc::new(RefCell::new(TodoList::load("main")));
        let input_mode = Rc::new(RefCell::new(InputMode::Normal));
        let pending_key: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let flat_todos = Rc::new(RefCell::new(Vec::new()));
        let display_settings = Rc::new(RefCell::new(DisplaySettings::default()));
        let inline_entry_row: Rc<RefCell<Option<ListBoxRow>>> = Rc::new(RefCell::new(None));
        let keybindings = Rc::new(Keybindings::load());
        let color_config = Rc::new(ColorConfig::load());

        // Create window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Zap - main")
            .default_width(500)
            .default_height(600)
            .build();

        // Main container
        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.add_css_class("main-container");

        // Header with cluster name and mode
        let header_box = GtkBox::new(Orientation::Horizontal, 8);
        header_box.set_margin_start(12);
        header_box.set_margin_end(12);
        header_box.set_margin_top(8);
        header_box.set_margin_bottom(4);

        // Cluster title at top
        let cluster_label = Label::new(Some("main"));
        cluster_label.add_css_class("cluster-title");
        cluster_label.set_halign(gtk4::Align::Start);

        // Mode indicator
        let mode_label = Label::new(Some("NORMAL"));
        mode_label.add_css_class("mode-indicator");
        mode_label.set_halign(gtk4::Align::End);
        mode_label.set_hexpand(true);

        header_box.append(&cluster_label);
        header_box.append(&mode_label);

        // Notification label (hidden by default)
        let notification_label = Label::new(None);
        notification_label.add_css_class("notification");
        notification_label.set_margin_start(12);
        notification_label.set_visible(false);

        // Todo list
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::Single);
        list_box.add_css_class("todo-list");

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&list_box));
        scrolled.set_margin_start(12);
        scrolled.set_margin_end(12);
        scrolled.set_margin_bottom(8);

        // Help label
        let help_label = Label::new(Some("j/k: nav | J/K: reorder | Enter: toggle | dd: del | i: insert | e: edit | za: fold | :: cmd"));
        help_label.add_css_class("help-text");
        help_label.set_margin_bottom(4);

        // Command bar at bottom (for command mode only)
        let command_entry = Entry::new();
        command_entry.add_css_class("command-bar");
        command_entry.set_margin_start(12);
        command_entry.set_margin_end(12);
        command_entry.set_margin_bottom(8);
        command_entry.set_can_focus(true);
        command_entry.set_sensitive(false);

        // Layout: header, notification, scrolled list, help, command bar (bottom)
        main_box.append(&header_box);
        main_box.append(&notification_label);
        main_box.append(&scrolled);
        main_box.append(&help_label);
        main_box.append(&command_entry);

        window.set_child(Some(&main_box));

        let zap = Self {
            window,
            todos,
            list_box,
            command_entry,
            mode_label,
            cluster_label,
            notification_label,
            input_mode,
            pending_key,
            flat_todos,
            display_settings,
            inline_entry_row,
            keybindings,
            color_config,
        };

        zap.refresh_list();
        zap.setup_keybindings();
        zap.setup_entry_handler();
        zap.setup_entry_autocomplete();
        zap.apply_css();

        zap
    }

    fn setup_entry_autocomplete(&self) {
        let command_entry = self.command_entry.clone();
        let input_mode = self.input_mode.clone();

        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key != gdk::Key::Tab {
                return gdk::glib::Propagation::Proceed;
            }

            let mode = input_mode.borrow().clone();
            if mode != InputMode::Command {
                return gdk::glib::Propagation::Proceed;
            }

            let text = command_entry.text().to_string();
            if let Some(completed) = autocomplete_command(&text) {
                command_entry.set_text(&completed);
                command_entry.set_position(-1);
            }

            gdk::glib::Propagation::Stop
        });

        self.command_entry.add_controller(key_controller);
    }

    fn refresh_list(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let todos = self.todos.borrow();
        let flat = todos.flatten();
        let settings = self.display_settings.borrow();

        for flat_todo in &flat {
            let row = create_todo_row(flat_todo, &settings);
            self.list_box.append(&row);
        }

        *self.flat_todos.borrow_mut() = flat;

        if let Some(first_row) = self.list_box.row_at_index(0) {
            self.list_box.select_row(Some(&first_row));
        }
    }

    fn setup_keybindings(&self) {
        let key_controller = EventControllerKey::new();

        let todos = self.todos.clone();
        let list_box = self.list_box.clone();
        let command_entry = self.command_entry.clone();
        let mode_label = self.mode_label.clone();
        let input_mode = self.input_mode.clone();
        let pending_key = self.pending_key.clone();
        let flat_todos = self.flat_todos.clone();
        let refresh_todos = self.todos.clone();
        let refresh_list_box = self.list_box.clone();
        let refresh_flat_todos = self.flat_todos.clone();
        let refresh_display_settings = self.display_settings.clone();
        let inline_entry_row = self.inline_entry_row.clone();
        let keybindings = self.keybindings.clone();

        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            let mode = input_mode.borrow().clone();
            let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            let alt = modifier.contains(gdk::ModifierType::ALT_MASK);

            // Handle non-normal modes - only Escape works
            if mode != InputMode::Normal {
                if let Some(Action::Cancel) = keybindings.get_action(&key, shift, ctrl, alt) {
                    *input_mode.borrow_mut() = InputMode::Normal;
                    mode_label.set_text("NORMAL");
                    if let Some(row) = inline_entry_row.borrow_mut().take() {
                        list_box.remove(&row);
                    }
                    command_entry.set_sensitive(false);
                    command_entry.set_text("");
                    list_box.grab_focus();
                    return gdk::glib::Propagation::Stop;
                }
                return gdk::glib::Propagation::Proceed;
            }

            // Check for sequence completion first
            let pending = pending_key.borrow().clone();
            if let Some(ref pending_str) = pending {
                if let Some(action) = keybindings.get_sequence_action(pending_str, &key) {
                    *pending_key.borrow_mut() = None;
                    return execute_action(
                        action, &todos, &list_box, &command_entry, &mode_label,
                        &input_mode, &flat_todos, &refresh_todos, &refresh_list_box,
                        &refresh_flat_todos, &refresh_display_settings, &inline_entry_row,
                    );
                }
                // Invalid sequence, clear pending
                *pending_key.borrow_mut() = None;
            }

            // Check if this key starts a sequence
            if let Some(seq_start) = keybindings.is_sequence_start(&key) {
                *pending_key.borrow_mut() = Some(seq_start);
                return gdk::glib::Propagation::Stop;
            }

            // Check for single key action
            if let Some(action) = keybindings.get_action(&key, shift, ctrl, alt) {
                *pending_key.borrow_mut() = None;
                return execute_action(
                    action, &todos, &list_box, &command_entry, &mode_label,
                    &input_mode, &flat_todos, &refresh_todos, &refresh_list_box,
                    &refresh_flat_todos, &refresh_display_settings, &inline_entry_row,
                );
            }

            *pending_key.borrow_mut() = None;
            gdk::glib::Propagation::Proceed
        });

        self.window.add_controller(key_controller);
    }
}

/// Execute an action from keybindings
fn execute_action(
    action: Action,
    todos: &Rc<RefCell<TodoList>>,
    list_box: &ListBox,
    command_entry: &Entry,
    mode_label: &Label,
    input_mode: &Rc<RefCell<InputMode>>,
    flat_todos: &Rc<RefCell<Vec<FlatTodo>>>,
    refresh_todos: &Rc<RefCell<TodoList>>,
    refresh_list_box: &ListBox,
    refresh_flat_todos: &Rc<RefCell<Vec<FlatTodo>>>,
    refresh_display_settings: &Rc<RefCell<DisplaySettings>>,
    inline_entry_row: &Rc<RefCell<Option<ListBoxRow>>>,
) -> gdk::glib::Propagation {
    match action {
        Action::MoveDown => {
            move_selection(list_box, 1);
        }
        Action::MoveUp => {
            move_selection(list_box, -1);
        }
        Action::JumpToFirst => {
            if let Some(first) = list_box.row_at_index(0) {
                list_box.select_row(Some(&first));
            }
        }
        Action::JumpToLast => {
            let count = flat_todos.borrow().len() as i32;
            if count > 0 {
                if let Some(last) = list_box.row_at_index(count - 1) {
                    list_box.select_row(Some(&last));
                }
            }
        }
        Action::ToggleComplete => {
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let task_id = flat_todo.todo.id.clone();
                    drop(flat);
                    todos.borrow_mut().toggle_at_path(&path);
                    refresh_list_with_settings(refresh_todos, refresh_list_box, refresh_flat_todos, refresh_display_settings);
                    // Find the task by ID after refresh (it may have moved)
                    let new_flat = refresh_flat_todos.borrow();
                    let new_index = new_flat.iter().position(|ft| ft.todo.id == task_id).unwrap_or(index);
                    drop(new_flat);
                    if let Some(new_row) = refresh_list_box.row_at_index(new_index as i32) {
                        refresh_list_box.select_row(Some(&new_row));
                    }
                }
            }
        }
        Action::Delete => {
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    drop(flat);
                    todos.borrow_mut().remove_at_path(&path);
                    refresh_list_with_settings(refresh_todos, refresh_list_box, refresh_flat_todos, refresh_display_settings);
                    let new_count = refresh_flat_todos.borrow().len() as i32;
                    if new_count > 0 {
                        let new_index = (index as i32).min(new_count - 1);
                        if let Some(new_row) = refresh_list_box.row_at_index(new_index) {
                            refresh_list_box.select_row(Some(&new_row));
                        }
                    }
                }
            }
        }
        Action::MoveTaskDown => {
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    drop(flat);
                    if todos.borrow_mut().move_down(&path) {
                        refresh_list_with_settings(refresh_todos, refresh_list_box, refresh_flat_todos, refresh_display_settings);
                        let new_flat = refresh_flat_todos.borrow();
                        for (i, ft) in new_flat.iter().enumerate() {
                            if ft.path.len() == path.len() {
                                let mut new_path = path.clone();
                                if let Some(last) = new_path.last_mut() {
                                    *last += 1;
                                }
                                if ft.path == new_path {
                                    if let Some(new_row) = refresh_list_box.row_at_index(i as i32) {
                                        refresh_list_box.select_row(Some(&new_row));
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
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    drop(flat);
                    if todos.borrow_mut().move_up(&path) {
                        refresh_list_with_settings(refresh_todos, refresh_list_box, refresh_flat_todos, refresh_display_settings);
                        let new_flat = refresh_flat_todos.borrow();
                        for (i, ft) in new_flat.iter().enumerate() {
                            if ft.path.len() == path.len() {
                                let mut new_path = path.clone();
                                if let Some(last) = new_path.last_mut() {
                                    if *last > 0 {
                                        *last -= 1;
                                    }
                                }
                                if ft.path == new_path {
                                    if let Some(new_row) = refresh_list_box.row_at_index(i as i32) {
                                        refresh_list_box.select_row(Some(&new_row));
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
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let id = flat_todo.todo.id.clone();
                    drop(flat);
                    todos.borrow_mut().toggle_fold(&id);
                    refresh_list_with_settings(refresh_todos, refresh_list_box, refresh_flat_todos, refresh_display_settings);
                    if let Some(new_row) = refresh_list_box.row_at_index(index as i32) {
                        refresh_list_box.select_row(Some(&new_row));
                    }
                }
            }
        }
        Action::Insert => {
            *input_mode.borrow_mut() = InputMode::Insert;
            mode_label.set_text("INSERT");

            let entry_row = create_inline_entry_row(0, "New task...");
            list_box.append(&entry_row);
            *inline_entry_row.borrow_mut() = Some(entry_row.clone());

            if let Some(entry) = get_entry_from_row(&entry_row) {
                let todos_c = refresh_todos.clone();
                let list_box_c = refresh_list_box.clone();
                let flat_todos_c = refresh_flat_todos.clone();
                let display_settings_c = refresh_display_settings.clone();
                let input_mode_c = input_mode.clone();
                let mode_label_c = mode_label.clone();
                let inline_entry_row_c = inline_entry_row.clone();

                entry.connect_activate(move |e| {
                    let text = e.text().to_string();
                    if !text.trim().is_empty() {
                        if let Some(section_name) = text.trim().strip_prefix("/section ") {
                            if !section_name.trim().is_empty() {
                                let todo = Todo::new_section(section_name.trim().to_string());
                                todos_c.borrow_mut().add(todo);
                            }
                        } else {
                            let (text_after_priority, priority) = parse_priority(&text);
                            let (task_text, due_date) = parse_date(&text_after_priority);
                            if !task_text.trim().is_empty() {
                                let todo = Todo::new(task_text, due_date, priority);
                                todos_c.borrow_mut().add(todo);
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
                    let count = flat_todos_c.borrow().len() as i32;
                    if count > 0 {
                        if let Some(last) = list_box_c.row_at_index(count - 1) {
                            list_box_c.select_row(Some(&last));
                        }
                    }
                });
                entry.grab_focus();
            }
        }
        Action::InsertSubtask => {
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let depth = flat_todo.depth + 1;
                    drop(flat);

                    *input_mode.borrow_mut() = InputMode::InsertSubtask(path.clone());
                    mode_label.set_text("INSERT (subtask)");

                    let entry_row = create_inline_entry_row(depth, "New subtask...");
                    let next_row = list_box.row_at_index(index as i32 + 1);
                    if let Some(next) = next_row {
                        entry_row.insert_before(list_box, Some(&next));
                    } else {
                        list_box.append(&entry_row);
                    }
                    *inline_entry_row.borrow_mut() = Some(entry_row.clone());

                    if let Some(entry) = get_entry_from_row(&entry_row) {
                        let todos_c = refresh_todos.clone();
                        let list_box_c = refresh_list_box.clone();
                        let flat_todos_c = refresh_flat_todos.clone();
                        let display_settings_c = refresh_display_settings.clone();
                        let input_mode_c = input_mode.clone();
                        let mode_label_c = mode_label.clone();
                        let inline_entry_row_c = inline_entry_row.clone();
                        let path_c = path.clone();

                        entry.connect_activate(move |e| {
                            let text = e.text().to_string();
                            if !text.trim().is_empty() {
                                let (text_after_priority, priority) = parse_priority(&text);
                                let (task_text, due_date) = parse_date(&text_after_priority);
                                if !task_text.trim().is_empty() {
                                    let todo = Todo::new(task_text, due_date, priority);
                                    todos_c.borrow_mut().add_subtask(&path_c, todo);
                                }
                            }
                            if let Some(row) = inline_entry_row_c.borrow_mut().take() {
                                list_box_c.remove(&row);
                            }
                            refresh_list_with_settings(&todos_c, &list_box_c, &flat_todos_c, &display_settings_c);
                            *input_mode_c.borrow_mut() = InputMode::Normal;
                            mode_label_c.set_text("NORMAL");
                            list_box_c.grab_focus();
                        });
                        entry.grab_focus();
                    }
                }
            }
        }
        Action::Edit => {
            if let Some(row) = list_box.selected_row() {
                let index = row.index() as usize;
                let flat = flat_todos.borrow();
                if let Some(flat_todo) = flat.get(index) {
                    let path = flat_todo.path.clone();
                    let current_text = flat_todo.todo.text.clone();
                    drop(flat);
                    *input_mode.borrow_mut() = InputMode::Edit(path);
                    mode_label.set_text("EDIT");
                    command_entry.set_placeholder_text(Some(""));
                    command_entry.set_text(&current_text);
                    command_entry.set_sensitive(true);
                    command_entry.grab_focus();
                    command_entry.set_position(-1);
                }
            }
        }
        Action::CommandMode => {
            *input_mode.borrow_mut() = InputMode::Command;
            mode_label.set_text("COMMAND");
            command_entry.set_placeholder_text(Some(""));
            command_entry.set_text(":");
            command_entry.set_sensitive(true);
            command_entry.grab_focus();
            command_entry.set_position(-1);
        }
        Action::Cancel => {
            // Handled in the main key handler
        }
    }
    gdk::glib::Propagation::Stop
}

impl ZapWindow {
    fn setup_entry_handler(&self) {
        // This handler is only for the command bar (Command and Edit modes)
        // Insert modes now use inline entries
        let todos = self.todos.clone();
        let list_box = self.list_box.clone();
        let mode_label = self.mode_label.clone();
        let cluster_label = self.cluster_label.clone();
        let notification_label = self.notification_label.clone();
        let input_mode = self.input_mode.clone();
        let flat_todos = self.flat_todos.clone();
        let display_settings = self.display_settings.clone();
        let window = self.window.clone();

        self.command_entry.connect_activate(move |e| {
            let text = e.text().to_string();
            let mode = input_mode.borrow().clone();

            // Hide any previous notification
            notification_label.set_visible(false);

            match mode {
                InputMode::Command => {
                    // Handle command
                    let cmd = text.trim();
                    if cmd == ":display_start" {
                        let mut settings = display_settings.borrow_mut();
                        settings.show_start_date = !settings.show_start_date;
                        drop(settings);
                        refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                    } else if cmd == ":ls" {
                        // List available clusters
                        let clusters = TodoList::list_clusters();
                        if clusters.is_empty() {
                            notification_label.set_text("No clusters found");
                        } else {
                            notification_label.set_text(&format!("Clusters: {}", clusters.join(", ")));
                        }
                        notification_label.remove_css_class("notification-error");
                        notification_label.set_visible(true);
                    } else if let Some(cluster_name) = cmd.strip_prefix(":e ") {
                        // Open/switch to cluster
                        let cluster_name = cluster_name.trim();
                        if !cluster_name.is_empty() {
                            let path = TodoList::cluster_path(cluster_name);
                            if path.exists() {
                                *todos.borrow_mut() = TodoList::load(cluster_name);
                                window.set_title(Some(&format!("Zap - {}", cluster_name)));
                                cluster_label.set_text(cluster_name);
                                refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                            } else {
                                notification_label.set_text(&format!("Cluster '{}' does not exist. Use :n to create.", cluster_name));
                                notification_label.add_css_class("notification-error");
                                notification_label.set_visible(true);
                            }
                        }
                    } else if let Some(cluster_name) = cmd.strip_prefix(":n ") {
                        // Create and open new cluster
                        let cluster_name = cluster_name.trim();
                        if !cluster_name.is_empty() {
                            let new_list = TodoList::load(cluster_name);
                            new_list.save(); // Create the file
                            *todos.borrow_mut() = new_list;
                            window.set_title(Some(&format!("Zap - {}", cluster_name)));
                            cluster_label.set_text(cluster_name);
                            notification_label.set_text(&format!("Created cluster '{}'", cluster_name));
                            notification_label.remove_css_class("notification-error");
                            notification_label.set_visible(true);
                            refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                        }
                    } else if cmd == ":sort" {
                        // Sort tasks by priority, date, then alphabetically
                        todos.borrow_mut().sort();
                        refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                        notification_label.set_text("Tasks sorted");
                        notification_label.remove_css_class("notification-error");
                        notification_label.set_visible(true);
                        let notification_label = notification_label.clone();
                        gtk4::glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
                            notification_label.set_visible(false);
                            gtk4::glib::ControlFlow::Break
                        });
                    }
                    // Unknown commands are silently ignored
                }
                InputMode::Edit(ref path) => {
                    if !text.trim().is_empty() {
                        let (text_after_priority, priority) = parse_priority(&text);
                        let (task_text, due_date) = parse_date(&text_after_priority);
                        if !task_text.trim().is_empty() {
                            todos.borrow_mut().update_at_path(path, task_text, due_date, priority);
                            refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
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

    fn apply_css(&self) {
        let css = self.color_config.generate_css();

        let provider = gtk4::CssProvider::new();
        provider.load_from_data(&css);

        gtk4::style_context_add_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&self.window),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn create_todo_row(flat_todo: &FlatTodo, settings: &DisplaySettings) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.add_css_class("todo-row");

    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_start(8 + (flat_todo.depth as i32 * 20));
    hbox.set_margin_end(8);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    // Section rendering - different from regular tasks
    if flat_todo.todo.is_section {
        row.add_css_class("section-row");

        // Fold chevron for sections with subtasks
        if flat_todo.has_subtasks {
            let chevron = if flat_todo.is_folded { "▶" } else { "▼" };
            let chevron_label = Label::new(Some(chevron));
            chevron_label.add_css_class("fold-chevron");
            hbox.append(&chevron_label);
        }

        // Section marker
        let marker = Label::new(Some("§"));
        marker.add_css_class("section-marker");
        hbox.append(&marker);

        // Section text
        let text_label = Label::new(Some(&flat_todo.todo.text));
        text_label.set_hexpand(true);
        text_label.set_halign(gtk4::Align::Start);
        text_label.add_css_class("section-text");
        hbox.append(&text_label);

        row.set_child(Some(&hbox));
        return row;
    }

    // Regular task rendering
    // Fold chevron or subtask indicator
    if flat_todo.has_subtasks {
        let chevron = if flat_todo.is_folded { "▶" } else { "▼" };
        let chevron_label = Label::new(Some(chevron));
        chevron_label.add_css_class("fold-chevron");
        hbox.append(&chevron_label);
    } else if flat_todo.depth > 0 {
        let indent = Label::new(Some("└"));
        indent.add_css_class("subtask-indicator");
        hbox.append(&indent);
    } else {
        // Empty space for alignment when no chevron or subtask indicator
        let spacer = Label::new(Some(" "));
        spacer.add_css_class("fold-spacer");
        hbox.append(&spacer);
    }

    // Apply max priority row background class
    if flat_todo.todo.priority == Priority::Max {
        row.add_css_class("priority-max-row");
    }

    // Priority indicator (always show for consistent alignment)
    let priority_label = Label::new(Some("●"));
    match flat_todo.todo.priority {
        Priority::Max => priority_label.add_css_class("priority-max"),
        Priority::High => priority_label.add_css_class("priority-high"),
        Priority::Medium => priority_label.add_css_class("priority-medium"),
        Priority::Low => priority_label.add_css_class("priority-low"),
        Priority::None => priority_label.add_css_class("priority-none"),
    }
    hbox.append(&priority_label);

    // Checkbox indicator
    let check = if flat_todo.todo.completed { "[x]" } else { "[ ]" };
    let check_label = Label::new(Some(check));
    check_label.add_css_class("todo-check");

    // Todo text
    let text_label = Label::new(Some(&flat_todo.todo.text));
    text_label.set_hexpand(true);
    text_label.set_halign(gtk4::Align::Start);
    if flat_todo.todo.completed {
        text_label.add_css_class("completed");
    }

    hbox.append(&check_label);
    hbox.append(&text_label);

    // Start date (if enabled)
    if settings.show_start_date {
        let created: DateTime<Utc> = DateTime::from_timestamp(flat_todo.todo.created_at, 0)
            .unwrap_or_else(|| Utc::now());
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

    // Due date
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

    row.set_child(Some(&hbox));
    row
}

fn move_selection(list_box: &ListBox, delta: i32) {
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

fn refresh_list_with_settings(
    todos: &Rc<RefCell<TodoList>>,
    list_box: &ListBox,
    flat_todos: &Rc<RefCell<Vec<FlatTodo>>>,
    display_settings: &Rc<RefCell<DisplaySettings>>,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let todos_ref = todos.borrow();
    let flat = todos_ref.flatten();
    let settings = display_settings.borrow();

    for flat_todo in &flat {
        let row = create_todo_row(flat_todo, &settings);
        list_box.append(&row);
    }

    *flat_todos.borrow_mut() = flat;
}

/// Autocomplete command input
fn autocomplete_command(input: &str) -> Option<String> {
    let commands = [":e ", ":n ", ":ls", ":sort", ":display_start"];

    // Check for command completion
    for cmd in &commands {
        if cmd.starts_with(input) && *cmd != input {
            return Some(cmd.to_string());
        }
    }

    // Check for cluster name completion after :e or :n
    if let Some(partial) = input.strip_prefix(":e ") {
        let clusters = TodoList::list_clusters();
        for cluster in clusters {
            if cluster.starts_with(partial) && cluster != partial {
                return Some(format!(":e {}", cluster));
            }
        }
    } else if let Some(partial) = input.strip_prefix(":n ") {
        let clusters = TodoList::list_clusters();
        for cluster in clusters {
            if cluster.starts_with(partial) && cluster != partial {
                return Some(format!(":n {}", cluster));
            }
        }
    }

    None
}

/// Create an inline entry row for insert modes
fn create_inline_entry_row(depth: usize, placeholder: &str) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.add_css_class("inline-entry-row");
    row.set_selectable(false);

    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_start(8 + (depth as i32 * 20));
    hbox.set_margin_end(8);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);

    // Indicator
    let indicator = Label::new(Some(">"));
    indicator.add_css_class("insert-indicator");
    hbox.append(&indicator);

    // Entry
    let entry = Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.add_css_class("inline-entry");
    entry.set_hexpand(true);
    hbox.append(&entry);

    row.set_child(Some(&hbox));
    row
}

/// Get the Entry widget from an inline entry row
fn get_entry_from_row(row: &ListBoxRow) -> Option<Entry> {
    let hbox = row.child()?.downcast::<GtkBox>().ok()?;
    // Entry is the second child (after the indicator)
    let mut child = hbox.first_child();
    child = child?.next_sibling(); // Skip indicator
    child?.downcast::<Entry>().ok()
}
