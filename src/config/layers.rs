// Config Layers - 6 层配置架构
//
// v0.7.0 - 实现 User/Workspace/Global/Naming/Agent/Session 配置层级

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use regex::Regex;

// ============================================================================
// 配置层级定义
// ============================================================================

/// 配置层级（优先级从高到低）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigLayer {
    /// Layer 1: 用户配置（最高优先级）
    User,
    /// Layer 2: 工作空间配置
    Workspace,
    /// Layer 3: 全局默认配置
    Global,
    /// Layer 4: Naming 匹配配置
    Naming,
    /// Layer 5: Agent 级配置
    Agent,
    /// Layer 6: 会话级配置（最低优先级）
    Session,
}

impl ConfigLayer {
    /// 获取优先级（数字越大优先级越高）
    pub fn priority(&self) -> u8 {
        match self {
            ConfigLayer::User => 6,
            ConfigLayer::Workspace => 5,
            ConfigLayer::Global => 4,
            ConfigLayer::Naming => 3,
            ConfigLayer::Agent => 2,
            ConfigLayer::Session => 1,
        }
    }
}

// ============================================================================
// 各层配置定义
// ============================================================================

/// 用户配置（Layer 1）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub user_id: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub tools: Vec<String>,
    pub preferences: HashMap<String, String>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// 工作空间配置（Layer 2）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    pub workspace_id: String,
    pub name: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub agents: Vec<String>,
    pub env: HashMap<String, String>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Naming 规则配置（Layer 4）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingConfig {
    pub name: String,
    pub pattern: String,
    pub match_type: MatchType,
    pub priority: u32,
    pub model: Option<String>,
    pub tools: Vec<String>,
}

/// 匹配类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    Exact,
    Prefix,
    Suffix,
    Wildcard,
    Regex,
    Tags,
}

/// Agent 配置（Layer 5）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentLayerConfig {
    pub agent_id: String,
    pub name: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub system_prompt: Option<String>,
    pub tags: Vec<String>,
}

/// 会话配置（Layer 6）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionLayerConfig {
    pub session_id: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub variables: HashMap<String, String>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Naming 匹配引擎
// ============================================================================

/// Naming 匹配引擎
pub struct NamingEngine {
    rules: Vec<NamingConfig>,
    regex_cache: HashMap<String, Regex>,
}

impl NamingEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            regex_cache: HashMap::new(),
        }
    }
    
    pub fn add_rule(&mut self, rule: NamingConfig) {
        if rule.match_type == MatchType::Regex {
            if let Ok(re) = Regex::new(&rule.pattern) {
                self.regex_cache.insert(rule.pattern.clone(), re);
            }
        }
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
    
    pub fn find_match(&self, agent_id: &str) -> Option<&NamingConfig> {
        self.rules.iter().find(|rule| self.matches(rule, agent_id))
    }
    
    fn matches(&self, rule: &NamingConfig, agent_id: &str) -> bool {
        match &rule.match_type {
            MatchType::Exact => agent_id == rule.pattern,
            MatchType::Prefix => agent_id.starts_with(&rule.pattern),
            MatchType::Suffix => agent_id.ends_with(&rule.pattern),
            MatchType::Wildcard => self.wildcard_match(&rule.pattern, agent_id),
            MatchType::Regex => {
                self.regex_cache.get(&rule.pattern)
                    .map(|re| re.is_match(agent_id))
                    .unwrap_or(false)
            }
            MatchType::Tags => false,
        }
    }
    
    fn wildcard_match(&self, pattern: &str, text: &str) -> bool {
        let regex_pattern = pattern.replace(".", r"\.").replace("*", ".*");
        Regex::new(&format!("^{}$", regex_pattern))
            .map(|re| re.is_match(text))
            .unwrap_or(false)
    }
}

impl Default for NamingEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 配置上下文和合并
// ============================================================================

/// 配置上下文
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    pub user_id: Option<String>,
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
}

/// 合并后的配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MergedLayerConfig {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub tools: Vec<String>,
    pub sources: HashMap<String, ConfigLayer>,
}

/// 配置层管理器
pub struct ConfigLayerManager {
    user_configs: HashMap<String, UserConfig>,
    workspace_configs: HashMap<String, WorkspaceConfig>,
    naming_engine: NamingEngine,
    agent_configs: HashMap<String, AgentLayerConfig>,
    session_configs: HashMap<String, SessionLayerConfig>,
}

impl ConfigLayerManager {
    pub fn new() -> Self {
        Self {
            user_configs: HashMap::new(),
            workspace_configs: HashMap::new(),
            naming_engine: NamingEngine::new(),
            agent_configs: HashMap::new(),
            session_configs: HashMap::new(),
        }
    }
    
    pub fn set_user_config(&mut self, config: UserConfig) {
        self.user_configs.insert(config.user_id.clone(), config);
    }
    
    pub fn set_workspace_config(&mut self, config: WorkspaceConfig) {
        self.workspace_configs.insert(config.workspace_id.clone(), config);
    }
    
    pub fn add_naming_rule(&mut self, rule: NamingConfig) {
        self.naming_engine.add_rule(rule);
    }
    
    pub fn set_agent_config(&mut self, config: AgentLayerConfig) {
        self.agent_configs.insert(config.agent_id.clone(), config);
    }
    
    pub fn set_session_config(&mut self, config: SessionLayerConfig) {
        self.session_configs.insert(config.session_id.clone(), config);
    }
    
    pub fn resolve(&self, context: &ConfigContext) -> MergedLayerConfig {
        let mut result = MergedLayerConfig::default();
        let mut sources: HashMap<String, ConfigLayer> = HashMap::new();
        
        // 1. Naming 匹配
        if let Some(agent_id) = &context.agent_id {
            if let Some(naming) = self.naming_engine.find_match(agent_id) {
                if naming.model.is_some() { result.model = naming.model.clone(); }
                if !naming.tools.is_empty() { result.tools = naming.tools.clone(); }
                sources.insert("naming".to_string(), ConfigLayer::Naming);
            }
        }
        
        // 2. Agent 配置
        if let Some(agent_id) = &context.agent_id {
            if let Some(agent) = self.agent_configs.get(agent_id) {
                if agent.model.is_some() { result.model = agent.model.clone(); }
                if !agent.tools.is_empty() { result.tools = agent.tools.clone(); }
                sources.insert("agent".to_string(), ConfigLayer::Agent);
            }
        }
        
        // 3. 工作空间配置
        if let Some(workspace_id) = &context.workspace_id {
            if let Some(workspace) = self.workspace_configs.get(workspace_id) {
                if workspace.model.is_some() { result.model = workspace.model.clone(); }
                if !workspace.tools.is_empty() { result.tools = workspace.tools.clone(); }
                sources.insert("workspace".to_string(), ConfigLayer::Workspace);
            }
        }
        
        // 4. 用户配置（最高优先级）
        if let Some(user_id) = &context.user_id {
            if let Some(user) = self.user_configs.get(user_id) {
                if user.model.is_some() { result.model = user.model.clone(); }
                if user.temperature.is_some() { result.temperature = user.temperature; }
                if user.max_tokens.is_some() { result.max_tokens = user.max_tokens; }
                if !user.tools.is_empty() { result.tools = user.tools.clone(); }
                sources.insert("user".to_string(), ConfigLayer::User);
            }
        }
        
        result.sources = sources;
        result
    }
}

impl Default for ConfigLayerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_layer_priority() {
        assert!(ConfigLayer::User.priority() > ConfigLayer::Workspace.priority());
        assert!(ConfigLayer::Workspace.priority() > ConfigLayer::Global.priority());
    }
    
    #[test]
    fn test_naming_engine_exact() {
        let mut engine = NamingEngine::new();
        engine.add_rule(NamingConfig {
            name: "test".to_string(),
            pattern: "agent-001".to_string(),
            match_type: MatchType::Exact,
            priority: 1,
            model: Some("glm-4".to_string()),
            tools: vec![],
        });
        
        assert!(engine.find_match("agent-001").is_some());
        assert!(engine.find_match("agent-002").is_none());
    }
    
    #[test]
    fn test_naming_engine_prefix() {
        let mut engine = NamingEngine::new();
        engine.add_rule(NamingConfig {
            name: "dev".to_string(),
            pattern: "dev-".to_string(),
            match_type: MatchType::Prefix,
            priority: 1,
            model: Some("glm-4-coder".to_string()),
            tools: vec![],
        });
        
        assert!(engine.find_match("dev-001").is_some());
        assert!(engine.find_match("prod-001").is_none());
    }
    
    #[test]
    fn test_naming_engine_wildcard() {
        let mut engine = NamingEngine::new();
        engine.add_rule(NamingConfig {
            name: "test".to_string(),
            pattern: "test-*-unit".to_string(),
            match_type: MatchType::Wildcard,
            priority: 1,
            model: None,
            tools: vec![],
        });
        
        assert!(engine.find_match("test-api-unit").is_some());
        assert!(engine.find_match("test-api-integration").is_none());
    }
    
    #[test]
    fn test_naming_engine_regex() {
        let mut engine = NamingEngine::new();
        engine.add_rule(NamingConfig {
            name: "api".to_string(),
            pattern: r"^api-v\d+$".to_string(),
            match_type: MatchType::Regex,
            priority: 1,
            model: None,
            tools: vec![],
        });
        
        assert!(engine.find_match("api-v1").is_some());
        assert!(engine.find_match("api-v10").is_some());
        assert!(engine.find_match("api-test").is_none());
    }
    
    #[test]
    fn test_config_layer_manager_resolve() {
        let mut manager = ConfigLayerManager::new();
        
        manager.set_user_config(UserConfig {
            user_id: "user-001".to_string(),
            model: Some("glm-5".to_string()),
            temperature: Some(0.6),
            max_tokens: Some(8000),
            tools: vec!["read".to_string(), "write".to_string()],
            ..Default::default()
        });
        
        let context = ConfigContext {
            user_id: Some("user-001".to_string()),
            ..Default::default()
        };
        
        let merged = manager.resolve(&context);
        
        assert_eq!(merged.model, Some("glm-5".to_string()));
        assert_eq!(merged.temperature, Some(0.6));
        assert_eq!(merged.max_tokens, Some(8000));
    }
    
    #[test]
    fn test_naming_priority() {
        let mut engine = NamingEngine::new();
        
        engine.add_rule(NamingConfig {
            name: "low".to_string(),
            pattern: "test-".to_string(),
            match_type: MatchType::Prefix,
            priority: 1,
            model: Some("low-model".to_string()),
            tools: vec![],
        });
        
        engine.add_rule(NamingConfig {
            name: "high".to_string(),
            pattern: "test-".to_string(),
            match_type: MatchType::Prefix,
            priority: 10,
            model: Some("high-model".to_string()),
            tools: vec![],
        });
        
        let matched = engine.find_match("test-001").unwrap();
        assert_eq!(matched.model, Some("high-model".to_string()));
    }
}