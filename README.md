# Zap

A minimal, high-speed, vim-style keyboard-first todo list application built with GTK4 and Rust.

## Features

- Vim-style keybindings
- Flexible date and priority syntax with bracket notation
- Priority markers with color coding
- Nested subtasks with folding
- Multiple task clusters (separate lists)
- Section headers for organization

## Installation

```bash
cargo build --release
```

## Usage

```bash
cargo run
```

## Keybindings

### Navigation (Normal Mode)

| Key | Action |
|-----|--------|
| `j` | Move selection down |
| `k` | Move selection up |
| `gg` | Jump to first item |
| `G` | Jump to last item |

### Task Operations

| Key | Action |
|-----|--------|
| `Enter` | Toggle task completion |
| `dd` | Delete selected task |
| `J` (Shift+j) | Move task down in order |
| `K` (Shift+k) | Move task up in order |
| `za` | Toggle fold/unfold subtasks |

### Insert Modes

| Key | Action |
|-----|--------|
| `i` | Insert new task (inline at bottom of list) |
| `Shift+Enter` | Insert subtask under selected item |
| `e` | Edit selected task text |
| `Escape` | Cancel and return to normal mode |

### Command Mode

| Key | Action |
|-----|--------|
| `:` | Enter command mode |
| `Tab` | Autocomplete command/cluster name |
| `Escape` | Cancel command |

## Task Input Syntax

When inserting or editing tasks, you can use the following syntax:

### Basic Task
```
task text
```

### Due Dates

Use `[date:...]` or `[d:...]` anywhere in the task (case-insensitive):

```
task text [date:today]        # Today
task text [date:tomorrow]     # Tomorrow (also: tom)
task text [date:monday]       # Next occurrence of weekday
task text [date:next friday]  # Skip to next week's weekday
task text [date:jan 15]       # Month and day (also: january 15)
task text [date:+3]           # 3 days from now
task text [date:5d]           # 5 days from now
task text [d:3/15]            # mm/dd (current year)
task text [d:3/15/25]         # mm/dd/yy
task text [d:3/15/2025]       # mm/dd/yyyy
```

### Priority Markers

Use `[priority:LEVEL]` or `[p:LEVEL]` anywhere in the task (case-insensitive):

```
task text [priority:max]      # Maximum priority (red indicator + red row background)
task text [priority:high]     # High priority (red indicator)
task text [p:medium]          # Medium priority (yellow indicator)
task text [p:low]             # Low priority (cyan indicator)
```

### Combined Example

```
Buy groceries [p:high] [d:tomorrow]
```

### Sections

Create organizational headers (not tasks):
```
/section Section Name
```

## Commands

| Command | Action |
|---------|--------|
| `:ls` | List all clusters |
| `:e cluster_name` | Open/switch to cluster |
| `:n cluster_name` | Create new cluster and open it |
| `:display_start` | Toggle showing task creation dates |

## Configuration

Configuration files are stored in `~/.config/zap/`:

### Keybindings (`keybindings.json`)

Customize keyboard shortcuts. Auto-generated with defaults on first run.

### Colors (`colors.json`)

Customize all UI colors. Auto-generated with defaults on first run. Example settings:
```json
{
  "main_bg": "#1e1e1e",
  "priority_low": "#56b6c2",
  "priority_medium": "#e5c07b",
  "priority_high": "#e06c75",
  "priority_max": "#e06c75",
  "priority_max_bg": "#5c2f2f"
}
```

## Data Storage

Tasks are stored as JSON files in:
- Linux: `~/.local/share/zap/`
- Default cluster: `main.json`

## License

MIT
