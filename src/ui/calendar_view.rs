use chrono::{Datelike, Local, NaiveDate};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Frame, Label, Orientation, ScrolledWindow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::todo::{Priority, TodoList};
use super::tab::CalendarState;

pub(crate) fn create_calendar_view(
    scrolled_calendar: &ScrolledWindow,
    calendar_state: &Rc<RefCell<Option<CalendarState>>>,
) {
    let today = Local::now().date_naive();
    let year = today.year();
    let month = today.month();
    let selected_day = today.day();

    let main_box = GtkBox::new(Orientation::Vertical, 8);
    main_box.set_margin_start(8);
    main_box.set_margin_end(8);
    main_box.set_margin_top(8);
    main_box.set_margin_bottom(8);

    let header_box = GtkBox::new(Orientation::Horizontal, 8);
    header_box.set_halign(gtk4::Align::Center);

    let left_btn = Button::with_label("◀");
    left_btn.add_css_class("calendar-nav-btn");
    header_box.append(&left_btn);

    let month_label = Label::new(None);
    month_label.add_css_class("calendar-header");
    month_label.set_width_chars(15);
    header_box.append(&month_label);

    let right_btn = Button::with_label("▶");
    right_btn.add_css_class("calendar-nav-btn");
    header_box.append(&right_btn);

    main_box.append(&header_box);

    let day_names_box = GtkBox::new(Orientation::Horizontal, 0);
    day_names_box.set_homogeneous(true);
    for day_name in &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
        let label = Label::new(Some(day_name));
        label.add_css_class("calendar-day-header");
        day_names_box.append(&label);
    }
    main_box.append(&day_names_box);

    let grid = gtk4::Grid::new();
    grid.set_row_homogeneous(true);
    grid.set_column_homogeneous(true);
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    grid.add_css_class("calendar-grid");
    main_box.append(&grid);

    scrolled_calendar.set_child(Some(&main_box));

    let state = CalendarState {
        year,
        month,
        selected_day,
        grid,
        day_frames: HashMap::new(),
        month_label,
    };
    *calendar_state.borrow_mut() = Some(state);

    let calendar_state_left = calendar_state.clone();
    left_btn.connect_clicked(move |_| {
        change_calendar_month(&calendar_state_left, -1);
    });

    let calendar_state_right = calendar_state.clone();
    right_btn.connect_clicked(move |_| {
        change_calendar_month(&calendar_state_right, 1);
    });

    refresh_calendar_view(calendar_state);
}

pub(crate) fn refresh_calendar_view(
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

    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    state.month_label.set_text(&format!("{} {}", month_names[(month - 1) as usize], year));

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

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let dim = days_in_month(year, month);
    let first_weekday = first_day.weekday().num_days_from_sunday();

    let today = Local::now().date_naive();
    let todo_list = TodoList::load("main");
    let flat_todos = todo_list.flatten();

    let mut tasks_by_day: HashMap<u32, Vec<_>> = HashMap::new();
    for flat_todo in flat_todos {
        let date = flat_todo.todo.due_date.unwrap_or(today);
        if flat_todo.todo.completed && date < today {
            continue;
        }
        if date.year() == year && date.month() == month {
            tasks_by_day.entry(date.day()).or_default().push(flat_todo);
        }
    }

    for day in 1..=dim {
        let col = ((first_weekday + day - 1) % 7) as i32;
        let row = ((first_weekday + day - 1) / 7) as i32;

        let frame = Frame::new(None);
        frame.add_css_class("calendar-day");

        let day_box = GtkBox::new(Orientation::Vertical, 2);
        day_box.set_margin_start(4);
        day_box.set_margin_end(4);
        day_box.set_margin_top(4);
        day_box.set_margin_bottom(4);

        let day_label = Label::new(Some(&day.to_string()));
        day_label.set_halign(gtk4::Align::Start);
        day_label.add_css_class("calendar-day-number");

        let this_date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        if this_date == today {
            frame.add_css_class("calendar-today");
        }
        if day == selected_day {
            frame.add_css_class("calendar-selected");
        }

        day_box.append(&day_label);

        if let Some(day_tasks) = tasks_by_day.get(&day) {
            for (i, flat_todo) in day_tasks.iter().enumerate() {
                if i >= 3 {
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

fn update_calendar_selection(calendar_state: &Rc<RefCell<Option<CalendarState>>>, new_day: u32) {
    let mut state_ref = calendar_state.borrow_mut();
    if let Some(state) = state_ref.as_mut() {
        if let Some(old_frame) = state.day_frames.get(&state.selected_day) {
            old_frame.remove_css_class("calendar-selected");
        }
        if let Some(new_frame) = state.day_frames.get(&new_day) {
            new_frame.add_css_class("calendar-selected");
            state.selected_day = new_day;
        }
    }
}

pub(crate) fn navigate_calendar(
    calendar_state: &Rc<RefCell<Option<CalendarState>>>,
    delta_days: i32,
    delta_weeks: i32,
) {
    let state_ref = calendar_state.borrow();
    if let Some(state) = state_ref.as_ref() {
        let dim = days_in_month(state.year, state.month);
        let current = state.selected_day as i32;
        let new_day = current + delta_days + (delta_weeks * 7);
        if new_day >= 1 && new_day <= dim as i32 {
            drop(state_ref);
            update_calendar_selection(calendar_state, new_day as u32);
        }
    }
}

pub(crate) fn change_calendar_month(
    calendar_state: &Rc<RefCell<Option<CalendarState>>>,
    delta: i32,
) {
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
            let max_day = days_in_month(new_year, new_month as u32);
            if state.selected_day > max_day {
                state.selected_day = max_day;
            }
        }
    }
    refresh_calendar_view(calendar_state);
}

pub(crate) fn get_selected_calendar_date(
    calendar_state: &Rc<RefCell<Option<CalendarState>>>,
) -> Option<NaiveDate> {
    let state_ref = calendar_state.borrow();
    state_ref.as_ref().and_then(|state| {
        NaiveDate::from_ymd_opt(state.year, state.month, state.selected_day)
    })
}

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

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len - 3])
    }
}
