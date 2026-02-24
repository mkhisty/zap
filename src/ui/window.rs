use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Box as GtkBox, Entry, EventControllerKey, Label,
    Notebook, Orientation,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::colors::ColorConfig;
use crate::keybindings::{Action, Keybindings};
use crate::todo::TodoList;
use super::actions::{ActionContext, execute_action};
use super::calendar_view::{
    change_calendar_month, get_selected_calendar_date, navigate_calendar, refresh_calendar_view,
};
use super::color_picker::show_task_color_picker;
use super::commands::{setup_entry_autocomplete, setup_entry_handler};
use super::list_view::refresh_list_with_settings;
use super::tab::{new_tab_content, TabContent};
use super::types::{DisplaySettings, InputMode, ViewType};

pub struct ZapWindow {
    pub window: ApplicationWindow,
    notebook: Notebook,
    todos: Rc<RefCell<TodoList>>,
    tabs: Rc<RefCell<Vec<TabContent>>>,
    command_entry: Entry,
    mode_label: Label,
    notification_label: Label,
    input_mode: Rc<RefCell<InputMode>>,
    pending_key: Rc<RefCell<Option<String>>>,
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
        let todos = Rc::new(RefCell::new(TodoList::load("main")));
        let tabs: Rc<RefCell<Vec<TabContent>>> = Rc::new(RefCell::new(Vec::new()));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("Zap")
            .default_width(500)
            .default_height(600)
            .build();

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.add_css_class("main-container");

        let header_box = GtkBox::new(Orientation::Horizontal, 8);
        header_box.set_margin_start(12);
        header_box.set_margin_end(12);
        header_box.set_margin_top(8);
        header_box.set_margin_bottom(4);

        let mode_label = Label::new(Some("NORMAL"));
        mode_label.add_css_class("mode-indicator");
        mode_label.set_halign(gtk4::Align::End);
        mode_label.set_hexpand(true);
        header_box.append(&mode_label);

        let notification_label = Label::new(None);
        notification_label.add_css_class("notification");
        notification_label.set_margin_start(12);
        notification_label.set_visible(false);

        let notebook = Notebook::new();
        notebook.set_vexpand(true);
        notebook.set_scrollable(true);

        let help_label = Label::new(Some(
            "j/k: nav | J/K: reorder | Enter: toggle | dd: del | i: insert | e: edit | za: fold | :: cmd | Ctrl+T/W: tabs | Ctrl+Shift+C: color",
        ));
        help_label.add_css_class("help-text");
        help_label.set_margin_bottom(4);

        let command_entry = Entry::new();
        command_entry.add_css_class("command-bar");
        command_entry.set_margin_start(12);
        command_entry.set_margin_end(12);
        command_entry.set_margin_bottom(8);
        command_entry.set_can_focus(true);
        command_entry.set_sensitive(false);

        main_box.append(&header_box);
        main_box.append(&notification_label);
        main_box.append(&notebook);
        main_box.append(&help_label);
        main_box.append(&command_entry);

        window.set_child(Some(&main_box));

        let zap = Self {
            window,
            notebook,
            todos,
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

        new_tab_content(&zap.todos, &zap.tabs, &zap.notebook, &zap.display_settings);
        zap.setup_keybindings();
        setup_entry_handler(
            &zap.command_entry,
            &zap.todos,
            &zap.tabs,
            &zap.notebook,
            &zap.mode_label,
            &zap.notification_label,
            &zap.input_mode,
            &zap.display_settings,
        );
        setup_entry_autocomplete(&zap.command_entry, &zap.input_mode);
        zap.apply_css();
        zap.setup_file_watcher();

        zap.window.connect_close_request(|_| {
            crate::hooks::fire(crate::hooks::HookEvent::AppQuit, None, None);
            gdk::glib::Propagation::Proceed
        });

        crate::hooks::fire(crate::hooks::HookEvent::AppStart, None, None);

        zap
    }

    fn setup_file_watcher(&self) {
        let todos = self.todos.clone();
        let tabs = self.tabs.clone();
        let display_settings = self.display_settings.clone();

        let last_mtime: Rc<RefCell<Option<std::time::SystemTime>>> = Rc::new(RefCell::new(None));

        let watch_path = TodoList::cluster_path("main");
        if let Ok(meta) = std::fs::metadata(&watch_path) {
            if let Ok(mtime) = meta.modified() {
                *last_mtime.borrow_mut() = Some(mtime);
            }
        }

        gtk4::glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            let Ok(meta) = std::fs::metadata(&watch_path) else {
                return gtk4::glib::ControlFlow::Continue;
            };
            let Ok(mtime) = meta.modified() else {
                return gtk4::glib::ControlFlow::Continue;
            };

            let stored = *last_mtime.borrow();
            if Some(mtime) == stored {
                return gtk4::glib::ControlFlow::Continue;
            }

            let file_hash = std::fs::read(&watch_path)
                .map(|b| hash_bytes(&b))
                .ok();
            let mem_hash = serde_json::to_string_pretty(&*todos.borrow())
                .map(|s| hash_bytes(s.as_bytes()))
                .ok();

            *last_mtime.borrow_mut() = Some(mtime);

            if file_hash != mem_hash {
                let new_list = TodoList::load("main");
                *todos.borrow_mut() = new_list;

                let tabs_ref = tabs.borrow();
                for tab in tabs_ref.iter() {
                    if *tab.view_type.borrow() == ViewType::Calendar {
                        refresh_calendar_view(&tab.calendar_state);
                    } else {
                        let selected_idx = tab.list_box.selected_row().map(|r| r.index());
                        refresh_list_with_settings(&todos, &tab.list_box, &tab.flat_todos, &display_settings);
                        let count = tab.flat_todos.borrow().len() as i32;
                        if count > 0 {
                            let new_idx = selected_idx.unwrap_or(0).min(count - 1).max(0);
                            if let Some(row) = tab.list_box.row_at_index(new_idx) {
                                tab.list_box.select_row(Some(&row));
                            }
                        }
                    }
                }
            }

            gtk4::glib::ControlFlow::Continue
        });
    }

    fn setup_keybindings(&self) {
        let key_controller = EventControllerKey::new();

        let todos = self.todos.clone();
        let tabs = self.tabs.clone();
        let notebook = self.notebook.clone();
        let command_entry = self.command_entry.clone();
        let mode_label = self.mode_label.clone();
        let input_mode = self.input_mode.clone();
        let pending_key = self.pending_key.clone();
        let display_settings = self.display_settings.clone();
        let keybindings = self.keybindings.clone();
        let window = self.window.clone();

        key_controller.connect_key_pressed(move |_, key, _, modifier| {
            let mode = input_mode.borrow().clone();
            let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            let alt = modifier.contains(gdk::ModifierType::ALT_MASK);

            // Ctrl+Shift+C — color picker (Normal mode only)
            if ctrl && shift && !alt && key == gdk::Key::C {
                if mode == InputMode::Normal {
                    show_task_color_picker(&window, &tabs, &notebook, &display_settings, &todos);
                }
                return gdk::glib::Propagation::Stop;
            }

            // Ctrl+T (new tab) and Ctrl+W (close tab)
            if ctrl && !shift && !alt {
                if key == gdk::Key::t {
                    new_tab_content(&todos, &tabs, &notebook, &display_settings);
                    return gdk::glib::Propagation::Stop;
                }
                if key == gdk::Key::w {
                    if let Some(current_page) = notebook.current_page() {
                        if tabs.borrow().len() > 1 {
                            notebook.remove_page(Some(current_page));
                            tabs.borrow_mut().remove(current_page as usize);
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

            // Get current tab
            let current_page = match notebook.current_page() {
                Some(p) => p as usize,
                None => return gdk::glib::Propagation::Proceed,
            };
            let tabs_ref = tabs.borrow();
            let tab = match tabs_ref.get(current_page) {
                Some(t) => t,
                None => return gdk::glib::Propagation::Proceed,
            };

            let tab_todos = todos.clone();
            let list_box = tab.list_box.clone();
            let flat_todos = tab.flat_todos.clone();
            let inline_entry_row = tab.inline_entry_row.clone();
            let view_type = tab.view_type.clone();
            let calendar_state = tab.calendar_state.clone();
            drop(tabs_ref);

            // Non-normal modes: only Escape works
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

            // Calendar view keybindings
            if *view_type.borrow() == ViewType::Calendar {
                let is_left = key == gdk::Key::Left;
                let is_right = key == gdk::Key::Right;
                let is_up = key == gdk::Key::Up;
                let is_down = key == gdk::Key::Down;

                if key == gdk::Key::less {
                    change_calendar_month(&calendar_state, -1);
                    return gdk::glib::Propagation::Stop;
                }
                if key == gdk::Key::greater {
                    change_calendar_month(&calendar_state, 1);
                    return gdk::glib::Propagation::Stop;
                }
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
                if key == gdk::Key::h || (is_left && !ctrl) {
                    navigate_calendar(&calendar_state, -1, 0);
                    return gdk::glib::Propagation::Stop;
                }
                if key == gdk::Key::l || (is_right && !ctrl) {
                    navigate_calendar(&calendar_state, 1, 0);
                    return gdk::glib::Propagation::Stop;
                }
                if key == gdk::Key::k || is_up {
                    navigate_calendar(&calendar_state, 0, -1);
                    return gdk::glib::Propagation::Stop;
                }
                if key == gdk::Key::j || is_down {
                    navigate_calendar(&calendar_state, 0, 1);
                    return gdk::glib::Propagation::Stop;
                }
                match key {
                    k if k == gdk::Key::i => {
                        if let Some(date) = get_selected_calendar_date(&calendar_state) {
                            *input_mode.borrow_mut() = InputMode::CalendarInsert(date);
                            mode_label.set_text("INSERT (calendar)");
                            command_entry.set_placeholder_text(Some(&format!(
                                "Task for {}...",
                                date.format("%b %d")
                            )));
                            command_entry.set_text("");
                            command_entry.set_sensitive(true);
                            command_entry.grab_focus();
                        }
                        return gdk::glib::Propagation::Stop;
                    }
                    k if k == gdk::Key::colon && shift => {
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

            // Plugin view — let it handle its own keys
            if let ViewType::Plugin(_) = *view_type.borrow() {
                // Extension point: plugin views handle their own keys via TabView::on_key
                // (add that method to the TabView trait if needed).
                return gdk::glib::Propagation::Proceed;
            }

            // List view — build ActionContext and dispatch
            let ctx = ActionContext {
                todos: tab_todos,
                list_box,
                flat_todos,
                display_settings: display_settings.clone(),
                inline_entry_row,
                input_mode: input_mode.clone(),
                mode_label: mode_label.clone(),
                command_entry: command_entry.clone(),
            };

            let pending = pending_key.borrow().clone();
            if let Some(ref pending_str) = pending {
                if let Some(action) = keybindings.get_sequence_action(pending_str, &key) {
                    *pending_key.borrow_mut() = None;
                    return execute_action(action, &ctx);
                }
                *pending_key.borrow_mut() = None;
            }

            if let Some(seq_start) = keybindings.is_sequence_start(&key) {
                *pending_key.borrow_mut() = Some(seq_start);
                return gdk::glib::Propagation::Stop;
            }

            if let Some(action) = keybindings.get_action(&key, shift, ctrl, alt) {
                *pending_key.borrow_mut() = None;
                return execute_action(action, &ctx);
            }

            *pending_key.borrow_mut() = None;
            gdk::glib::Propagation::Proceed
        });

        self.window.add_controller(key_controller);
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

fn hash_bytes(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}
