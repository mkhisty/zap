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
- **`ui/actions.rs`** — `execute_action()` dispatches keybinding `Action` variants against an `ActionContext`. Also contains `parse_task_input()` which runs the full parse pipeline (priority → date → color).
- **`ui/commands.rs`** — Command-mode entry handler (`setup_entry_handler`): processes `:` commands (new cluster, rename, delete, toggle display settings, etc.).
- **`ui/list_view.rs`** — Row rendering (`create_todo_row`) and list refresh helpers.
- **`ui/calendar_view.rs`** — Calendar grid construction and refresh.
- **`ui/color_picker.rs`** — Color picker dialog for per-task color selection.
- **`keybindings.rs`** — Configurable keybinding system supporting single keys and two-key sequences (gg, dd, za). Config stored at `~/.config/zap/keybindings.json`
- **`colors.rs`** — `ColorConfig` with full CSS generation for GTK styling. Config stored at `~/.config/zap/colors.json`. Also provides per-cluster accent colors from a deterministic palette
- **`date_parser.rs`** — Parses bracket syntax (`[date:tomorrow]`, `[p:high]`, `[color:#ff0000]`) from task input text, stripping markers and returning structured data

### Key Patterns

- **Path-based tree addressing**: Todos are a tree (subtasks). A `Vec<usize>` path addresses any node (e.g., `[2, 0]` = first subtask of third top-level todo). `FlatTodo` flattens this tree for display while preserving paths.
- **State via `Rc<RefCell<...>>`**: GTK4 callbacks share mutable state through `Rc<RefCell<>>` wrappers. Each tab has its own `TabContent` with independent `TodoList`, `ListBox`, and flat todo cache.
- **Input parsing pipeline**: Task text goes through `parse_priority()` → `parse_date()` in sequence, each stripping its bracket markers and returning remaining text + parsed value.
- **Auto-save**: `TodoList::save()` is called after every mutation (add, delete, toggle, move, edit). No explicit save command.
- **CSS-based theming**: `ColorConfig::generate_css()` produces a full GTK CSS stylesheet. Accent colors are applied per-cluster via a separate CSS provider.
