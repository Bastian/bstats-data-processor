use std::collections::HashMap;

use serde_json::{Value, json};

use crate::{models::charts::drilldown_pie::DrilldownPie, submit_data_schema::SubmitDataSchema};

use super::Parser;

pub struct HytaleVersionParser;

impl Parser for HytaleVersionParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        let version = schema.extra.get("hytaleVersion").and_then(|v| v.as_str())?;
        let year_month = extract_year_month(version);
        Some(json!(DrilldownPie {
            values: HashMap::from([(year_month, HashMap::from([(version.to_string(), 1)]))])
        }))
    }
}

/// Extracts the year and month from a Hytale version string.
///
/// Hytale versions have the format "YYYY.MM.DD-commithash" (e.g., "2026.02.17-255364b8e").
/// This function returns "YYYY.MM".
fn extract_year_month(version: &str) -> String {
    let parts: Vec<&str> = version.splitn(3, '.').collect();
    if parts.len() >= 2 {
        // Take just the month digits (strip any trailing content like "17-255364b8e")
        let month = parts[1]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if !month.is_empty() {
            return format!("{}.{}", parts[0], month);
        }
    }
    version.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_year_month() {
        assert_eq!(extract_year_month("2026.02.17-255364b8e"), "2026.02");
        assert_eq!(extract_year_month("2026.03.01-abc123"), "2026.03");
        assert_eq!(extract_year_month("2026.12.25-deadbeef"), "2026.12");
        assert_eq!(extract_year_month("2026.02"), "2026.02");
        assert_eq!(extract_year_month("garbage"), "garbage");
        assert_eq!(extract_year_month(""), "");
    }

    #[test]
    fn test_extract_year_month_does_not_panic() {
        assert!(std::panic::catch_unwind(|| extract_year_month("...")).is_ok());
        assert!(std::panic::catch_unwind(|| extract_year_month("a.b.c")).is_ok());
    }
}
