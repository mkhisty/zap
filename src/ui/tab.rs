use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Frame, Grid, Label, ListBox, ListBoxRow, Notebook, ScrolledWindow, SelectionMode, Stack,
    StackTransitionType,
};

use crate::todo::{FlatTodo, TodoList};
use super::list_view::create_todo_row;
use super::types::{DisplaySettings, ViewType};

pub(crate) struct CalendarState {
    pub year: i32,
    pub month: u32,
    pub selected_day: u32,
    pub grid: Grid,
    pub day_frames: HashMap<u32, Frame>,
    pub month_label: Label,
}

pub(crate) struct TabContent {
    pub list_box: ListBox,
    pub flat_todos: Rc<RefCell<Vec<FlatTodo>>>,
    pub inline_entry_row: Rc<RefCell<Option<ListBoxRow>>>,
    pub view_type: Rc<RefCell<ViewType>>,
    pub calendar_state: Rc<RefCell<Option<CalendarState>>>,
    pub content_stack: Stack,
    #[allow(dead_code)]
    pub scrolled_list: ScrolledWindow,
    pub scrolled_calendar: ScrolledWindow,
    pub tab_label_widget: Label,
}

pub(crate) fn new_tab_content(
    todos: &Rc<RefCell<TodoList>>,
    tabs: &Rc<RefCell<Vec<TabContent>>>,
    notebook: &Notebook,
    display_settings: &Rc<RefCell<DisplaySettings>>,
) -> u32 {
    let flat_todos = Rc::new(RefCell::new(Vec::new()));
    let inline_entry_row: Rc<RefCell<Option<ListBoxRow>>> = Rc::new(RefCell::new(None));
    let view_type = Rc::new(RefCell::new(ViewType::List));
    let calendar_state: Rc<RefCell<Option<CalendarState>>> = Rc::new(RefCell::new(None));

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

    let tab_num = tabs.borrow().len() + 1;
    let tab_label = Label::new(Some(&format!("{}", tab_num)));
    let page_num = notebook.append_page(&content_stack, Some(&tab_label));

    let tab_content = TabContent {
        list_box: list_box.clone(),
        flat_todos: flat_todos.clone(),
        inline_entry_row,
        view_type,
        calendar_state,
        content_stack,
        scrolled_list,
        scrolled_calendar,
        tab_label_widget: tab_label,
    };
    tabs.borrow_mut().push(tab_content);

    let tab_index = tabs.borrow().len() - 1;
    refresh_tab(tab_index, todos, tabs, display_settings);

    notebook.set_current_page(Some(page_num));
    list_box.grab_focus();

    page_num
}

pub(crate) fn refresh_tab(
    tab_index: usize,
    todos: &Rc<RefCell<TodoList>>,
    tabs: &Rc<RefCell<Vec<TabContent>>>,
    display_settings: &Rc<RefCell<DisplaySettings>>,
) {
    let tabs_ref = tabs.borrow();
    if let Some(tab) = tabs_ref.get(tab_index) {
        while let Some(child) = tab.list_box.first_child() {
            tab.list_box.remove(&child);
        }

        let todos_ref = todos.borrow();
        let flat = todos_ref.flatten();
        let settings = display_settings.borrow();

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
