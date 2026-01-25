use gtk4::gdk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// All possible actions that can be triggered by keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // Navigation
    MoveDown,
    MoveUp,
    JumpToFirst,  // gg
    JumpToLast,   // G

    // Task operations
    ToggleComplete,
    Delete,  // dd
    MoveTaskDown,
    MoveTaskUp,
    ToggleFold,  // za

    // Insert modes
    Insert,
    InsertSubtask,
    Edit,

    // Command mode
    CommandMode,

    // Cancel/escape
    Cancel,
}
/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    pub bindings: HashMap<String, KeyBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    pub action: Action,
    /// For multi-key sequences like "gg" or "dd", this is the pending key
    #[serde(default)]
    pub pending: Option<String>,
}

/// Runtime keybindings manager
pub struct Keybindings {
    /// Direct key -> action mappings (single key press)
    single_key: HashMap<(String, bool, bool, bool), Action>,
    /// Two-key sequences like "gg", "dd", "za"
    sequences: HashMap<(String, String), Action>,
}

impl Keybindings {
    pub fn load() -> Self {
        let config = Self::load_config();
        Self::from_config(&config)
    }

    fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("zap");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("keybindings.json")
    }

    fn load_config() -> KeybindingsConfig {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }

        // Create default config
        let config = Self::default_config();
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            fs::write(&path, json).ok();
        }
        config
    }

    fn default_config() -> KeybindingsConfig {
        let mut bindings = HashMap::new();

        // Navigation
        bindings.insert("move_down".to_string(), KeyBinding {
            key: "j".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::MoveDown,
            pending: None,
        });
        bindings.insert("move_up".to_string(), KeyBinding {
            key: "k".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::MoveUp,
            pending: None,
        });
        bindings.insert("jump_to_first".to_string(), KeyBinding {
            key: "g".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::JumpToFirst,
            pending: Some("g".to_string()),
        });
        bindings.insert("jump_to_last".to_string(), KeyBinding {
            key: "G".to_string(),
            shift: true, ctrl: false, alt: false,
            action: Action::JumpToLast,
            pending: None,
        });

        // Task operations
        bindings.insert("toggle_complete".to_string(), KeyBinding {
            key: "Return".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::ToggleComplete,
            pending: None,
        });
        bindings.insert("delete".to_string(), KeyBinding {
            key: "d".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::Delete,
            pending: Some("d".to_string()),
        });
        bindings.insert("move_task_down".to_string(), KeyBinding {
            key: "J".to_string(),
            shift: true, ctrl: false, alt: false,
            action: Action::MoveTaskDown,
            pending: None,
        });
        bindings.insert("move_task_up".to_string(), KeyBinding {
            key: "K".to_string(),
            shift: true, ctrl: false, alt: false,
            action: Action::MoveTaskUp,
            pending: None,
        });
        bindings.insert("toggle_fold".to_string(), KeyBinding {
            key: "a".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::ToggleFold,
            pending: Some("z".to_string()),
        });

        // Insert modes
        bindings.insert("insert".to_string(), KeyBinding {
            key: "i".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::Insert,
            pending: None,
        });
        bindings.insert("insert_subtask".to_string(), KeyBinding {
            key: "Return".to_string(),
            shift: true, ctrl: false, alt: false,
            action: Action::InsertSubtask,
            pending: None,
        });
        bindings.insert("edit".to_string(), KeyBinding {
            key: "e".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::Edit,
            pending: None,
        });

        // Command mode
        bindings.insert("command_mode".to_string(), KeyBinding {
            key: "colon".to_string(),
            shift: true, ctrl: false, alt: false,
            action: Action::CommandMode,
            pending: None,
        });

        // Cancel
        bindings.insert("cancel".to_string(), KeyBinding {
            key: "Escape".to_string(),
            shift: false, ctrl: false, alt: false,
            action: Action::Cancel,
            pending: None,
        });

        KeybindingsConfig { bindings }
    }

    fn from_config(config: &KeybindingsConfig) -> Self {
        let mut single_key = HashMap::new();
        let mut sequences = HashMap::new();

        for binding in config.bindings.values() {
            if let Some(ref pending) = binding.pending {
                // This is a sequence like "gg", "dd", "za"
                sequences.insert(
                    (pending.clone(), binding.key.clone()),
                    binding.action,
                );
            } else {
                // Single key binding
                single_key.insert(
                    (binding.key.clone(), binding.shift, binding.ctrl, binding.alt),
                    binding.action,
                );
            }
        }

        Self { single_key, sequences }
    }

    /// Get action for a single key press
    pub fn get_action(&self, key: &gdk::Key, shift: bool, ctrl: bool, alt: bool) -> Option<Action> {
        let key_name = key_to_string(key);
        self.single_key.get(&(key_name, shift, ctrl, alt)).copied()
    }

    /// Get action for a two-key sequence
    pub fn get_sequence_action(&self, pending: &str, key: &gdk::Key) -> Option<Action> {
        let key_name = key_to_string(key);
        self.sequences.get(&(pending.to_string(), key_name)).copied()
    }

    /// Check if a key starts a sequence
    pub fn is_sequence_start(&self, key: &gdk::Key) -> Option<String> {
        let key_name = key_to_string(key);
        for (pending, _) in self.sequences.keys() {
            if pending == &key_name {
                return Some(key_name);
            }
        }
        None
    }
}

/// Convert a GDK key to a string name
fn key_to_string(key: &gdk::Key) -> String {
    match *key {
        k if k == gdk::Key::Return => "Return".to_string(),
        k if k == gdk::Key::Escape => "Escape".to_string(),
        k if k == gdk::Key::Tab => "Tab".to_string(),
        k if k == gdk::Key::BackSpace => "BackSpace".to_string(),
        k if k == gdk::Key::colon => "colon".to_string(),
        k if k == gdk::Key::space => "space".to_string(),
        _ => key.name().map(|s| s.to_string()).unwrap_or_default(),
    }
}
