use chrono::NaiveDate;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InputMode {
    Normal,
    Insert,
    InsertSubtask(Vec<usize>),
    Edit(Vec<usize>),
    Command,
    CalendarInsert(NaiveDate),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DisplaySettings {
    pub show_start_date: bool,
    pub flattened: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ViewType {
    List,
    Calendar,
    Plugin(String), // name matches TabView::view_name()
}
