//! Configuration module for NewClaw
//!
//! This module provides a 6-layer configuration architecture with hot reload support.
//!
//! # Configuration Layers (from lowest to highest priority)
//!
//! 1. **Global** - System-wide defaults
//! 2. **Agent** - Per-agent configuration
//! 3. **Channel** - Per-channel configuration (e.g., QQ, Feishu)
//! 4. **User** - Per-user preferences
//! 5. **Group** - Per-group settings
//! 6. **Session** - Per-session overrides (highest priority)
//!
//! # Hot Reload
//!
//! Configuration files are watched for changes. When a file is modified:
//! 1. The change is detected by `watcher.rs`
//! 2. The configuration is reloaded by `hot_reload.rs`
//! 3. Subscribers are notified via `notification.rs`

pub mod watcher;
pub mod hot_reload;
pub mod notification;
pub mod layers;
pub mod merge;
pub mod types;

pub use watcher::{ConfigWatcher, WatchEvent};
pub use hot_reload::{ConfigHotReloadManager, ConfigVersion};
pub use notification::{ConfigNotificationManager, ConfigChangeEvent, ConfigSubscriber};
pub use layers::{ConfigLayer, ConfigScope, LayeredConfig, ConfigContext};
pub use merge::{ConfigMerger, ConfigDiff};
pub use types::{Config, ConfigError, ConfigResult};

/// Default configuration file paths
pub const DEFAULT_CONFIG_DIR: &str = "config";
pub const GLOBAL_CONFIG_FILE: &str = "config/global.toml";
pub const AGENTS_CONFIG_FILE: &str = "config/agents.toml";
pub const CHANNELS_CONFIG_FILE: &str = "config/channels.toml";
pub const USERS_CONFIG_FILE: &str = "config/users.toml";
pub const GROUPS_CONFIG_FILE: &str = "config/groups.toml";