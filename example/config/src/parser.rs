use std::collections::HashMap;

use crate::Value;

/// Error type for parsing failures.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// Parse a simple `key = value` config format.
///
/// Supported value types:
///   - Strings: `key = "value"`
///   - Integers: `key = 42`
///   - Booleans: `key = true` or `key = false`
///
/// Lines starting with `#` are comments. Blank lines are ignored.
pub fn parse(input: &str) -> Result<HashMap<String, Value>, ParseError> {
    let mut map = HashMap::new();

    for (idx, line) in input.lines().enumerate() {
        let line = line.trim();

        // Skip blanks and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, raw_value) = line.split_once('=').ok_or_else(|| ParseError {
            line: idx + 1,
            message: format!("expected `key = value`, got: {line}"),
        })?;

        let key = key.trim().to_string();
        let raw_value = raw_value.trim();

        let value = parse_value(raw_value).ok_or_else(|| ParseError {
            line: idx + 1,
            message: format!("unrecognized value: {raw_value}"),
        })?;

        map.insert(key, value);
    }

    Ok(map)
}

fn parse_value(raw: &str) -> Option<Value> {
    // Try string
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        let inner = &raw[1..raw.len() - 1];
        return Some(Value::Str(inner.to_string()));
    }

    // Try boolean
    match raw {
        "true" => return Some(Value::Bool(true)),
        "false" => return Some(Value::Bool(false)),
        _ => {}
    }

    // Try integer
    if let Ok(n) = raw.parse::<i64>() {
        return Some(Value::Int(n));
    }

    None
}
