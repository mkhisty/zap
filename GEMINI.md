# GEMINI.md

## Project Overview

**Zap** is a minimal, vim-style keyboard-first todo list application built with **Rust** and **GTK4**. It features a modal interface (Normal, Insert, Edit, Command modes) and supports hierarchical tasks with subtasks. Data is persisted automatically as JSON in `~/.local/share/zap/`, and configuration is managed via JSON files in `~/.config/zap/`.

### Core Technologies
- **Language:** Rust (2021 edition)
- **UI Framework:** GTK4 (via `gtk4-rs`)
- **Data Handling:** `serde` / `serde_json` for persistence
- **Time/Date:** `chrono` for task scheduling
- **Pattern Matching:** `regex` for bracket syntax parsing

### Architecture
- **Data Model (`src/todo.rs`):** Uses a tree structure for tasks. `TodoList` handles persistence, folding state, and path-based addressing (`Vec<usize>`) for nested tasks.
- **UI Management (`src/ui/window.rs`):** The `ZapWindow` manages the main GTK loop, tab switching, and modal input dispatching.
- **Input Parsing (`src/ui/actions.rs` & `src/date_parser.rs`):** Employs a pipeline to strip metadata markers (priority, date, color) from task text using bracket syntax (e.g., `[p:high]`, `[d:tomorrow]`).
- **Extensibility:** Support for shell hooks (`src/hooks.rs`), custom shell commands, and pluggable tab views via the `TabView` trait.

---

## Building and Running

### Prerequisites
- Rust and Cargo installed.
- GTK4 development libraries (e.g., `libgtk-4-dev` on Debian/Ubuntu, `gtk4` on Fedora/Arch).

### Commands
- **Build:** `cargo build`
- **Run:** `cargo run`
- **Release Build:** `cargo build --release`
- **Test:** `cargo test` (primarily tests date and priority parsing in `src/date_parser.rs`)

---

## Development Conventions

### Task Addressing
Tasks are addressed using a **path-based approach** (`Vec<usize>`). For example, a path of `[1, 0]` refers to the first subtask of the second top-level task. Always use `get_mut_at_path` or `get_at_path` when operating on specific tasks.

### Modal Input
The application operates in different `InputMode` states:
- `Normal`: Navigation and task operations.
- `Insert` / `InsertSubtask`: Adding new tasks.
- `Edit`: Modifying existing task text.
- `Command`: Executing `:` commands.
- `CalendarInsert`: Adding tasks from the calendar view.

### Persistence & State
- **Auto-save:** `TodoList::save()` is called after every mutation. There is no manual save button.
- **State Management:** GTK callbacks use `Rc<RefCell<...>>` for shared mutable state.
- **File Watcher:** The application watches the main data file and refreshes the UI if external changes are detected.

### Extension Points
1. **Shell Hooks:** Executables in `~/.config/zap/hooks/` triggered by events (e.g., `on-task-complete`).
2. **Custom Commands:** Scripts in `~/.config/zap/commands/` or built-in Rust commands in `src/ui/commands.rs`.
3. **Bracket Syntax:** New markers can be added by implementing a parser in `src/date_parser.rs` and wiring it through `src/ui/actions.rs`.
4. **Tab Views:** Implement the `TabView` trait to add new views accessible via `:e <name>`.

### CSS Theming
Styling is handled via `src/colors.rs`, which generates a GTK CSS stylesheet from `~/.config/zap/colors.json`. Avoid hardcoding styles in Rust; use CSS classes and the configuration system.
