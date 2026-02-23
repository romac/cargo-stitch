mod parser;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A configuration value that can be a string, integer, or boolean.
#[derive(Debug, Clone)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Holds a set of key-value configuration entries.
#[derive(Debug, Clone)]
pub struct Config {
    entries: HashMap<String, Value>,
}

impl Config {
    /// Load configuration from a file at the given path.
    pub fn load(path: &Path) -> Config {
        let content = fs::read_to_string(path).unwrap();
        let entries = parser::parse(&content).unwrap();
        Config { entries }
    }

    /// Create an empty configuration.
    pub fn empty() -> Config {
        Config {
            entries: HashMap::new(),
        }
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key)
    }

    /// Get a string value, panicking if the key is missing or not a string.
    pub fn get_str(&self, key: &str) -> &str {
        self.get(key).unwrap().as_str().unwrap()
    }

    /// Get an integer value, panicking if the key is missing or not an int.
    pub fn get_int(&self, key: &str) -> i64 {
        self.get(key).unwrap().as_int().unwrap()
    }

    /// Get a boolean value, panicking if the key is missing or not a bool.
    pub fn get_bool(&self, key: &str) -> bool {
        self.get(key).unwrap().as_bool().unwrap()
    }

    /// Insert a key-value pair into the configuration.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.entries.insert(key.into(), value);
    }

    /// Return the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Merge another config into this one. Existing keys are overwritten.
    pub fn merge(&mut self, other: Config) {
        for (k, v) in other.entries {
            self.entries.insert(k, v);
        }
    }
}
