use serde_json::{json, Value};

use crate::{charts::simple_pie::SimplePie, submit_data_schema::SubmitDataSchema};

use super::Parser;

/// Parser that extracts values from the request
pub struct NameInRequestParser {
    /// The name of the field in the request
    pub name_in_request: String,
    /// Position in the request: "global" or "plugin"
    pub position: String,
    /// Optional data type to interpret the value
    pub data_type: Option<String>,
    /// Optional custom name for true values
    pub true_value: Option<String>,
    /// Optional custom name for false values
    pub false_value: Option<String>,
}

impl Parser for NameInRequestParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        let raw_value = if self.position == "global" {
            schema.extra.get(&self.name_in_request).cloned()
        } else if self.position == "plugin" {
            schema.service.extra.get(&self.name_in_request).cloned()
        } else {
            None
        }?;

        // Different data types might require different handling
        match self.data_type.as_deref() {
            Some("boolean") => {
                // Convert to boolean
                let is_truthy = match &raw_value {
                    Value::Bool(b) => *b,
                    Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
                    Value::String(s) => !s.is_empty() && s != "0" && s.to_lowercase() != "false",
                    _ => true,
                };

                // Boolean values might have custom true/false representations
                let string_value = if is_truthy {
                    self.true_value
                        .as_ref()
                        .unwrap_or(&"true".to_string())
                        .clone()
                } else {
                    self.false_value
                        .as_ref()
                        .unwrap_or(&"false".to_string())
                        .clone()
                };

                Some(json!(SimplePie {
                    value: string_value
                }))
            }
            Some("number") => Some(json!({"value": raw_value})),
            // Unknown type or string
            _ => Some(json!({"value": raw_value})),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn get_schema() -> SubmitDataSchema {
        SubmitDataSchema {
            server_uuid: "test-server-uuid".to_string(),
            metrics_version: None,
            service: crate::submit_data_schema::SubmitDataServiceSchema {
                id: 1,
                custom_charts: None,
                extra: {
                    let mut map = HashMap::new();
                    map.insert("pluginStringField".to_string(), json!("myPluginValue"));
                    map.insert("pluginNumberField".to_string(), json!(123));
                    map.insert("pluginBooleanField".to_string(), json!(false));
                    map.insert("pluginField".to_string(), json!({"plugin": "value"}));
                    map
                },
            },
            extra: {
                let mut map = HashMap::new();
                map.insert("globalStringField".to_string(), json!("myGlobalValue"));
                map.insert("globalNumberField".to_string(), json!(42));
                map.insert("globalBooleanField".to_string(), json!(true));
                map.insert("globalField".to_string(), json!({"global": "value"}));
                map
            },
        }
    }

    fn parser(
        field: &str,
        position: &str,
        data_type: Option<&str>,
        true_val: Option<&str>,
        false_val: Option<&str>,
    ) -> NameInRequestParser {
        NameInRequestParser {
            name_in_request: field.to_string(),
            position: position.to_string(),
            data_type: data_type.map(|s| s.to_string()),
            true_value: true_val.map(|s| s.to_string()),
            false_value: false_val.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_basic_field_extraction() {
        let schema = get_schema();

        // Test string fields
        assert_eq!(
            parser("globalStringField", "global", None, None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": "myGlobalValue"}).as_object().unwrap())
        );
        assert_eq!(
            parser("pluginStringField", "plugin", None, None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": "myPluginValue"}).as_object().unwrap())
        );

        // Test number fields
        assert_eq!(
            parser("globalNumberField", "global", Some("number"), None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": 42}).as_object().unwrap())
        );
        assert_eq!(
            parser("pluginNumberField", "plugin", Some("number"), None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": 123}).as_object().unwrap())
        );
    }

    #[test]
    fn test_boolean_conversion() {
        let schema = get_schema();

        // Boolean fields with default true/false
        assert_eq!(
            parser("globalBooleanField", "global", Some("boolean"), None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": "true"}).as_object().unwrap())
        );
        assert_eq!(
            parser("pluginBooleanField", "plugin", Some("boolean"), None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": "false"}).as_object().unwrap())
        );

        // Boolean with custom name
        assert_eq!(
            parser(
                "globalBooleanField",
                "global",
                Some("boolean"),
                Some("enabled"),
                Some("disabled")
            )
            .parse(&schema)
            .unwrap()
            .as_object(),
            Some(json!({"value": "enabled"}).as_object().unwrap())
        );
    }

    #[test]
    fn test_boolean_from_different_types() {
        let mut schema = get_schema();

        // Various truthy/falsy values
        let test_cases = [
            ("numberTruthy", json!(1), "true"),
            ("numberFalsy", json!(0), "false"),
            ("stringTruthy", json!("yes"), "true"),
            ("stringEmpty", json!(""), "false"),
            ("stringZero", json!("0"), "false"),
            ("stringFalse", json!("FALSE"), "false"),
        ];

        for (field_name, value, expected) in test_cases {
            schema.extra.insert(field_name.to_string(), value);
            assert_eq!(
                parser(field_name, "global", Some("boolean"), None, None)
                    .parse(&schema)
                    .unwrap()
                    .as_object(),
                Some(json!({"value": expected}).as_object().unwrap()),
                "Failed for field: {}",
                field_name
            );
        }
    }

    #[test]
    fn test_edge_cases() {
        let schema = get_schema();

        // Non-existent field
        assert!(parser("nonExistent", "global", None, None, None)
            .parse(&schema)
            .is_none());

        // Invalid position
        assert!(parser("globalStringField", "invalid", None, None, None)
            .parse(&schema)
            .is_none());

        // Complex values
        assert_eq!(
            parser("globalField", "global", None, None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": {"global": "value"}}).as_object().unwrap())
        );

        // Complex value as boolean
        assert_eq!(
            parser("globalField", "global", Some("boolean"), None, None)
                .parse(&schema)
                .unwrap()
                .as_object(),
            Some(json!({"value": "true"}).as_object().unwrap())
        );
    }
}
