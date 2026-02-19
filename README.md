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
