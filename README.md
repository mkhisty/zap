# Zap

A minimal, vim-style keyboard-first todo list app built with GTK4 and Rust.

## Installation

```bash
cargo build --release
./target/release/zap
```

## Keybindings

### Normal Mode — Navigation

| Key | Action |
|-----|--------|
| `j` | Move selection down |
| `k` | Move selection up |
| `gg` | Jump to first item |
| `G` | Jump to last item |

### Normal Mode — Task Operations

| Key | Action |
|-----|--------|
| `Enter` | Toggle task completion |
| `Alt+Enter` | Mark task as abandoned (never-done) |
| `dd` | Delete selected task |
| `J` | Move task down |
| `K` | Move task up |
| `za` | Toggle fold/unfold subtasks |
| `i` | Insert new top-level task |
| `Shift+Enter` | Insert subtask under selected task |
| `e` | Edit selected task |
| `:` | Enter command mode |

### Global Shortcuts (any mode)

| Key | Action |
|-----|--------|
| `Ctrl+T` | Open new tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Shift+C` | Open color picker for selected task (Normal mode) |
| `Escape` | Return to Normal mode / cancel |

### Calendar View Navigation

| Key | Action |
|-----|--------|
| `h` / `←` | Previous day |
| `l` / `→` | Next day |
| `k` / `↑` | Previous week |
| `j` / `↓` | Next week |
| `<` / `Ctrl+←` | Previous month |
| `>` / `Ctrl+→` | Next month |
| `i` | Insert task on selected date |

## Commands

Enter command mode with `:`. Press `Tab` to autocomplete.

| Command | Action |
|---------|--------|
| `:e calendar` | Switch current tab to calendar view |
| `:e list` | Switch current tab to list view |
| `:sort` | Sort tasks by priority → completion → due date → alphabetically |
| `:flatten` | Toggle between hierarchical and flat (all tasks at one level) view |
| `:display_start` | Toggle showing task creation dates |

## Task Input Syntax

Metadata markers can appear anywhere in the task text. They are stripped from the display but stored internally.

### Priority

```
[priority:max]    or  [p:max]
[priority:high]   or  [p:high]
[priority:medium] or  [p:medium]
[priority:low]    or  [p:low]
```

### Due Date

```
[date:today]          [d:today]
[date:tomorrow]       [d:tom]
[date:monday]         [d:fri]        # next occurrence of weekday
[date:next monday]                   # skip to following week
[date:jan 15]         [d:january 15]
[date:+3]             [d:3d]         # relative days
[d:3/15]                             # mm/dd (current year)
[d:3/15/25]                          # mm/dd/yy
[d:3/15/2025]                        # mm/dd/yyyy
```

### Color

```
[color:#ff0000]   or  [c:#f00]       # hex color (3 or 6 digit)
[color:red]       or  [c:blue]       # CSS color name
[color:none]                         # remove color
```

Colors are inherited by subtasks and shown as stacked left-border bars.

### Sections

```
/section Section Name
```

Creates a styled `§` header (not a task). Works in both top-level insert and subtask insert modes.

### Combined Examples

```
Fix login bug [p:high] [d:tomorrow] [c:#e06c75]
Meeting [date:next monday] [p:medium]
Deploy release [d:+3]
/section Work
```

## Color Picker

Press `Ctrl+Shift+C` in Normal mode to open a color swatch dialog for the selected task. Click a color to apply it, or **None** to remove it. Press `Escape` to dismiss without changes.

## Sorting

`:sort` orders tasks by:
1. Incomplete before complete
2. Non-abandoned before abandoned
3. Priority: Max → High → Medium → Low → None
4. Due date: earlier first, no due date last
5. Alphabetical (case-insensitive)

## Configuration

Config files are auto-generated on first run at `~/.config/zap/`.

### Keybindings — `~/.config/zap/keybindings.json`

Single keys and two-key sequences are supported:

```json
{
  "move_down": "j",
  "move_up": "k",
  "go_to_first": "gg",
  "go_to_last": "G",
  "toggle_complete": "Return",
  "delete": "dd",
  "move_task_down": "J",
  "move_task_up": "K",
  "toggle_fold": "za",
  "insert": "i",
  "edit": "e"
}
```

### Colors — `~/.config/zap/colors.json`

```json
{
  "main_bg": "#1e1e1e",
  "text_color": "#abb2bf",
  "priority_low": "#56b6c2",
  "priority_medium": "#e5c07b",
  "priority_high": "#e06c75",
  "priority_max": "#e06c75",
  "priority_max_bg": "#5c2f2f",
  "section_bg": "#2a2a2a",
  "section_border": "#7c5cbf"
}
```

## Data Storage

Tasks are saved as JSON after every change:

- `~/.local/share/zap/main.json`

Multiple tabs share the same underlying task list.

---

## Extending Zap

Zap has four extension points designed for low-friction customisation without patching the core.

### 1. Shell Hooks

Zap fires hook scripts at key lifecycle events. Place executable scripts in `~/.config/zap/hooks/` named after the event. Scripts that don't exist are silently skipped.

**Event names / script filenames:**

| Event | Filename |
|-------|----------|
| App launched | `on-app-start` |
| App closed | `on-app-quit` |
| Task created | `on-task-create` |
| Task completed | `on-task-complete` |
| Task deleted | `on-task-delete` |
| Task edited | `on-task-edit` |
| Task abandoned | `on-task-abandon` |

**Environment variables passed to every hook:**

| Variable | Value |
|----------|-------|
| `ZAP_EVENT` | The event name (e.g. `on-task-create`) |
| `ZAP_DATA_DIR` | Path to `~/.local/share/zap/` |
| `ZAP_TASK_ID` | UUID of the affected task (task events only) |
| `ZAP_TASK_TEXT` | Display text of the affected task (task events only) |

**Example** — log task completions:
```bash
#!/usr/bin/env bash
# ~/.config/zap/hooks/on-task-complete
echo "$(date -Is) $ZAP_TASK_TEXT" >> ~/completed-tasks.log
```

Hooks are fire-and-forget subprocesses; they never block the UI.

### 2. Custom Shell Commands

Place an executable script at `~/.config/zap/commands/<name>` and invoke it from Zap's command mode as `:<name> [arg]`.

```
:mycommand some argument
```

Zap passes the argument string as `$1` and sets:

| Variable | Value |
|----------|-------|
| `ZAP_CLUSTER` | Active cluster name (e.g. `main`) |

**To add a built-in Rust command** instead of a shell script:
1. Add an `else if cmd == ":<name>"` branch in `src/ui/commands.rs` inside `setup_entry_handler()`.
2. Add the command string to the `commands` array in `autocomplete_command()` so Tab-completion works.

### 3. Custom Bracket Syntax

Bracket markers (`[date:tomorrow]`, `[p:high]`, `[color:#ff0000]`) are parsed in `src/date_parser.rs`.

**To add a new marker (e.g. `[remind:...]`):**

1. **Add a parser function** in `src/date_parser.rs` following the pattern of `parse_date()`:
   ```rust
   /// Strips `[remind:<value>]` from text and returns (remaining_text, Option<value>).
   pub fn parse_remind(input: &str) -> (String, Option<String>) { ... }
   ```
2. **Add a field** to `Todo` in `src/todo.rs` (e.g. `pub remind_at: Option<String>`), with `#[serde(default)]`.
3. **Add a field** to `ParsedInput` in `src/ui/actions.rs` (e.g. `pub remind_at: Option<String>`).
4. **Wire the parser** into `parse_task_input()` in `src/ui/actions.rs`, calling your new function in the pipeline and populating `ParsedInput`.
5. **Pass the value** through `Todo::new()` and `TodoList::update_at_path()` as needed.

### 4. New Tab Views

Custom tab views appear via `:e <name>` in command mode. A view is a Rust struct implementing the `TabView` trait (`src/ui/tab_view.rs`):

```rust
pub trait TabView {
    fn widget(&self) -> gtk4::Widget;           // root GTK widget
    fn refresh(&self, todos: &TodoList);        // called on data change
    fn view_name(&self) -> &str;               // matches the `:e <name>` identifier
}
```

**To add a new view (e.g. "board"):**

1. **Create** `src/ui/board_view.rs` and implement `TabView` for your struct.
2. **Declare** the module in `src/ui/mod.rs`: `mod board_view;`
3. **Add a match arm** in `commands.rs` inside the `":e "` match block, before the `name => { ... }` fallback:
   ```rust
   "board" => {
       let mut tabs_mut = tabs.borrow_mut();
       let tab = &mut tabs_mut[current_page];
       let view = board_view::BoardView::new(&todos.borrow());
       tab.scrolled_plugin.set_child(Some(&view.widget()));
       *tab.plugin_view.borrow_mut() = Some(Box::new(view));
       *tab.view_type.borrow_mut() = ViewType::Plugin("board".to_string());
       tab.content_stack.set_visible_child_name("plugin");
       tab.tab_label_widget.set_text("[board]");
   }
   ```
4. **Add autocomplete** — include `":e board"` in the `commands` array in `autocomplete_command()`.

When the todo list changes, `TabView::refresh()` is called automatically (add a call to the file-watcher loop in `window.rs` if you want live updates).

### 5. Dialogs

For modal dialogs (e.g. a settings panel or a date picker), see `src/ui/color_picker.rs` as the reference implementation:

- Create a `gtk4::Window` with `set_transient_for(&parent_window)` and `set_modal(true)`.
- Connect `connect_close_request` or button signals to perform the action and close.
- Present with `.present()`.
