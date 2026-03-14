//! Attribute definitions for ABAC (Attribute-Based Access Control)
//!
//! This module defines:
//! - Attribute categories (Subject, Resource, Action, Environment)
//! - Attribute value types (String, Number, Boolean, List, Set)
//! - Attribute resolver trait and implementations

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// Attribute category for ABAC
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeCategory {
    /// Subject attributes (who is making the request)
    /// Examples: user_id, role, department, clearance_level
    Subject,
    
    /// Resource attributes (what is being accessed)
    /// Examples: resource_id, resource_type, owner, classification
    Resource,
    
    /// Action attributes (what operation is being performed)
    /// Examples: action_type, http_method, operation_name
    Action,
    
    /// Environment attributes (context of the request)
    /// Examples: time, location, ip_address, device_type
    Environment,
}

impl AttributeCategory {
    /// Get the prefix for this category
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Resource => "resource",
            Self::Action => "action",
            Self::Environment => "env",
        }
    }
    
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "subject" | "sub" => Some(Self::Subject),
            "resource" | "res" => Some(Self::Resource),
            "action" | "act" => Some(Self::Action),
            "environment" | "env" => Some(Self::Environment),
            _ => None,
        }
    }
}

impl std::fmt::Display for AttributeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subject => write!(f, "subject"),
            Self::Resource => write!(f, "resource"),
            Self::Action => write!(f, "action"),
            Self::Environment => write!(f, "environment"),
        }
    }
}

/// Attribute value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// String value
    String(String),
    
    /// Numeric value (supports integers and floats)
    Number(f64),
    
    /// Boolean value
    Boolean(bool),
    
    /// List of values (ordered, allows duplicates)
    List(Vec<AttributeValue>),
    
    /// Set of string values (unordered, no duplicates)
    Set(HashSet<String>),
    
    /// Null value
    Null,
}

impl AttributeValue {
    /// Create a string value
    pub fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }
    
    /// Create a number value
    pub fn number(n: impl Into<f64>) -> Self {
        Self::Number(n.into())
    }
    
    /// Create a boolean value
    pub fn boolean(b: bool) -> Self {
        Self::Boolean(b)
    }
    
    /// Create a list value
    pub fn list(items: Vec<AttributeValue>) -> Self {
        Self::List(items)
    }
    
    /// Create a set value
    pub fn set(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Set(items.into_iter().map(|s| s.into()).collect())
    }
    
    /// Create a null value
    pub fn null() -> Self {
        Self::Null
    }
    
    /// Check if this is a null value
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    
    /// Get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    
    /// Get as number
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    /// Get as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    
    /// Get as list
    pub fn as_list(&self) -> Option<&Vec<AttributeValue>> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }
    
    /// Get as set
    pub fn as_set(&self) -> Option<&HashSet<String>> {
        match self {
            Self::Set(items) => Some(items),
            _ => None,
        }
    }
    
    /// Convert to a display string
    pub fn to_display_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) => n.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::List(items) => {
                let items: Vec<String> = items.iter()
                    .map(|v| v.to_display_string())
                    .collect();
                format!("[{}]", items.join(", "))
            }
            Self::Set(items) => {
                let mut items: Vec<&String> = items.iter().collect();
                items.sort();
                format!("{{{}}}", items.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
            }
            Self::Null => "null".to_string(),
        }
    }
    
    /// Check equality with type coercion
    pub fn equals(&self, other: &AttributeValue) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Set(a), Self::Set(b)) => a == b,
            (Self::Null, Self::Null) => true,
            // Type coercion: string to number
            (Self::String(a), Self::Number(b)) => a.parse::<f64>().ok().map_or(false, |n| (n - b).abs() < f64::EPSILON),
            (Self::Number(a), Self::String(b)) => b.parse::<f64>().ok().map_or(false, |n| (n - a).abs() < f64::EPSILON),
            // String to boolean
            (Self::String(a), Self::Boolean(b)) => {
                a.to_lowercase() == "true" && *b || a.to_lowercase() == "false" && !*b
            }
            (Self::Boolean(a), Self::String(b)) => {
                b.to_lowercase() == "true" && *a || b.to_lowercase() == "false" && !*a
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl Default for AttributeValue {
    fn default() -> Self {
        Self::Null
    }
}

/// An attribute with its category and name
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Unique identifier for this attribute
    pub id: String,
    
    /// Attribute category
    pub category: AttributeCategory,
    
    /// Attribute name
    pub name: String,
    
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    
    /// Expected value type (for validation)
    #[serde(default)]
    pub value_type: AttributeValueType,
    
    /// Whether this attribute is required
    #[serde(default)]
    pub required: bool,
    
    /// Default value if not provided
    #[serde(default)]
    pub default: Option<AttributeValue>,
    
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

/// Expected attribute value type for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AttributeValueType {
    #[default]
    Any,
    String,
    Number,
    Boolean,
    List,
    Set,
}

impl Attribute {
    /// Create a new attribute
    pub fn new(category: AttributeCategory, name: impl Into<String>) -> Self {
        let name = name.into();
        let id = format!("{}.{}", category.prefix(), name);
        Self {
            id,
            category,
            name,
            description: String::new(),
            value_type: AttributeValueType::default(),
            required: false,
            default: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    
    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self.updated_at = Utc::now();
        self
    }
    
    /// Set value type
    pub fn with_value_type(mut self, value_type: AttributeValueType) -> Self {
        self.value_type = value_type;
        self.updated_at = Utc::now();
        self
    }
    
    /// Set required flag
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self.updated_at = Utc::now();
        self
    }
    
    /// Set default value
    pub fn with_default(mut self, default: AttributeValue) -> Self {
        self.default = Some(default);
        self.updated_at = Utc::now();
        self
    }
    
    /// Validate a value against this attribute's type
    pub fn validate(&self, value: &AttributeValue) -> Result<(), AttributeError> {
        match self.value_type {
            AttributeValueType::Any => Ok(()),
            AttributeValueType::String => {
                if matches!(value, AttributeValue::String(_)) {
                    Ok(())
                } else {
                    Err(AttributeError::TypeMismatch {
                        expected: "string".to_string(),
                        actual: value.to_display_string(),
                    })
                }
            }
            AttributeValueType::Number => {
                if matches!(value, AttributeValue::Number(_)) {
                    Ok(())
                } else {
                    Err(AttributeError::TypeMismatch {
                        expected: "number".to_string(),
                        actual: value.to_display_string(),
                    })
                }
            }
            AttributeValueType::Boolean => {
                if matches!(value, AttributeValue::Boolean(_)) {
                    Ok(())
                } else {
                    Err(AttributeError::TypeMismatch {
                        expected: "boolean".to_string(),
                        actual: value.to_display_string(),
                    })
                }
            }
            AttributeValueType::List => {
                if matches!(value, AttributeValue::List(_)) {
                    Ok(())
                } else {
                    Err(AttributeError::TypeMismatch {
                        expected: "list".to_string(),
                        actual: value.to_display_string(),
                    })
                }
            }
            AttributeValueType::Set => {
                if matches!(value, AttributeValue::Set(_)) {
                    Ok(())
                } else {
                    Err(AttributeError::TypeMismatch {
                        expected: "set".to_string(),
                        actual: value.to_display_string(),
                    })
                }
            }
        }
    }
}

/// Attribute collection organized by category
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttributeBag {
    /// Subject attributes
    #[serde(default)]
    pub subject: HashMap<String, AttributeValue>,
    
    /// Resource attributes
    #[serde(default)]
    pub resource: HashMap<String, AttributeValue>,
    
    /// Action attributes
    #[serde(default)]
    pub action: HashMap<String, AttributeValue>,
    
    /// Environment attributes
    #[serde(default)]
    pub environment: HashMap<String, AttributeValue>,
}

impl AttributeBag {
    /// Create a new empty attribute bag
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get a value by category and name
    pub fn get(&self, category: &AttributeCategory, name: &str) -> Option<&AttributeValue> {
        match category {
            AttributeCategory::Subject => self.subject.get(name),
            AttributeCategory::Resource => self.resource.get(name),
            AttributeCategory::Action => self.action.get(name),
            AttributeCategory::Environment => self.environment.get(name),
        }
    }
    
    /// Set a value by category and name
    pub fn set(&mut self, category: AttributeCategory, name: impl Into<String>, value: AttributeValue) {
        let name = name.into();
        match category {
            AttributeCategory::Subject => self.subject.insert(name, value),
            AttributeCategory::Resource => self.resource.insert(name, value),
            AttributeCategory::Action => self.action.insert(name, value),
            AttributeCategory::Environment => self.environment.insert(name, value),
        };
    }
    
    /// Add a subject attribute
    pub fn with_subject(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.subject.insert(name.into(), value);
        self
    }
    
    /// Add a resource attribute
    pub fn with_resource(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.resource.insert(name.into(), value);
        self
    }
    
    /// Add an action attribute
    pub fn with_action(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.action.insert(name.into(), value);
        self
    }
    
    /// Add an environment attribute
    pub fn with_environment(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.environment.insert(name.into(), value);
        self
    }
    
    /// Get all values for a category
    pub fn get_category(&self, category: &AttributeCategory) -> &HashMap<String, AttributeValue> {
        match category {
            AttributeCategory::Subject => &self.subject,
            AttributeCategory::Resource => &self.resource,
            AttributeCategory::Action => &self.action,
            AttributeCategory::Environment => &self.environment,
        }
    }
    
    /// Merge another attribute bag into this one
    pub fn merge(&mut self, other: &AttributeBag) {
        for (k, v) in &other.subject {
            self.subject.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &other.resource {
            self.resource.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &other.action {
            self.action.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &other.environment {
            self.environment.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    
    /// Count total attributes
    pub fn len(&self) -> usize {
        self.subject.len() + self.resource.len() + self.action.len() + self.environment.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Clear all attributes
    pub fn clear(&mut self) {
        self.subject.clear();
        self.resource.clear();
        self.action.clear();
        self.environment.clear();
    }
}

/// Trait for resolving attribute values
#[async_trait::async_trait]
pub trait AttributeResolver: Send + Sync {
    /// Resolve a single attribute value
    async fn resolve(&self, category: &AttributeCategory, name: &str) -> Result<Option<AttributeValue>, AttributeError>;
    
    /// Resolve all attributes for a category
    async fn resolve_category(&self, category: &AttributeCategory) -> Result<HashMap<String, AttributeValue>, AttributeError>;
}

/// In-memory attribute resolver for static attributes
#[derive(Debug, Clone, Default)]
pub struct StaticAttributeResolver {
    attributes: AttributeBag,
}

impl StaticAttributeResolver {
    /// Create a new static resolver
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set an attribute
    pub fn set(&mut self, category: AttributeCategory, name: impl Into<String>, value: AttributeValue) {
        self.attributes.set(category, name, value);
    }
    
    /// Set subject attribute
    pub fn with_subject(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.subject.insert(name.into(), value);
        self
    }
    
    /// Set resource attribute
    pub fn with_resource(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.resource.insert(name.into(), value);
        self
    }
    
    /// Set action attribute
    pub fn with_action(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.action.insert(name.into(), value);
        self
    }
    
    /// Set environment attribute
    pub fn with_environment(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.environment.insert(name.into(), value);
        self
    }
}

#[async_trait::async_trait]
impl AttributeResolver for StaticAttributeResolver {
    async fn resolve(&self, category: &AttributeCategory, name: &str) -> Result<Option<AttributeValue>, AttributeError> {
        Ok(self.attributes.get(category, name).cloned())
    }
    
    async fn resolve_category(&self, category: &AttributeCategory) -> Result<HashMap<String, AttributeValue>, AttributeError> {
        Ok(self.attributes.get_category(category).clone())
    }
}

/// Composite resolver that chains multiple resolvers
pub struct CompositeAttributeResolver {
    resolvers: Vec<Box<dyn AttributeResolver>>,
}

impl CompositeAttributeResolver {
    /// Create a new composite resolver
    pub fn new() -> Self {
        Self { resolvers: Vec::new() }
    }
    
    /// Add a resolver
    pub fn add(mut self, resolver: Box<dyn AttributeResolver>) -> Self {
        self.resolvers.push(resolver);
        self
    }
}

impl Default for CompositeAttributeResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AttributeResolver for CompositeAttributeResolver {
    async fn resolve(&self, category: &AttributeCategory, name: &str) -> Result<Option<AttributeValue>, AttributeError> {
        for resolver in &self.resolvers {
            if let Some(value) = resolver.resolve(category, name).await? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    
    async fn resolve_category(&self, category: &AttributeCategory) -> Result<HashMap<String, AttributeValue>, AttributeError> {
        let mut result = HashMap::new();
        for resolver in &self.resolvers {
            let attrs = resolver.resolve_category(category).await?;
            result.extend(attrs);
        }
        Ok(result)
    }
}

/// Attribute error type
#[derive(Debug, thiserror::Error)]
pub enum AttributeError {
    #[error("Attribute not found: {0}")]
    NotFound(String),
    
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Resolver error: {0}")]
    ResolverError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attribute_category() {
        assert_eq!(AttributeCategory::Subject.prefix(), "subject");
        assert_eq!(AttributeCategory::Resource.prefix(), "resource");
        assert_eq!(AttributeCategory::Action.prefix(), "action");
        assert_eq!(AttributeCategory::Environment.prefix(), "env");
        
        assert_eq!(AttributeCategory::from_str("subject"), Some(AttributeCategory::Subject));
        assert_eq!(AttributeCategory::from_str("sub"), Some(AttributeCategory::Subject));
        assert_eq!(AttributeCategory::from_str("env"), Some(AttributeCategory::Environment));
    }
    
    #[test]
    fn test_attribute_value_string() {
        let v = AttributeValue::string("hello");
        assert_eq!(v.as_string(), Some("hello"));
        assert!(v.as_number().is_none());
        assert!(v.as_boolean().is_none());
    }
    
    #[test]
    fn test_attribute_value_number() {
        let v = AttributeValue::number(42.5);
        assert_eq!(v.as_number(), Some(42.5));
        assert!(v.as_string().is_none());
    }
    
    #[test]
    fn test_attribute_value_boolean() {
        let v = AttributeValue::boolean(true);
        assert_eq!(v.as_boolean(), Some(true));
    }
    
    #[test]
    fn test_attribute_value_list() {
        let v = AttributeValue::list(vec![
            AttributeValue::string("a"),
            AttributeValue::string("b"),
        ]);
        assert_eq!(v.as_list().unwrap().len(), 2);
    }
    
    #[test]
    fn test_attribute_value_set() {
        let v = AttributeValue::set(vec!["a", "b", "c"]);
        assert_eq!(v.as_set().unwrap().len(), 3);
    }
    
    #[test]
    fn test_attribute_value_equals() {
        assert!(AttributeValue::string("hello").equals(&AttributeValue::string("hello")));
        assert!(!AttributeValue::string("hello").equals(&AttributeValue::string("world")));
        
        assert!(AttributeValue::number(42.0).equals(&AttributeValue::number(42.0)));
        assert!(AttributeValue::string("42").equals(&AttributeValue::number(42.0)));
        assert!(AttributeValue::boolean(true).equals(&AttributeValue::string("true")));
    }
    
    #[test]
    fn test_attribute() {
        let attr = Attribute::new(AttributeCategory::Subject, "user_id")
            .with_description("User identifier")
            .with_value_type(AttributeValueType::String)
            .with_required(true);
        
        assert_eq!(attr.id, "subject.user_id");
        assert_eq!(attr.name, "user_id");
        assert!(attr.required);
        
        // Validation
        assert!(attr.validate(&AttributeValue::string("user123")).is_ok());
        assert!(attr.validate(&AttributeValue::number(123)).is_err());
    }
    
    #[test]
    fn test_attribute_bag() {
        let mut bag = AttributeBag::new()
            .with_subject("user_id", AttributeValue::string("user123"))
            .with_subject("role", AttributeValue::string("admin"))
            .with_resource("resource_id", AttributeValue::string("doc456"))
            .with_action("action", AttributeValue::string("read"))
            .with_environment("ip", AttributeValue::string("192.168.1.1"));
        
        assert_eq!(bag.len(), 5);
        
        // Get values
        assert_eq!(
            bag.get(&AttributeCategory::Subject, "user_id"),
            Some(&AttributeValue::string("user123"))
        );
        assert_eq!(
            bag.get(&AttributeCategory::Resource, "resource_id"),
            Some(&AttributeValue::string("doc456"))
        );
    }
    
    #[tokio::test]
    async fn test_static_attribute_resolver() {
        let resolver = StaticAttributeResolver::new()
            .with_subject("user_id", AttributeValue::string("user123"))
            .with_resource("type", AttributeValue::string("document"));
        
        let value = resolver.resolve(&AttributeCategory::Subject, "user_id").await.unwrap();
        assert_eq!(value, Some(AttributeValue::string("user123")));
        
        let value = resolver.resolve(&AttributeCategory::Subject, "missing").await.unwrap();
        assert!(value.is_none());
        
        let attrs = resolver.resolve_category(&AttributeCategory::Resource).await.unwrap();
        assert_eq!(attrs.len(), 1);
    }
    
    #[tokio::test]
    async fn test_composite_attribute_resolver() {
        let r1 = StaticAttributeResolver::new()
            .with_subject("user_id", AttributeValue::string("user123"));
        
        let r2 = StaticAttributeResolver::new()
            .with_subject("role", AttributeValue::string("admin"))
            .with_resource("type", AttributeValue::string("document"));
        
        let composite = CompositeAttributeResolver::new()
            .add(Box::new(r1))
            .add(Box::new(r2));
        
        let value = composite.resolve(&AttributeCategory::Subject, "user_id").await.unwrap();
        assert_eq!(value, Some(AttributeValue::string("user123")));
        
        let value = composite.resolve(&AttributeCategory::Subject, "role").await.unwrap();
        assert_eq!(value, Some(AttributeValue::string("admin")));
    }
}