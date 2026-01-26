use chrono::{DateTime, Datelike, Local, NaiveDate, Utc};
use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Box as GtkBox, Entry, EventControllerKey, Frame, Grid,
    Label, ListBox, ListBoxRow, Notebook, Orientation, ScrolledWindow, SelectionMode, Stack,
    StackTransitionType,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    CalendarInsert(NaiveDate),   // Inserting a task on a specific calendar date
}

#[derive(Clone, Debug, Default)]
struct DisplaySettings {
    show_start_date: bool,
}

/// View type for a tab
#[derive(Clone, Debug, PartialEq)]
enum ViewType {
    List,
    Calendar,
}

/// Calendar state
struct CalendarState {
    year: i32,
    month: u32,
    selected_day: u32,
    grid: Grid,
    day_frames: HashMap<u32, Frame>,
    month_label: Label,
}

/// Per-tab content state
struct TabContent {
    todos: Rc<RefCell<TodoList>>,
    list_box: ListBox,
    flat_todos: Rc<RefCell<Vec<FlatTodo>>>,
    inline_entry_row: Rc<RefCell<Option<ListBoxRow>>>,
    cluster_name: String,
    view_type: Rc<RefCell<ViewType>>,
    calendar_state: Rc<RefCell<Option<CalendarState>>>,
    content_stack: gtk4::Stack,
    #[allow(dead_code)]
    scrolled_list: ScrolledWindow,
    scrolled_calendar: ScrolledWindow,
}

pub struct ZapWindow {
    pub window: ApplicationWindow,
    notebook: Notebook,
    tabs: Rc<RefCell<Vec<TabContent>>>,
    command_entry: Entry,  // Bottom entry for command mode only
    mode_label: Label,
    notification_label: Label,
    input_mode: Rc<RefCell<InputMode>>,
    pending_key: Rc<RefCell<Option<String>>>,  // For key sequences like gg, dd, za
    display_settings: Rc<RefCell<DisplaySettings>>,
    keybindings: Rc<Keybindings>,
    color_config: Rc<ColorConfig>,
}

impl ZapWindow {
    pub fn new(app: &Application) -> Self {
        let input_mode = Rc::new(RefCell::new(InputMode::Normal));
        let pending_key: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let display_settings = Rc::new(RefCell::new(DisplaySettings::default()));
        let keybindings = Rc::new(Keybindings::load());
        let color_config = Rc::new(ColorConfig::load());
        let tabs: Rc<RefCell<Vec<TabContent>>> = Rc::new(RefCell::new(Vec::new()));

        // Create window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Zap")
            .default_width(500)
            .default_height(600)
            .build();

        // Main container
        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.add_css_class("main-container");

        // Header with mode indicator
        let header_box = GtkBox::new(Orientation::Horizontal, 8);
        header_box.set_margin_start(12);
        header_box.set_margin_end(12);
        header_box.set_margin_top(8);
        header_box.set_margin_bottom(4);

        // Mode indicator
        let mode_label = Label::new(Some("NORMAL"));
        mode_label.add_css_class("mode-indicator");
        mode_label.set_halign(gtk4::Align::End);
        mode_label.set_hexpand(true);

        header_box.append(&mode_label);

        // Notification label (hidden by default)
        let notification_label = Label::new(None);
        notification_label.add_css_class("notification");
        notification_label.set_margin_start(12);
        notification_label.set_visible(false);

        // Notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);
        notebook.set_scrollable(true);
        notebook.add_css_class("zap-notebook");

        // Help label
        let help_label = Label::new(Some("j/k: nav | J/K: reorder | Enter: toggle | dd: del | i: insert | e: edit | za: fold | :: cmd | Ctrl+T/W: tabs"));
        help_label.add_css_class("help-text");
        help_label.set_margin_bottom(4);

        // Command bar at bottom (for command mode only) - shared across tabs
        let command_entry = Entry::new();
        command_entry.add_css_class("command-bar");
        command_entry.set_margin_start(12);
        command_entry.set_margin_end(12);
        command_entry.set_margin_bottom(8);
        command_entry.set_can_focus(true);
        command_entry.set_sensitive(false);

        // Layout: header, notification, notebook (tabs), help, command bar (bottom)
        main_box.append(&header_box);
        main_box.append(&notification_label);
        main_box.append(&notebook);
        main_box.append(&help_label);
        main_box.append(&command_entry);

        window.set_child(Some(&main_box));

        let zap = Self {
            window,
            notebook,
            tabs,
            command_entry,
            mode_label,
            notification_label,
            input_mode,
            pending_key,
            display_settings,
            keybindings,
            color_config,
        };

        // Create initial tab with "main" cluster
        zap.add_tab("main");
        zap.setup_keybindings();
        zap.setup_entry_handler();
        zap.setup_entry_autocomplete();
        zap.apply_css();

        zap
    }

    /// Create a new tab with the given cluster name
    fn add_tab(&self, cluster_name: &str) {
        let todos = Rc::new(RefCell::new(TodoList::load(cluster_name)));
        let flat_todos = Rc::new(RefCell::new(Vec::new()));
        let inline_entry_row: Rc<RefCell<Option<ListBoxRow>>> = Rc::new(RefCell::new(None));
        let view_type = Rc::new(RefCell::new(ViewType::List));
        let calendar_state: Rc<RefCell<Option<CalendarState>>> = Rc::new(RefCell::new(None));

        // Create stack for switching between list and calendar views
        let content_stack = Stack::new();
        content_stack.set_transition_type(StackTransitionType::Crossfade);
        content_stack.set_transition_duration(150);

        // Create list view
        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::Single);
        list_box.add_css_class("todo-list");

        let scrolled_list = ScrolledWindow::new();
        scrolled_list.set_vexpand(true);
        scrolled_list.set_child(Some(&list_box));
        scrolled_list.set_margin_start(12);
        scrolled_list.set_margin_end(12);
        scrolled_list.set_margin_bottom(8);

        content_stack.add_named(&scrolled_list, Some("list"));

        // Create calendar view container (will be populated when switched to)
        let scrolled_calendar = ScrolledWindow::new();
        scrolled_calendar.set_vexpand(true);
        scrolled_calendar.set_margin_start(12);
        scrolled_calendar.set_margin_end(12);
        scrolled_calendar.set_margin_bottom(8);

        content_stack.add_named(&scrolled_calendar, Some("calendar"));
        content_stack.set_visible_child_name("list");

        // Tab label
        let tab_label_box = GtkBox::new(Orientation::Horizontal, 4);
        let tab_label = Label::new(Some(cluster_name));
        tab_label_box.append(&tab_label);

        // Add the page to notebook
        let page_num = self.notebook.append_page(&content_stack, Some(&tab_label_box));

        // Store tab content
        let tab_content = TabContent {
            todos: todos.clone(),
            list_box: list_box.clone(),
            flat_todos: flat_todos.clone(),
            inline_entry_row,
            cluster_name: cluster_name.to_string(),
            view_type,
            calendar_state,
            content_stack,
            scrolled_list,
            scrolled_calendar,
        };
        self.tabs.borrow_mut().push(tab_content);

        // Refresh the list for this tab
        self.refresh_tab(self.tabs.borrow().len() - 1);

        // Switch to the new tab
        self.notebook.set_current_page(Some(page_num));

        // Focus the list
        list_box.grab_focus();
    }

    /// Refresh a specific tab's list
    fn refresh_tab(&self, tab_index: usize) {
        let tabs = self.tabs.borrow();
        if let Some(tab) = tabs.get(tab_index) {
            while let Some(child) = tab.list_box.first_child() {
                tab.list_box.remove(&child);
            }

            let todos = tab.todos.borrow();
            let flat = todos.flatten();
            let settings = self.display_settings.borrow();

            for flat_todo in &flat {
                let row = create_todo_row(flat_todo, &settings);
                tab.list_box.append(&row);
            }

            *tab.flat_todos.borrow_mut() = flat;

            if let Some(first_row) = tab.list_box.row_at_index(0) {
                tab.list_box.select_row(Some(&first_row));
            }
        }
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

    fn setup_keybindings(&self) {
        let key_controller = EventControllerKey::new();

        let tabs = self.tabs.clone();
        let notebook = self.notebook.clone();
        let command_entry = self.command_entry.clone();
        let mode_label = self.mode_label.clone();
        let input_mode = self.input_mode.clone();
        let pending_key = self.pending_key.clone();
        let display_settings = self.display_settings.clone();
        let keybindings = self.keybindings.clone();

        // Clone self references for tab operations
        let tabs_for_new = tabs.clone();
        let notebook_for_new = notebook.clone();
        let display_settings_for_new = display_settings.clone();

        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            let mode = input_mode.borrow().clone();
            let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            let alt = modifier.contains(gdk::ModifierType::ALT_MASK);

            // Handle Ctrl+T (new tab) and Ctrl+W (close tab) - these work in all modes
            if ctrl && !shift && !alt {
                if key == gdk::Key::t {
                    // Open new tab
                    open_new_tab(&tabs_for_new, &notebook_for_new, &display_settings_for_new);
                    return gdk::glib::Propagation::Stop;
                }
                if key == gdk::Key::w {
                    // Close current tab (if more than one)
                    if let Some(current_page) = notebook.current_page() {
                        if tabs.borrow().len() > 1 {
                            notebook.remove_page(Some(current_page));
                            tabs.borrow_mut().remove(current_page as usize);

                            // Focus the list in the now-current tab
                            if let Some(new_page) = notebook.current_page() {
                                let tabs_ref = tabs.borrow();
                                if let Some(tab) = tabs_ref.get(new_page as usize) {
                                    tab.list_box.grab_focus();
                                }
                            }
                        }
                    }
                    return gdk::glib::Propagation::Stop;
                }
            }

            // Get current tab content
            let current_page = match notebook.current_page() {
                Some(p) => p as usize,
                None => return gdk::glib::Propagation::Proceed,
            };
            let tabs_ref = tabs.borrow();
            let tab = match tabs_ref.get(current_page) {
                Some(t) => t,
                None => return gdk::glib::Propagation::Proceed,
            };

            let todos = tab.todos.clone();
            let list_box = tab.list_box.clone();
            let flat_todos = tab.flat_todos.clone();
            let inline_entry_row = tab.inline_entry_row.clone();
            let view_type = tab.view_type.clone();
            let calendar_state = tab.calendar_state.clone();
            drop(tabs_ref);

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
                    if *view_type.borrow() == ViewType::List {
                        list_box.grab_focus();
                    }
                    return gdk::glib::Propagation::Stop;
                }
                return gdk::glib::Propagation::Proceed;
            }

            // Check if we're in calendar view
            if *view_type.borrow() == ViewType::Calendar {
                // Calendar-specific keybindings
                // Get key name for arrow key detection
                let key_name = key.name().map(|s| s.to_string()).unwrap_or_default();
                let is_left = key_name == "Left" || key == gdk::Key::Left;
                let is_right = key_name == "Right" || key == gdk::Key::Right;
                let is_up = key_name == "Up" || key == gdk::Key::Up;
                let is_down = key_name == "Down" || key == gdk::Key::Down;

                // Ctrl+Left/Right for month navigation
                if ctrl && !shift && !alt {
                    if is_left {
                        change_calendar_month(&calendar_state, -1);
                        return gdk::glib::Propagation::Stop;
                    }
                    if is_right {
                        change_calendar_month(&calendar_state, 1);
                        return gdk::glib::Propagation::Stop;
                    }
                }

                match key {
                    k if k == gdk::Key::h || (is_left && !ctrl) => {
                        navigate_calendar(&calendar_state, -1, 0);
                        return gdk::glib::Propagation::Stop;
                    }
                    k if k == gdk::Key::l || (is_right && !ctrl) => {
                        navigate_calendar(&calendar_state, 1, 0);
                        return gdk::glib::Propagation::Stop;
                    }
                    k if k == gdk::Key::k || is_up => {
                        navigate_calendar(&calendar_state, 0, -1);
                        return gdk::glib::Propagation::Stop;
                    }
                    k if k == gdk::Key::j || is_down => {
                        navigate_calendar(&calendar_state, 0, 1);
                        return gdk::glib::Propagation::Stop;
                    }
                    k if k == gdk::Key::i => {
                        // Insert task on selected date
                        if let Some(date) = get_selected_calendar_date(&calendar_state) {
                            *input_mode.borrow_mut() = InputMode::CalendarInsert(date);
                            mode_label.set_text("INSERT (calendar)");
                            command_entry.set_placeholder_text(Some(&format!("Task for {}...", date.format("%b %d"))));
                            command_entry.set_text("");
                            command_entry.set_sensitive(true);
                            command_entry.grab_focus();
                        }
                        return gdk::glib::Propagation::Stop;
                    }
                    k if k == gdk::Key::colon && shift => {
                        // Command mode
                        *input_mode.borrow_mut() = InputMode::Command;
                        mode_label.set_text("COMMAND");
                        command_entry.set_placeholder_text(Some(""));
                        command_entry.set_text(":");
                        command_entry.set_sensitive(true);
                        command_entry.grab_focus();
                        command_entry.set_position(-1);
                        return gdk::glib::Propagation::Stop;
                    }
                    _ => {}
                }
                return gdk::glib::Propagation::Proceed;
            }

            // List view keybindings
            // Check for sequence completion first
            let pending = pending_key.borrow().clone();
            if let Some(ref pending_str) = pending {
                if let Some(action) = keybindings.get_sequence_action(pending_str, &key) {
                    *pending_key.borrow_mut() = None;
                    return execute_action(
                        action, &todos, &list_box, &command_entry, &mode_label,
                        &input_mode, &flat_todos, &todos, &list_box,
                        &flat_todos, &display_settings, &inline_entry_row,
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
                    &input_mode, &flat_todos, &todos, &list_box,
                    &flat_todos, &display_settings, &inline_entry_row,
                );
            }

            *pending_key.borrow_mut() = None;
            gdk::glib::Propagation::Proceed
        });

        self.window.add_controller(key_controller);
    }
}

/// Open a new blank tab
fn open_new_tab(
    tabs: &Rc<RefCell<Vec<TabContent>>>,
    notebook: &Notebook,
    _display_settings: &Rc<RefCell<DisplaySettings>>,
) {
    // Create an empty tab with no cluster loaded
    let todos = Rc::new(RefCell::new(TodoList::default()));
    let flat_todos = Rc::new(RefCell::new(Vec::new()));
    let inline_entry_row: Rc<RefCell<Option<ListBoxRow>>> = Rc::new(RefCell::new(None));
    let view_type = Rc::new(RefCell::new(ViewType::List));
    let calendar_state: Rc<RefCell<Option<CalendarState>>> = Rc::new(RefCell::new(None));

    // Create stack for switching between list and calendar views
    let content_stack = Stack::new();
    content_stack.set_transition_type(StackTransitionType::Crossfade);
    content_stack.set_transition_duration(150);

    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::Single);
    list_box.add_css_class("todo-list");

    let scrolled_list = ScrolledWindow::new();
    scrolled_list.set_vexpand(true);
    scrolled_list.set_child(Some(&list_box));
    scrolled_list.set_margin_start(12);
    scrolled_list.set_margin_end(12);
    scrolled_list.set_margin_bottom(8);

    content_stack.add_named(&scrolled_list, Some("list"));

    let scrolled_calendar = ScrolledWindow::new();
    scrolled_calendar.set_vexpand(true);
    scrolled_calendar.set_margin_start(12);
    scrolled_calendar.set_margin_end(12);
    scrolled_calendar.set_margin_bottom(8);

    content_stack.add_named(&scrolled_calendar, Some("calendar"));
    content_stack.set_visible_child_name("list");

    // Tab label - empty/new tab
    let tab_label = Label::new(Some("[new]"));

    // Add the page to notebook
    let page_num = notebook.append_page(&content_stack, Some(&tab_label));

    // Store tab content
    let tab_content = TabContent {
        todos,
        list_box: list_box.clone(),
        flat_todos,
        inline_entry_row,
        cluster_name: String::new(),
        view_type,
        calendar_state,
        content_stack,
        scrolled_list,
        scrolled_calendar,
    };
    tabs.borrow_mut().push(tab_content);

    // Switch to the new tab
    notebook.set_current_page(Some(page_num));
    list_box.grab_focus();
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
                    // Insert at the position right after the selected row
                    list_box.insert(&entry_row, index as i32 + 1);
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

                        // Delay focus to allow UI to update and scroll into view
                        let entry_for_focus = entry.clone();
                        gdk::glib::idle_add_local_once(move || {
                            entry_for_focus.grab_focus();
                        });
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
        let tabs = self.tabs.clone();
        let notebook = self.notebook.clone();
        let mode_label = self.mode_label.clone();
        let notification_label = self.notification_label.clone();
        let input_mode = self.input_mode.clone();
        let display_settings = self.display_settings.clone();

        self.command_entry.connect_activate(move |e| {
            let text = e.text().to_string();
            let mode = input_mode.borrow().clone();

            // Hide any previous notification
            notification_label.set_visible(false);

            // Get current tab
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

            let todos = tab.todos.clone();
            let list_box = tab.list_box.clone();
            let flat_todos = tab.flat_todos.clone();
            drop(tabs_ref);

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
                    } else if cmd == ":e calendar" || cmd == ":e cal" {
                        // Switch to calendar view
                        let mut tabs_mut = tabs.borrow_mut();
                        let tab = &mut tabs_mut[current_page];
                        *tab.view_type.borrow_mut() = ViewType::Calendar;

                        // Create calendar if not exists
                        if tab.calendar_state.borrow().is_none() {
                            create_calendar_view(&tab.scrolled_calendar, &tab.calendar_state);
                        } else {
                            refresh_calendar_view(&tab.calendar_state);
                        }

                        tab.content_stack.set_visible_child_name("calendar");
                        // Update tab label
                        if let Some(page_widget) = notebook.nth_page(Some(current_page as u32)) {
                            let label = if tab.cluster_name.is_empty() {
                                "[calendar]".to_string()
                            } else {
                                format!("{} [cal]", tab.cluster_name)
                            };
                            notebook.set_tab_label_text(&page_widget, &label);
                        }
                    } else if cmd == ":e list" {
                        // Switch back to list view
                        let mut tabs_mut = tabs.borrow_mut();
                        let tab = &mut tabs_mut[current_page];
                        *tab.view_type.borrow_mut() = ViewType::List;
                        tab.content_stack.set_visible_child_name("list");
                        // Update tab label
                        if let Some(page_widget) = notebook.nth_page(Some(current_page as u32)) {
                            let label = if tab.cluster_name.is_empty() {
                                "[new]".to_string()
                            } else {
                                tab.cluster_name.clone()
                            };
                            notebook.set_tab_label_text(&page_widget, &label);
                        }
                        tab.list_box.grab_focus();
                    } else if let Some(cluster_name) = cmd.strip_prefix(":e ") {
                        // Open cluster in current tab
                        let cluster_name = cluster_name.trim();
                        // Handle calendar/list as special cases (fallback)
                        if cluster_name == "calendar" || cluster_name == "cal" {
                            let mut tabs_mut = tabs.borrow_mut();
                            let tab = &mut tabs_mut[current_page];
                            *tab.view_type.borrow_mut() = ViewType::Calendar;
                            if tab.calendar_state.borrow().is_none() {
                                create_calendar_view(&tab.scrolled_calendar, &tab.calendar_state);
                            } else {
                                refresh_calendar_view(&tab.calendar_state);
                            }
                            tab.content_stack.set_visible_child_name("calendar");
                            if let Some(page_widget) = notebook.nth_page(Some(current_page as u32)) {
                                let label = if tab.cluster_name.is_empty() {
                                    "[calendar]".to_string()
                                } else {
                                    format!("{} [cal]", tab.cluster_name)
                                };
                                notebook.set_tab_label_text(&page_widget, &label);
                            }
                        } else if cluster_name == "list" {
                            let mut tabs_mut = tabs.borrow_mut();
                            let tab = &mut tabs_mut[current_page];
                            *tab.view_type.borrow_mut() = ViewType::List;
                            tab.content_stack.set_visible_child_name("list");
                            if let Some(page_widget) = notebook.nth_page(Some(current_page as u32)) {
                                let label = if tab.cluster_name.is_empty() {
                                    "[new]".to_string()
                                } else {
                                    tab.cluster_name.clone()
                                };
                                notebook.set_tab_label_text(&page_widget, &label);
                            }
                            tab.list_box.grab_focus();
                        } else if !cluster_name.is_empty() {
                            let path = TodoList::cluster_path(cluster_name);
                            if path.exists() {
                                *todos.borrow_mut() = TodoList::load(cluster_name);
                                // Update the tab label
                                if let Some(page_widget) = notebook.nth_page(Some(current_page as u32)) {
                                    notebook.set_tab_label_text(&page_widget, cluster_name);
                                }
                                // Update stored cluster name
                                tabs.borrow_mut()[current_page].cluster_name = cluster_name.to_string();
                                // Switch to list view
                                tabs.borrow_mut()[current_page].content_stack.set_visible_child_name("list");
                                *tabs.borrow_mut()[current_page].view_type.borrow_mut() = ViewType::List;
                                refresh_list_with_settings(&todos, &list_box, &flat_todos, &display_settings);
                            } else {
                                notification_label.set_text(&format!("Cluster '{}' does not exist. Use :n to create.", cluster_name));
                                notification_label.add_css_class("notification-error");
                                notification_label.set_visible(true);
                            }
                        }
                    } else if let Some(cluster_name) = cmd.strip_prefix(":n ") {
                        // Create and open new cluster in current tab
                        let cluster_name = cluster_name.trim();
                        if !cluster_name.is_empty() {
                            let new_list = TodoList::load(cluster_name);
                            new_list.save(); // Create the file
                            *todos.borrow_mut() = new_list;
                            // Update the tab label
                            if let Some(page_widget) = notebook.nth_page(Some(current_page as u32)) {
                                notebook.set_tab_label_text(&page_widget, cluster_name);
                            }
                            // Update stored cluster name
                            tabs.borrow_mut()[current_page].cluster_name = cluster_name.to_string();
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
                InputMode::CalendarInsert(date) => {
                    if !text.trim().is_empty() {
                        let (text_after_priority, priority) = parse_priority(&text);
                        // Ignore any date in the text, use the calendar date
                        let (task_text, _) = parse_date(&text_after_priority);
                        if !task_text.trim().is_empty() {
                            let todo = Todo::new(task_text, Some(date), priority);
                            todos.borrow_mut().add(todo);
                            // Refresh calendar view
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
    let commands = [":e ", ":e calendar", ":e list", ":n ", ":ls", ":sort", ":display_start"];

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

/// Create and populate the calendar view for a tab
fn create_calendar_view(
    scrolled_calendar: &ScrolledWindow,
    calendar_state: &Rc<RefCell<Option<CalendarState>>>,
) {
    let today = Local::now().date_naive();
    let year = today.year();
    let month = today.month();
    let selected_day = today.day();

    // Main container
    let main_box = GtkBox::new(Orientation::Vertical, 8);
    main_box.set_margin_start(8);
    main_box.set_margin_end(8);
    main_box.set_margin_top(8);
    main_box.set_margin_bottom(8);

    // Month/Year header
    let month_label = Label::new(None);
    month_label.add_css_class("calendar-header");
    main_box.append(&month_label);

    // Day names header
    let day_names_box = GtkBox::new(Orientation::Horizontal, 0);
    day_names_box.set_homogeneous(true);
    for day_name in &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
        let label = Label::new(Some(day_name));
        label.add_css_class("calendar-day-header");
        day_names_box.append(&label);
    }
    main_box.append(&day_names_box);

    // Calendar grid
    let grid = Grid::new();
    grid.set_row_homogeneous(true);
    grid.set_column_homogeneous(true);
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    grid.add_css_class("calendar-grid");
    main_box.append(&grid);

    scrolled_calendar.set_child(Some(&main_box));

    // Store calendar state
    let state = CalendarState {
        year,
        month,
        selected_day,
        grid,
        day_frames: HashMap::new(),
        month_label,
    };
    *calendar_state.borrow_mut() = Some(state);

    // Populate the calendar with tasks from all clusters
    refresh_calendar_view(calendar_state);
}

/// Refresh the calendar view with tasks from ALL clusters
fn refresh_calendar_view(
    calendar_state: &Rc<RefCell<Option<CalendarState>>>,
) {
    let mut state_ref = calendar_state.borrow_mut();
    let state = match state_ref.as_mut() {
        Some(s) => s,
        None => return,
    };

    let year = state.year;
    let month = state.month;
    let selected_day = state.selected_day;

    // Update month label
    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    state.month_label.set_text(&format!("{} {}", month_names[(month - 1) as usize], year));

    // Clear existing grid content - collect children first to avoid removal issues
    let mut children = Vec::new();
    let mut child = state.grid.first_child();
    while let Some(c) = child {
        let next = c.next_sibling();
        children.push(c);
        child = next;
    }
    for c in children {
        state.grid.remove(&c);
    }
    state.day_frames.clear();

    // Get first day of month and number of days
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let days_in_month = days_in_month(year, month);
    let first_weekday = first_day.weekday().num_days_from_sunday();

    // Load tasks from main cluster only
    let today = Local::now().date_naive();
    let todo_list = TodoList::load("main");
    let flat_todos = todo_list.flatten();

    // Group tasks by day
    let mut tasks_by_day: HashMap<u32, Vec<FlatTodo>> = HashMap::new();
    for flat_todo in flat_todos {
        let date = flat_todo.todo.due_date.unwrap_or(today);
        if date.year() == year && date.month() == month {
            tasks_by_day.entry(date.day()).or_default().push(flat_todo);
        }
    }

    // Create day cells
    for day in 1..=days_in_month {
        let col = ((first_weekday + day - 1) % 7) as i32;
        let row = ((first_weekday + day - 1) / 7) as i32;

        let frame = Frame::new(None);
        frame.add_css_class("calendar-day");

        let day_box = GtkBox::new(Orientation::Vertical, 2);
        day_box.set_margin_start(4);
        day_box.set_margin_end(4);
        day_box.set_margin_top(4);
        day_box.set_margin_bottom(4);

        // Day number
        let day_label = Label::new(Some(&day.to_string()));
        day_label.set_halign(gtk4::Align::Start);
        day_label.add_css_class("calendar-day-number");

        // Highlight today
        let this_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        if this_date == today {
            frame.add_css_class("calendar-today");
        }

        // Highlight selected day
        if day == selected_day {
            frame.add_css_class("calendar-selected");
        }

        day_box.append(&day_label);

        // Add tasks for this day
        if let Some(day_tasks) = tasks_by_day.get(&day) {
            for (i, flat_todo) in day_tasks.iter().enumerate() {
                if i >= 3 {
                    // Show "+N more" if too many tasks
                    let more_label = Label::new(Some(&format!("+{} more", day_tasks.len() - 3)));
                    more_label.add_css_class("calendar-task-more");
                    more_label.set_halign(gtk4::Align::Start);
                    day_box.append(&more_label);
                    break;
                }
                let task_label = Label::new(Some(&truncate_text(&flat_todo.todo.text, 15)));
                task_label.set_halign(gtk4::Align::Start);
                task_label.add_css_class("calendar-task");
                if flat_todo.todo.completed {
                    task_label.add_css_class("calendar-task-completed");
                }
                // Priority coloring
                match flat_todo.todo.priority {
                    Priority::Max => task_label.add_css_class("calendar-task-max"),
                    Priority::High => task_label.add_css_class("calendar-task-high"),
                    Priority::Medium => task_label.add_css_class("calendar-task-medium"),
                    _ => {}
                }
                day_box.append(&task_label);
            }
        }

        frame.set_child(Some(&day_box));
        state.grid.attach(&frame, col, row, 1, 1);
        state.day_frames.insert(day, frame);
    }
}

/// Update calendar selection highlighting
fn update_calendar_selection(calendar_state: &Rc<RefCell<Option<CalendarState>>>, new_day: u32) {
    let mut state_ref = calendar_state.borrow_mut();
    if let Some(state) = state_ref.as_mut() {
        // Remove selection from old day
        if let Some(old_frame) = state.day_frames.get(&state.selected_day) {
            old_frame.remove_css_class("calendar-selected");
        }
        // Add selection to new day
        if let Some(new_frame) = state.day_frames.get(&new_day) {
            new_frame.add_css_class("calendar-selected");
            state.selected_day = new_day;
        }
    }
}

/// Navigate calendar selection
fn navigate_calendar(calendar_state: &Rc<RefCell<Option<CalendarState>>>, delta_days: i32, delta_weeks: i32) {
    let state_ref = calendar_state.borrow();
    if let Some(state) = state_ref.as_ref() {
        let days_in_month = days_in_month(state.year, state.month);
        let current = state.selected_day as i32;
        let new_day = current + delta_days + (delta_weeks * 7);

        if new_day >= 1 && new_day <= days_in_month as i32 {
            drop(state_ref);
            update_calendar_selection(calendar_state, new_day as u32);
        }
    }
}

/// Change calendar month (delta: -1 for previous, +1 for next)
fn change_calendar_month(calendar_state: &Rc<RefCell<Option<CalendarState>>>, delta: i32) {
    {
        let mut state_ref = calendar_state.borrow_mut();
        if let Some(state) = state_ref.as_mut() {
            let mut new_month = state.month as i32 + delta;
            let mut new_year = state.year;

            if new_month < 1 {
                new_month = 12;
                new_year -= 1;
            } else if new_month > 12 {
                new_month = 1;
                new_year += 1;
            }

            state.year = new_year;
            state.month = new_month as u32;
            // Keep selected day, but clamp to valid range for new month
            let max_day = days_in_month(new_year, new_month as u32);
            if state.selected_day > max_day {
                state.selected_day = max_day;
            }
        }
    }
    // Refresh the calendar with new month
    refresh_calendar_view(calendar_state);
}

/// Get number of days in a month
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Truncate text to fit in calendar cell
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len - 3])
    }
}

/// Get the currently selected date in the calendar
fn get_selected_calendar_date(calendar_state: &Rc<RefCell<Option<CalendarState>>>) -> Option<NaiveDate> {
    let state_ref = calendar_state.borrow();
    state_ref.as_ref().and_then(|state| {
        NaiveDate::from_ymd_opt(state.year, state.month, state.selected_day)
    })
}
