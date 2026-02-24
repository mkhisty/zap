use gtk4::Widget;

use crate::todo::TodoList;

pub trait TabView {
    /// Returns the root GTK widget for this view.
    #[allow(dead_code)]
    fn widget(&self) -> Widget;
    /// Called whenever the todo list changes; update the view's display.
    fn refresh(&self, todos: &TodoList);
    /// Returns the `:e <name>` identifier (e.g. "board", "timeline").
    fn view_name(&self) -> &str;
}
