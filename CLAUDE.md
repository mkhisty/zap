# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build              # debug build
cargo build --release    # release build
cargo run                # run the app
```

## Testing

```bash
cargo test                              # run all tests
cargo test --lib date_parser::tests     # run date_parser tests (only module with tests)
```

## Architecture

Zap is a vim-style keyboard-first GTK4 todo list app written in Rust. It uses a single-window GTK4 application with no async runtime.

### Module Overview

- **`main.rs`** — GTK4 `Application` bootstrap, creates `ZapWindow`
- **`todo.rs`** — Data model: `Todo`, `TodoList`, `FlatTodo`, `Priority`. TodoList handles persistence (JSON files in `~/.local/share/zap/`), tree flattening with fold state, and path-based navigation into nested subtasks
- **`ui/window.rs`** — `ZapWindow` construction, key event dispatching, and top-level GTK widget setup. Handles vim-style modal input (Normal/Insert/Edit/Command/CalendarInsert modes).
- **`ui/types.rs`** — Shared enums: `InputMode`, `ViewType`, `DisplaySettings`.
- **`ui/tab.rs`** — `TabContent` struct (per-tab state: list box, flat todo cache, calendar state, view stack) and `new_tab_content()` constructor.
- **`ui/tab_view.rs`** — `TabView` trait: the interface for pluggable tab views (`widget()`, `refresh()`, `view_name()`). Implement this to add a new `:e <name>` view.
- **`ui/actions.rs`** — `execute_action()` dispatches keybinding `Action` variants against an `ActionContext`. Also contains `parse_task_input()` which runs the full parse pipeline (priority → date → color).
- **`ui/commands.rs`** — Command-mode entry handler (`setup_entry_handler`): processes `:` commands (new cluster, rename, delete, toggle display settings, etc.). Also handles custom shell commands from `~/.config/zap/commands/`.
- **`ui/list_view.rs`** — Row rendering (`create_todo_row`) and list refresh helpers.
- **`ui/calendar_view.rs`** — Calendar grid construction and refresh.
- **`ui/color_picker.rs`** — Color picker dialog for per-task color selection. Reference implementation for modal dialogs.
- **`keybindings.rs`** — Configurable keybinding system supporting single keys and two-key sequences (gg, dd, za). Config stored at `~/.config/zap/keybindings.json`
- **`colors.rs`** — `ColorConfig` with full CSS generation for GTK styling. Config stored at `~/.config/zap/colors.json`. Also provides per-cluster accent colors from a deterministic palette
- **`date_parser.rs`** — Parses bracket syntax (`[date:tomorrow]`, `[p:high]`, `[color:#ff0000]`) from task input text, stripping markers and returning structured data
- **`hooks.rs`** — Fire-and-forget shell hook system. Spawns executable scripts from `~/.config/zap/hooks/<event-name>` with `ZAP_EVENT`, `ZAP_DATA_DIR`, `ZAP_TASK_ID`, `ZAP_TASK_TEXT` env vars. Events: `on-app-start`, `on-app-quit`, `on-task-create`, `on-task-complete`, `on-task-delete`, `on-task-edit`, `on-task-abandon`.

### Key Patterns

- **Path-based tree addressing**: Todos are a tree (subtasks). A `Vec<usize>` path addresses any node (e.g., `[2, 0]` = first subtask of third top-level todo). `FlatTodo` flattens this tree for display while preserving paths.
- **State via `Rc<RefCell<...>>`**: GTK4 callbacks share mutable state through `Rc<RefCell<>>` wrappers. Each tab has its own `TabContent` with independent `TodoList`, `ListBox`, and flat todo cache.
- **Input parsing pipeline**: Task text goes through `parse_priority()` → `parse_date()` → `parse_color()` in sequence, each stripping its bracket markers and returning remaining text + parsed value. The pipeline lives in `parse_task_input()` (`ui/actions.rs`).
- **Auto-save**: `TodoList::save()` is called after every mutation (add, delete, toggle, move, edit). No explicit save command.
- **CSS-based theming**: `ColorConfig::generate_css()` produces a full GTK CSS stylesheet. Accent colors are applied per-cluster via a separate CSS provider.
- **Sections**: Task text starting with `/section <name>` creates a styled `§` header row (not a real task). Handled in `list_view.rs` during row creation.

### Extension Points

The four ways to extend Zap without patching core (all documented in detail in README.md):

1. **Shell hooks** — Drop executables into `~/.config/zap/hooks/`. Call `hooks::fire()` from Rust to emit new events.
2. **Custom commands** — Drop executables into `~/.config/zap/commands/<name>`; callable as `:<name> [arg]` from command mode. To add a built-in Rust command, add an `else if` branch in `commands.rs::setup_entry_handler()` and register it in `autocomplete_command()`.
3. **Bracket syntax** — Add a `parse_<marker>()` function in `date_parser.rs`, a field on `Todo` (with `#[serde(default)]`), wire it through `ParsedInput` in `actions.rs`, and pass it through `Todo::new()`.
4. **Tab views** — Implement `TabView` trait, declare the module in `ui/mod.rs`, add a match arm in the `":e "` block in `commands.rs`, and add it to `autocomplete_command()`.
