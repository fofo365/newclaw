//! Configuration merger for combining multiple configuration layers
//!
//! This module provides the `ConfigMerger` which handles merging configurations
//! from different layers according to the 6-layer priority system.

use serde::{Deserialize, Serialize};
use super::types::Config;
use super::layers::{ConfigScope, ConfigContext, LayeredConfig};

/// Configuration merger
///
/// Handles merging configurations from multiple layers according to priority.
/// Lower priority layers are merged first, then higher priority layers override.
pub struct ConfigMerger;

impl ConfigMerger {
    /// Merge multiple configurations in order
    ///
    /// Configurations are merged in the order provided, with later configs
    /// overriding earlier ones for the same fields.
    pub fn merge(configs: Vec<Config>) -> Config {
        configs.into_iter()
            .fold(Config::default(), |mut acc, config| {
                acc.merge(&config);
                acc
            })
    }
    
    /// Merge configurations with explicit priority order
    ///
    /// Sorts the configurations by scope priority before merging.
    pub fn merge_with_scopes(configs: Vec<(ConfigScope, Config)>) -> Config {
        let mut sorted: Vec<_> = configs.into_iter().collect();
        sorted.sort_by_key(|(scope, _)| scope.priority());
        
        Self::merge(sorted.into_iter().map(|(_, config)| config).collect())
    }
    
    /// Create a layered config and resolve it for a context
    pub fn resolve_for_context(
        layered: &LayeredConfig,
        context: &ConfigContext,
    ) -> Config {
        let scopes = context.to_scopes();
        layered.resolve(&scopes)
    }
    
    /// Calculate the difference between two configurations
    pub fn diff(old: &Config, new: &Config) -> ConfigDiff {
        let mut changes = Vec::new();
        
        if old.model != new.model {
            changes.push(ConfigChange {
                field: "model".to_string(),
                old_value: Some(old.model.clone()),
                new_value: Some(new.model.clone()),
            });
        }
        
        if (old.temperature - new.temperature).abs() > f32::EPSILON {
            changes.push(ConfigChange {
                field: "temperature".to_string(),
                old_value: Some(old.temperature.to_string()),
                new_value: Some(new.temperature.to_string()),
            });
        }
        
        if old.max_tokens != new.max_tokens {
            changes.push(ConfigChange {
                field: "max_tokens".to_string(),
                old_value: Some(old.max_tokens.to_string()),
                new_value: Some(new.max_tokens.to_string()),
            });
        }
        
        if old.system_prompt != new.system_prompt {
            changes.push(ConfigChange {
                field: "system_prompt".to_string(),
                old_value: old.system_prompt.clone(),
                new_value: new.system_prompt.clone(),
            });
        }
        
        // Check tool changes
        let added_tools: Vec<_> = new.tools.iter()
            .filter(|t| !old.tools.contains(t))
            .collect();
        let removed_tools: Vec<_> = old.tools.iter()
            .filter(|t| !new.tools.contains(t))
            .collect();
        
        if !added_tools.is_empty() {
            changes.push(ConfigChange {
                field: "tools_added".to_string(),
                old_value: None,
                new_value: Some(added_tools.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",")),
            });
        }
        if !removed_tools.is_empty() {
            changes.push(ConfigChange {
                field: "tools_removed".to_string(),
                old_value: Some(removed_tools.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",")),
                new_value: None,
            });
        }
        
        // Check metadata changes
        for (key, value) in &new.metadata {
            match old.metadata.get(key) {
                Some(old_value) if old_value != value => {
                    changes.push(ConfigChange {
                        field: format!("metadata.{}", key),
                        old_value: Some(old_value.clone()),
                        new_value: Some(value.clone()),
                    });
                }
                None => {
                    changes.push(ConfigChange {
                        field: format!("metadata.{}", key),
                        old_value: None,
                        new_value: Some(value.clone()),
                    });
                }
                _ => {}
            }
        }
        
        for key in old.metadata.keys() {
            if !new.metadata.contains_key(key) {
                changes.push(ConfigChange {
                    field: format!("metadata.{}", key),
                    old_value: old.metadata.get(key).cloned(),
                    new_value: None,
                });
            }
        }
        
        ConfigDiff { changes }
    }
}

/// A single configuration change
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigChange {
    /// Field that changed
    pub field: String,
    
    /// Old value (None if field was added)
    pub old_value: Option<String>,
    
    /// New value (None if field was removed)
    pub new_value: Option<String>,
}

impl ConfigChange {
    /// Check if this is an addition
    pub fn is_addition(&self) -> bool {
        self.old_value.is_none() && self.new_value.is_some()
    }
    
    /// Check if this is a removal
    pub fn is_removal(&self) -> bool {
        self.old_value.is_some() && self.new_value.is_none()
    }
    
    /// Check if this is a modification
    pub fn is_modification(&self) -> bool {
        self.old_value.is_some() && self.new_value.is_some()
    }
}

/// Configuration difference between two configs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigDiff {
    /// List of changes
    pub changes: Vec<ConfigChange>,
}

impl ConfigDiff {
    /// Create an empty diff
    pub fn new() -> Self {
        Self { changes: Vec::new() }
    }
    
    /// Check if there are any changes
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
    
    /// Get the number of changes
    pub fn len(&self) -> usize {
        self.changes.len()
    }
    
    /// Get changes by field name
    pub fn get(&self, field: &str) -> Option<&ConfigChange> {
        self.changes.iter().find(|c| c.field == field)
    }
    
    /// Check if a specific field changed
    pub fn has_change(&self, field: &str) -> bool {
        self.changes.iter().any(|c| c.field == field)
    }
    
    /// Get all field names that changed
    pub fn changed_fields(&self) -> Vec<&str> {
        self.changes.iter().map(|c| c.field.as_str()).collect()
    }
    
    /// Filter changes by predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<&ConfigChange>
    where
        F: Fn(&ConfigChange) -> bool,
    {
        self.changes.iter().filter(|c| predicate(c)).collect()
    }
    
    /// Get additions only
    pub fn additions(&self) -> Vec<&ConfigChange> {
        self.filter(|c| c.is_addition())
    }
    
    /// Get removals only
    pub fn removals(&self) -> Vec<&ConfigChange> {
        self.filter(|c| c.is_removal())
    }
    
    /// Get modifications only
    pub fn modifications(&self) -> Vec<&ConfigChange> {
        self.filter(|c| c.is_modification())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_merge_basic() {
        let configs = vec![
            Config::new("glm-4").with_temperature(0.7),
            Config::new("glm-5").with_temperature(0.5),
        ];
        
        let merged = ConfigMerger::merge(configs);
        
        assert_eq!(merged.model, "glm-5");
        assert_eq!(merged.temperature, 0.5);
    }
    
    #[test]
    fn test_merge_with_scopes() {
        let configs = vec![
            (ConfigScope::Session("s1".into()), Config::new("model-session")),
            (ConfigScope::Global, Config::new("model-global")),
            (ConfigScope::User("u1".into()), Config::new("model-user")),
        ];
        
        let merged = ConfigMerger::merge_with_scopes(configs);
        
        // Session has highest priority
        assert_eq!(merged.model, "model-session");
    }
    
    #[test]
    fn test_diff_no_changes() {
        let config1 = Config::new("glm-4").with_temperature(0.7);
        let config2 = Config::new("glm-4").with_temperature(0.7);
        
        let diff = ConfigMerger::diff(&config1, &config2);
        
        assert!(diff.is_empty());
    }
    
    #[test]
    fn test_diff_model_change() {
        let config1 = Config::new("glm-4");
        let config2 = Config::new("glm-5");
        
        let diff = ConfigMerger::diff(&config1, &config2);
        
        assert_eq!(diff.len(), 1);
        assert!(diff.has_change("model"));
        
        let change = diff.get("model").unwrap();
        assert_eq!(change.old_value, Some("glm-4".to_string()));
        assert_eq!(change.new_value, Some("glm-5".to_string()));
        assert!(change.is_modification());
    }
    
    #[test]
    fn test_diff_multiple_changes() {
        let mut config1 = Config::new("glm-4")
            .with_temperature(0.7)
            .with_max_tokens(4096);
        config1.tools.push("read".to_string());
        
        let mut config2 = Config::new("glm-5")
            .with_temperature(0.5)
            .with_max_tokens(2048);
        config2.tools.push("read".to_string());
        config2.tools.push("write".to_string());
        
        let diff = ConfigMerger::diff(&config1, &config2);
        
        assert!(diff.has_change("model"));
        assert!(diff.has_change("temperature"));
        assert!(diff.has_change("max_tokens"));
        assert!(diff.has_change("tools_added"));
    }
    
    #[test]
    fn test_diff_tools() {
        let mut config1 = Config::new("glm-4");
        config1.tools = vec!["read".to_string(), "exec".to_string()];
        
        let mut config2 = Config::new("glm-4");
        config2.tools = vec!["read".to_string(), "write".to_string()];
        
        let diff = ConfigMerger::diff(&config1, &config2);
        
        assert!(diff.has_change("tools_added"));
        assert!(diff.has_change("tools_removed"));
        
        let added = diff.get("tools_added").unwrap();
        assert_eq!(added.new_value, Some("write".to_string()));
        
        let removed = diff.get("tools_removed").unwrap();
        assert_eq!(removed.old_value, Some("exec".to_string()));
    }
    
    #[test]
    fn test_config_change_types() {
        let addition = ConfigChange {
            field: "test".to_string(),
            old_value: None,
            new_value: Some("value".to_string()),
        };
        assert!(addition.is_addition());
        assert!(!addition.is_removal());
        assert!(!addition.is_modification());
        
        let removal = ConfigChange {
            field: "test".to_string(),
            old_value: Some("value".to_string()),
            new_value: None,
        };
        assert!(!removal.is_addition());
        assert!(removal.is_removal());
        assert!(!removal.is_modification());
        
        let modification = ConfigChange {
            field: "test".to_string(),
            old_value: Some("old".to_string()),
            new_value: Some("new".to_string()),
        };
        assert!(!modification.is_addition());
        assert!(!modification.is_removal());
        assert!(modification.is_modification());
    }
    
    #[test]
    fn test_resolve_for_context() {
        let mut layered = LayeredConfig::new();
        
        layered.set_layer(ConfigLayer::new(
            ConfigScope::Global,
            Config::new("model-global"),
        ));
        
        layered.set_layer(ConfigLayer::new(
            ConfigScope::User("u1".into()),
            Config::new("model-user"),
        ));
        
        let ctx = ConfigContext::new()
            .with_user("u1");
        
        let resolved = ConfigMerger::resolve_for_context(&layered, &ctx);
        
        assert_eq!(resolved.model, "model-user");
    }
    
    use super::super::layers::ConfigLayer;
}