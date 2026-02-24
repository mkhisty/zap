use std::path::PathBuf;

pub enum HookEvent {
    AppStart,
    AppQuit,
    TaskCreate,
    TaskComplete,
    TaskDelete,
    TaskEdit,
    TaskAbandon,
}

impl HookEvent {
    fn name(&self) -> &'static str {
        match self {
            HookEvent::AppStart => "on-app-start",
            HookEvent::AppQuit => "on-app-quit",
            HookEvent::TaskCreate => "on-task-create",
            HookEvent::TaskComplete => "on-task-complete",
            HookEvent::TaskDelete => "on-task-delete",
            HookEvent::TaskEdit => "on-task-edit",
            HookEvent::TaskAbandon => "on-task-abandon",
        }
    }
}

/// Spawns ~/.config/zap/hooks/<event_name> as a subprocess with task context in env vars.
/// Fire-and-forget; never blocks the GTK main loop.
pub fn fire(event: HookEvent, task_id: Option<&str>, task_text: Option<&str>) {
    let script_path: PathBuf = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("zap/hooks")
        .join(event.name());

    if !script_path.exists() {
        return;
    }

    let data_dir = crate::todo::TodoList::data_dir();
    let mut cmd = std::process::Command::new(&script_path);
    cmd.env("ZAP_EVENT", event.name());
    cmd.env("ZAP_DATA_DIR", &data_dir);
    if let Some(id) = task_id {
        cmd.env("ZAP_TASK_ID", id);
    }
    if let Some(text) = task_text {
        cmd.env("ZAP_TASK_TEXT", text);
    }
    cmd.spawn().ok();
}
