// Plugin System for NewClaw

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub entry_point: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    pub plugin_name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
    pub plugin: String,
    pub action: String,
    pub data: serde_json::Value,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    
    fn version(&self) -> &str;
    
    fn init(&mut self, context: PluginContext) -> Result<()>;
    
    fn handle(&self, message: &PluginMessage) -> Result<serde_json::Value>;
    
    fn shutdown(&mut self) -> Result<()>;
}

// In-process plugin registry
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();
        self.plugins.insert(name, plugin);
        Ok(())
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|p| p.as_ref())
    }
    
    pub fn list(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }
    
    pub fn dispatch(&self, message: &PluginMessage) -> Result<serde_json::Value> {
        if let Some(plugin) = self.get(&message.plugin) {
            plugin.handle(message)
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", message.plugin))
        }
    }
    
    pub fn shutdown_all(&mut self) -> Result<()> {
        for (_name, plugin) in self.plugins.iter_mut() {
            plugin.shutdown()?;
        }
        self.plugins.clear();
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Example plugin implementation
pub struct ExamplePlugin {
    name: String,
    version: String,
}

impl ExamplePlugin {
    pub fn new() -> Self {
        Self {
            name: "example".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

impl Plugin for ExamplePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn init(&mut self, _context: PluginContext) -> Result<()> {
        println!("Example plugin initialized");
        Ok(())
    }
    
    fn handle(&self, message: &PluginMessage) -> Result<serde_json::Value> {
        match message.action.as_str() {
            "echo" => Ok(message.data.clone()),
            "uptime" => Ok(serde_json::json!({
                "uptime": "0s",
            })),
            _ => Err(anyhow::anyhow!("Unknown action: {}", message.action)),
        }
    }
    
    fn shutdown(&mut self) -> Result<()> {
        println!("Example plugin shutdown");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        let plugin = ExamplePlugin::new();
        
        registry.register(Box::new(plugin)).unwrap();
        assert_eq!(registry.list().len(), 1);
        
        let message = PluginMessage {
            plugin: "example".to_string(),
            action: "echo".to_string(),
            data: serde_json::json!({"test": "data"}),
        };
        
        let result = registry.dispatch(&message).unwrap();
        assert_eq!(result["test"], "data");
    }
}
