use serde_json::{Value, json};

use crate::{models::charts::simple_pie::SimplePie, submit_data_schema::SubmitDataSchema};

use super::Parser;

pub struct HytaleAuthModeParser;

impl Parser for HytaleAuthModeParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        let auth_mode = schema.extra.get("authMode").and_then(|v| v.as_str())?;
        let normalized = normalize_auth_mode(auth_mode);
        Some(json!(SimplePie { value: normalized }))
    }
}

fn normalize_auth_mode(auth_mode: &str) -> String {
    match auth_mode.to_uppercase().as_str() {
        "AUTHENTICATED" => "Authenticated".to_string(),
        "OFFLINE" => "Offline".to_string(),
        "INSECURE" => "Insecure".to_string(),
        _ => auth_mode.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_auth_mode() {
        assert_eq!(normalize_auth_mode("AUTHENTICATED"), "Authenticated");
        assert_eq!(normalize_auth_mode("OFFLINE"), "Offline");
        assert_eq!(normalize_auth_mode("INSECURE"), "Insecure");
        assert_eq!(normalize_auth_mode("authenticated"), "Authenticated");
        assert_eq!(normalize_auth_mode("offline"), "Offline");
        assert_eq!(normalize_auth_mode("insecure"), "Insecure");
        assert_eq!(normalize_auth_mode("unknown"), "unknown");
    }
}
