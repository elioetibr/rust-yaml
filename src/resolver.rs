//! YAML resolver for tag resolution and implicit typing

use std::collections::HashMap;

/// Trait for YAML resolvers that handle tag resolution
pub trait Resolver {
    /// Resolve a tag for implicit typing
    fn resolve_tag(&self, value: &str, implicit: bool) -> Option<String>;

    /// Add an implicit resolver pattern
    fn add_implicit_resolver(&mut self, tag: String, pattern: String);

    /// Reset the resolver state
    fn reset(&mut self);
}

/// Basic resolver with standard YAML 1.2 implicit typing
#[derive(Debug)]
pub struct BasicResolver {
    implicit_resolvers: HashMap<String, String>,
}

impl BasicResolver {
    /// Create a new resolver with standard YAML 1.2 resolvers
    pub fn new() -> Self {
        let mut resolver = Self {
            implicit_resolvers: HashMap::new(),
        };

        // Add standard YAML 1.2 implicit resolvers
        resolver.add_standard_resolvers();
        resolver
    }

    fn add_standard_resolvers(&mut self) {
        // Boolean values
        self.implicit_resolvers
            .insert("true".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("True".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("TRUE".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("false".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("False".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("FALSE".to_string(), "tag:yaml.org,2002:bool".to_string());

        // Null values
        self.implicit_resolvers
            .insert("null".to_string(), "tag:yaml.org,2002:null".to_string());
        self.implicit_resolvers
            .insert("Null".to_string(), "tag:yaml.org,2002:null".to_string());
        self.implicit_resolvers
            .insert("NULL".to_string(), "tag:yaml.org,2002:null".to_string());
        self.implicit_resolvers
            .insert("~".to_string(), "tag:yaml.org,2002:null".to_string());
    }

    /// Check if a string represents an integer
    pub fn is_int(&self, value: &str) -> bool {
        value.parse::<i64>().is_ok()
    }

    /// Check if a string represents a float
    pub fn is_float(&self, value: &str) -> bool {
        value.parse::<f64>().is_ok()
    }
}

impl Default for BasicResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver for BasicResolver {
    fn resolve_tag(&self, value: &str, implicit: bool) -> Option<String> {
        if !implicit {
            return None;
        }

        // Check explicit mappings first
        if let Some(tag) = self.implicit_resolvers.get(value) {
            return Some(tag.clone());
        }

        // Check numeric types
        if self.is_int(value) {
            return Some("tag:yaml.org,2002:int".to_string());
        }

        if self.is_float(value) {
            return Some("tag:yaml.org,2002:float".to_string());
        }

        // Default to string
        Some("tag:yaml.org,2002:str".to_string())
    }

    fn add_implicit_resolver(&mut self, tag: String, pattern: String) {
        self.implicit_resolvers.insert(pattern, tag);
    }

    fn reset(&mut self) {
        // Keep the standard resolvers, don't clear them
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = BasicResolver::new();
        assert!(!resolver.implicit_resolvers.is_empty());
    }

    #[test]
    fn test_boolean_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("true", true),
            Some("tag:yaml.org,2002:bool".to_string())
        );
        assert_eq!(
            resolver.resolve_tag("false", true),
            Some("tag:yaml.org,2002:bool".to_string())
        );
    }

    #[test]
    fn test_null_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("null", true),
            Some("tag:yaml.org,2002:null".to_string())
        );
        assert_eq!(
            resolver.resolve_tag("~", true),
            Some("tag:yaml.org,2002:null".to_string())
        );
    }

    #[test]
    fn test_numeric_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("42", true),
            Some("tag:yaml.org,2002:int".to_string())
        );
        assert_eq!(
            resolver.resolve_tag("3.14", true),
            Some("tag:yaml.org,2002:float".to_string())
        );
    }

    #[test]
    fn test_string_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("hello", true),
            Some("tag:yaml.org,2002:str".to_string())
        );
    }

    #[test]
    fn test_explicit_tag_resolution() {
        let resolver = BasicResolver::new();

        // When not implicit, should return None
        assert_eq!(resolver.resolve_tag("true", false), None);
    }

    #[test]
    fn test_custom_resolver() {
        let mut resolver = BasicResolver::new();

        resolver.add_implicit_resolver(
            "tag:example.com,2002:custom".to_string(),
            "CUSTOM".to_string(),
        );

        assert_eq!(
            resolver.resolve_tag("CUSTOM", true),
            Some("tag:example.com,2002:custom".to_string())
        );
    }
}
