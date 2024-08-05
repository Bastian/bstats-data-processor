use std::collections::HashMap;

use serde_json::{json, Value};

use crate::{charts::drilldown_pie::DrilldownPie, submit_data_schema::SubmitDataSchema};

use super::Parser;

pub struct JavaVersionParser;

impl Parser for JavaVersionParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        let java_version = schema.extra.get("javaVersion").and_then(|v| v.as_str())?;
        let major_version = get_java_major_version(java_version);
        Some(json!(DrilldownPie {
            values: HashMap::from([(
                format!("Java {}", major_version),
                HashMap::from([(java_version.to_string(), 1),])
            ),])
        }))
    }
}

fn get_java_major_version(java_version: &str) -> &str {
    // Java versions post Java 9 have the format "$MAJOR.$MINOR.$SECURITY.$PATCH"
    // Java versions pre Java 9 have the format "1.$MAJOR.$MINOR_$SECURITY"
    if java_version.starts_with("1.") {
        return java_version.split('.').nth(1).unwrap();
    }

    java_version
        .split('.')
        .next()
        .unwrap()
        .split('-') // Get rid of the -ea, -internal, etc. suffix
        .next()
        .unwrap()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_java_major_version() {
        assert_eq!(get_java_major_version("1.8.0_101"), "8");
        assert_eq!(get_java_major_version("9.0.1"), "9");
        assert_eq!(get_java_major_version("11.0.1"), "11");
        assert_eq!(get_java_major_version("21.0.1-internal"), "21");
        assert_eq!(get_java_major_version("21.0.4-ea"), "21");
        assert_eq!(get_java_major_version("21"), "21");
        assert_eq!(get_java_major_version("21-ea"), "21");
        assert_eq!(get_java_major_version("22.0.1"), "22");

        // We do not want garbage data to cause a panic. Garbage in, garbage out is fine though.
        assert!(std::panic::catch_unwind(|| get_java_major_version("garbage data")).is_ok());
        assert!(std::panic::catch_unwind(|| get_java_major_version("..")).is_ok());
        assert!(std::panic::catch_unwind(|| get_java_major_version("")).is_ok());
        assert!(std::panic::catch_unwind(|| get_java_major_version("1.")).is_ok());
        assert!(std::panic::catch_unwind(|| get_java_major_version("9.")).is_ok());
    }
}
