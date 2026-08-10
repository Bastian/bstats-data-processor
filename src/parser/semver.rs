use std::collections::HashMap;

use serde_json::{Value, json};

use crate::{models::charts::drilldown_pie::DrilldownPie, parser::ParserInput};

use super::Parser;

pub struct SemVerParser {
    pub field_name: String,
    pub label_prefix: String,
}

impl Parser for SemVerParser {
    fn parse(&self, input: &ParserInput) -> Option<Value> {
        let version = input
            .global
            .get(&self.field_name)
            .and_then(|v| v.as_str())?;
        let major_version = get_major_version(version);
        Some(json!(DrilldownPie {
            values: HashMap::from([(
                format!("{} {}", self.label_prefix, major_version),
                HashMap::from([(version.to_string(), 1),])
            ),])
        }))
    }
}

fn get_major_version(version: &str) -> &str {
    version.split('.').next().unwrap_or(version)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_major_version() {
        assert_eq!(get_major_version("8.2.0"), "8");
        assert_eq!(get_major_version("8.3.14"), "8");
        assert_eq!(get_major_version("7.4.33"), "7");
        assert_eq!(get_major_version("10.0.1"), "10");
        assert_eq!(get_major_version("8"), "8");
        assert_eq!(get_major_version(""), "");

        // Should not panic on garbage input
        assert!(std::panic::catch_unwind(|| get_major_version("garbage")).is_ok());
        assert!(std::panic::catch_unwind(|| get_major_version("..")).is_ok());
    }
}
