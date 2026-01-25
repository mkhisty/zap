use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Priority {
    #[default]
    None,
    Low,
    #[serde(alias = "Mid")]
    Medium,
    #[serde(alias = "Top")]
    High,
    Max,
}

impl Priority {
    /// Returns sort order (lower = higher priority, sorted first)
    fn sort_order(&self) -> u8 {
        match self {
            Priority::Max => 0,
            Priority::High => 1,
            Priority::Medium => 2,
            Priority::Low => 3,
            Priority::None => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub text: String,
    pub completed: bool,
    pub due_date: Option<NaiveDate>,
    pub created_at: i64,
    #[serde(default)]
    pub subtasks: Vec<Todo>,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub is_section: bool,
}

impl Todo {
    pub fn new(text: String, due_date: Option<NaiveDate>, priority: Priority) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text,
            completed: false,
            due_date,
            created_at: Utc::now().timestamp(),
            subtasks: Vec::new(),
            priority,
            is_section: false,
        }
    }

    pub fn new_section(text: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text,
            completed: false,
            due_date: None,
            created_at: Utc::now().timestamp(),
            subtasks: Vec::new(),
            priority: Priority::None,
            is_section: true,
        }
    }

    pub fn toggle(&mut self) {
        self.completed = !self.completed;
    }

    pub fn has_subtasks(&self) -> bool {
        !self.subtasks.is_empty()
    }
}

/// A flattened view of a todo with its depth level for display
#[derive(Debug, Clone)]
pub struct FlatTodo {
    pub todo: Todo,
    pub depth: usize,
    pub path: Vec<usize>,
    pub has_subtasks: bool,
    pub is_folded: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TodoList {
    pub todos: Vec<Todo>,
    #[serde(skip)]
    cluster_name: String,
    #[serde(skip)]
    folded_ids: HashSet<String>,
}

impl TodoList {
    pub fn data_dir() -> PathBuf {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("zap");
        fs::create_dir_all(&data_dir).ok();
        data_dir
    }

    pub fn cluster_path(name: &str) -> PathBuf {
        Self::data_dir().join(format!("{}.json", name))
    }

    pub fn load(cluster_name: &str) -> Self {
        let path = Self::cluster_path(cluster_name);
        let mut list = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        };
        list.cluster_name = cluster_name.to_string();
        list
    }

    pub fn save(&self) {
        let path = Self::cluster_path(&self.cluster_name);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            fs::write(path, json).ok();
        }
    }

    pub fn cluster_name(&self) -> &str {
        &self.cluster_name
    }

    /// List available clusters
    pub fn list_clusters() -> Vec<String> {
        let dir = Self::data_dir();
        let mut clusters = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_stem() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "json" {
                            clusters.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        clusters.sort();
        clusters
    }

    /// Toggle fold state for a todo by ID
    pub fn toggle_fold(&mut self, id: &str) {
        if self.folded_ids.contains(id) {
            self.folded_ids.remove(id);
        } else {
            self.folded_ids.insert(id.to_string());
        }
    }

    pub fn is_folded(&self, id: &str) -> bool {
        self.folded_ids.contains(id)
    }

    /// Get a flattened list of all todos with depth info, respecting fold state
    pub fn flatten(&self) -> Vec<FlatTodo> {
        let mut result = Vec::new();
        for (i, todo) in self.todos.iter().enumerate() {
            self.flatten_recursive(todo, 0, vec![i], &mut result);
        }
        result
    }

    fn flatten_recursive(&self, todo: &Todo, depth: usize, path: Vec<usize>, result: &mut Vec<FlatTodo>) {
        let is_folded = self.is_folded(&todo.id);
        let has_subtasks = todo.has_subtasks();

        result.push(FlatTodo {
            todo: todo.clone(),
            depth,
            path: path.clone(),
            has_subtasks,
            is_folded,
        });

        // Only include subtasks if not folded
        if !is_folded {
            for (i, subtask) in todo.subtasks.iter().enumerate() {
                let mut sub_path = path.clone();
                sub_path.push(i);
                self.flatten_recursive(subtask, depth + 1, sub_path, result);
            }
        }
    }

    /// Get mutable reference to todo at path
    fn get_mut_at_path(&mut self, path: &[usize]) -> Option<&mut Todo> {
        if path.is_empty() {
            return None;
        }
        let mut current = self.todos.get_mut(path[0])?;
        for &idx in &path[1..] {
            current = current.subtasks.get_mut(idx)?;
        }
        Some(current)
    }

    /// Get immutable reference to todo at path
    pub fn get_at_path(&self, path: &[usize]) -> Option<&Todo> {
        if path.is_empty() {
            return None;
        }
        let mut current = self.todos.get(path[0])?;
        for &idx in &path[1..] {
            current = current.subtasks.get(idx)?;
        }
        Some(current)
    }

    /// Get the parent's subtask list and the index within it
    fn get_parent_list_mut(&mut self, path: &[usize]) -> Option<(&mut Vec<Todo>, usize)> {
        if path.is_empty() {
            return None;
        }
        if path.len() == 1 {
            Some((&mut self.todos, path[0]))
        } else {
            let parent_path = &path[..path.len() - 1];
            let idx = *path.last().unwrap();
            let parent = self.get_mut_at_path(parent_path)?;
            Some((&mut parent.subtasks, idx))
        }
    }

    pub fn add(&mut self, todo: Todo) {
        self.todos.push(todo);
        self.save();
    }

    pub fn add_subtask(&mut self, path: &[usize], subtask: Todo) {
        if let Some(parent) = self.get_mut_at_path(path) {
            parent.subtasks.push(subtask);
            self.save();
        }
    }

    pub fn update_at_path(
        &mut self,
        path: &[usize],
        text: String,
        due_date: Option<NaiveDate>,
        priority: Priority,
    ) {
        if let Some(todo) = self.get_mut_at_path(path) {
            todo.text = text;
            todo.due_date = due_date;
            todo.priority = priority;
            self.save();
        }
    }

    pub fn remove_at_path(&mut self, path: &[usize]) {
        if let Some((list, idx)) = self.get_parent_list_mut(path) {
            if idx < list.len() {
                list.remove(idx);
                self.save();
            }
        }
    }

    pub fn toggle_at_path(&mut self, path: &[usize]) -> Option<usize> {
        let is_completed = if let Some(todo) = self.get_mut_at_path(path) {
            todo.toggle();
            todo.completed
        } else {
            return None;
        };

        // Move completed tasks to the bottom of their list
        let new_index = if is_completed {
            if let Some((list, idx)) = self.get_parent_list_mut(path) {
                let task = list.remove(idx);
                list.push(task);
                Some(list.len() - 1)
            } else {
                None
            }
        } else {
            None
        };

        self.save();
        new_index
    }

    pub fn move_up(&mut self, path: &[usize]) -> bool {
        if let Some((list, idx)) = self.get_parent_list_mut(path) {
            if idx > 0 && idx < list.len() {
                list.swap(idx, idx - 1);
                self.save();
                return true;
            }
        }
        false
    }

    pub fn move_down(&mut self, path: &[usize]) -> bool {
        if let Some((list, idx)) = self.get_parent_list_mut(path) {
            if idx + 1 < list.len() {
                list.swap(idx, idx + 1);
                self.save();
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.flatten().len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.todos.is_empty()
    }

    /// Sort tasks by priority (highest first), then by date (earliest first, None last),
    /// then alphabetically. Also recursively sorts subtasks.
    pub fn sort(&mut self) {
        Self::sort_todos(&mut self.todos);
        self.save();
    }

    fn sort_todos(todos: &mut Vec<Todo>) {
        // Recursively sort subtasks first
        for todo in todos.iter_mut() {
            if !todo.subtasks.is_empty() {
                Self::sort_todos(&mut todo.subtasks);
            }
        }

        // Sort this level: priority (highest first), date (earliest first, None last), alphabetical
        todos.sort_by(|a, b| {
            // Sections stay in place relative to each other but sort after regular tasks
            if a.is_section != b.is_section {
                return a.is_section.cmp(&b.is_section);
            }

            // Completed tasks sort after incomplete tasks
            if a.completed != b.completed {
                return a.completed.cmp(&b.completed);
            }

            // Priority (lower sort_order = higher priority = comes first)
            let priority_cmp = a.priority.sort_order().cmp(&b.priority.sort_order());
            if priority_cmp != std::cmp::Ordering::Equal {
                return priority_cmp;
            }

            // Date (earlier dates first, None last)
            let date_cmp = match (&a.due_date, &b.due_date) {
                (Some(da), Some(db)) => da.cmp(db),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };
            if date_cmp != std::cmp::Ordering::Equal {
                return date_cmp;
            }

            // Alphabetical (case-insensitive)
            a.text.to_lowercase().cmp(&b.text.to_lowercase())
        });
    }
}
