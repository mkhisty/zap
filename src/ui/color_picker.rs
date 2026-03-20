use gtk4::prelude::*;
use gtk4::{gdk, ApplicationWindow, Box as GtkBox, Button, EventControllerKey, Grid, Label, Notebook, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::todo::TodoList;
use super::list_view::refresh_list_with_settings;
use super::tab::TabContent;
use super::types::DisplaySettings;

const PICKER_COLORS: &[&str] = &[
    "#e06c75", "#be5046", "#e74c3c", "#c0392b", "#ff6b6b",
    "#ff4757", "#ee5a24", "#e55039", "#fc5c65", "#eb3b5a",
    "#d19a66", "#f78c6c", "#e5c07b", "#f39c12", "#f1c40f",
    "#fed330", "#e67e22", "#fa8231", "#fd9644", "#ffc048",
    "#98c379", "#c3e88d", "#2ecc71", "#27ae60", "#20bf6b",
    "#26de81", "#0fb9b1", "#00d2d3", "#1abc9c", "#16a085",
    "#61afef", "#56b6c2", "#7ec8e3", "#3498db", "#2980b9",
    "#45aaf2", "#2d98da", "#4b7bec", "#3867d6", "#0652dd",
    "#c678dd", "#bb80b3", "#a55eea", "#8854d0", "#6c5ce7",
    "#9b59b6", "#8e44ad", "#e84393", "#fd79a8", "#f368e0",
];

pub(crate) fn show_task_color_picker(
    parent_window: &ApplicationWindow,
    tabs: &Rc<RefCell<Vec<TabContent>>>,
    notebook: &Notebook,
    display_settings: &Rc<RefCell<DisplaySettings>>,
    shared_todos: &Rc<RefCell<TodoList>>,
) {
    let current_page = match notebook.current_page() {
        Some(p) => p as usize,
        None => return,
    };
    let tabs_ref = tabs.borrow();
    let tab = match tabs_ref.get(current_page) {
        Some(t) => t,
        None => return,
    };
    let selected_path = {
        let row = match tab.list_box.selected_row() {
            Some(r) => r,
            None => return,
        };
        let index = row.index() as usize;
        let flat = tab.flat_todos.borrow();
        match flat.get(index) {
            Some(ft) => ft.path.clone(),
            None => return,
        }
    };
    drop(tabs_ref);

    let dialog = gtk4::Window::builder()
        .title("Pick Task Color")
        .transient_for(parent_window)
        .modal(true)
        .default_width(420)
        .default_height(280)
        .build();

    let main_box = GtkBox::new(Orientation::Vertical, 8);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);
    main_box.set_margin_top(12);
    main_box.set_margin_bottom(12);

    let title_label = Label::new(Some("Select task color:"));
    title_label.set_halign(gtk4::Align::Start);
    main_box.append(&title_label);

    let grid = Grid::new();
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);

    let btn_css_provider = gtk4::CssProvider::new();
    let mut css = String::new();
    for (i, color) in PICKER_COLORS.iter().enumerate() {
        css.push_str(&format!(
            ".cpick-{} {{ background-color: {}; background-image: none; box-shadow: none; min-width: 32px; min-height: 32px; border-radius: 4px; border: 1px solid rgba(255,255,255,0.1); padding: 0; }}\n\
             .cpick-{}:hover {{ border: 2px solid white; }}\n",
            i, color, i
        ));
    }
    css.push_str(
        ".cpick-none { background-color: #3e3e3e; background-image: none; box-shadow: none; min-width: 32px; min-height: 32px; border-radius: 4px; border: 1px solid rgba(255,255,255,0.2); padding: 0; }\n\
         .cpick-none:hover { border: 2px solid white; }\n"
    );
    btn_css_provider.load_from_data(&css);

    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(parent_window),
        &btn_css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
    );

    for (i, color) in PICKER_COLORS.iter().enumerate() {
        let col = (i % 10) as i32;
        let row = (i / 10) as i32;

        let btn = Button::new();
        btn.add_css_class(&format!("cpick-{}", i));

        let color_str = color.to_string();
        let todos_c = shared_todos.clone();
        let tabs_c = tabs.clone();
        let notebook_c = notebook.clone();
        let dialog_c = dialog.clone();
        let parent_c = parent_window.clone();
        let btn_css_c = btn_css_provider.clone();
        let ds_c = display_settings.clone();
        let path_c = selected_path.clone();

        btn.connect_clicked(move |_| {
            apply_task_color(&todos_c, &tabs_c, &notebook_c, &ds_c, &path_c, Some(&color_str));
            gtk4::style_context_remove_provider_for_display(
                &gtk4::prelude::WidgetExt::display(&parent_c),
                &btn_css_c,
            );
            dialog_c.close();
        });

        grid.attach(&btn, col, row, 1, 1);
    }

    main_box.append(&grid);

    let none_btn = Button::with_label("None (remove color)");
    none_btn.add_css_class("cpick-none");
    let todos_c = shared_todos.clone();
    let tabs_c = tabs.clone();
    let notebook_c = notebook.clone();
    let dialog_c = dialog.clone();
    let parent_c = parent_window.clone();
    let btn_css_c = btn_css_provider.clone();
    let ds_c = display_settings.clone();
    let path_c = selected_path.clone();
    none_btn.connect_clicked(move |_| {
        apply_task_color(&todos_c, &tabs_c, &notebook_c, &ds_c, &path_c, None);
        gtk4::style_context_remove_provider_for_display(
            &gtk4::prelude::WidgetExt::display(&parent_c),
            &btn_css_c,
        );
        dialog_c.close();
    });
    main_box.append(&none_btn);

    dialog.set_child(Some(&main_box));

    let key_controller = EventControllerKey::new();
    let dialog_for_esc = dialog.clone();
    let parent_for_esc = parent_window.clone();
    let btn_css_for_esc = btn_css_provider.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            gtk4::style_context_remove_provider_for_display(
                &gtk4::prelude::WidgetExt::display(&parent_for_esc),
                &btn_css_for_esc,
            );
            dialog_for_esc.close();
            return gdk::glib::Propagation::Stop;
        }
        gdk::glib::Propagation::Proceed
    });
    dialog.add_controller(key_controller);

    dialog.present();
}

fn apply_task_color(
    shared_todos: &Rc<RefCell<TodoList>>,
    tabs: &Rc<RefCell<Vec<TabContent>>>,
    notebook: &Notebook,
    display_settings: &Rc<RefCell<DisplaySettings>>,
    path: &[usize],
    color: Option<&str>,
) {
    {
        let mut todos = shared_todos.borrow_mut();
        if let Some(todo) = todos.get_mut_at_path(path) {
            todo.color = color.map(|c| c.to_string());
        }
        todos.save();
    }

    let current_page = match notebook.current_page() {
        Some(p) => p as usize,
        None => return,
    };
    let tabs_ref = tabs.borrow();
    let tab = match tabs_ref.get(current_page) {
        Some(t) => t,
        None => return,
    };
    let list_box = tab.list_box.clone();
    let flat_todos = tab.flat_todos.clone();
    let tab_filter = tab.filter.clone();
    drop(tabs_ref);
    refresh_list_with_settings(shared_todos, &list_box, &flat_todos, display_settings, &tab_filter.borrow());
}
