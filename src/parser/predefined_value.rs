use serde_json::{json, Value};

use crate::{
    models::charts::{simple_pie::SimplePie, single_line_chart::SingleLineChart},
    submit_data_schema::SubmitDataSchema,
};

use super::Parser;

pub struct PredefinedValueParser {
    pub value: Value,
    pub country_name: Option<String>,
}

impl Parser for PredefinedValueParser {
    fn parse(&self, _schema: &SubmitDataSchema) -> Option<Value> {
        self.parse()
    }
}

impl PredefinedValueParser {
    fn parse(&self) -> Option<Value> {
        if self
            .value
            .as_str()
            .map(|s| s.eq("%country.name%"))
            .unwrap_or(false)
        {
            let country_name = self.country_name.as_ref()?;
            return Some(json!(SimplePie {
                value: country_name.to_string()
            }));
        }

        if let Some(num) = self.value.as_i64() {
            let safe_num = i32::try_from(num).unwrap_or({
                // Clamp to i32 range if conversion fails
                if num > i32::MAX as i64 {
                    i32::MAX
                } else {
                    i32::MIN
                }
            });
            return Some(json!(SingleLineChart { value: safe_num }));
        }

        // For other types, keep the value as is
        Some(json!({"value": self.value.clone()}))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn parser(value: Value, country_name: Option<String>) -> PredefinedValueParser {
        PredefinedValueParser {
            value,
            country_name,
        }
    }

    #[test]
    fn test_parse_predefined_value() {
        assert_eq!(
            parser(json!("%country.name%"), Some(String::from("Germany")))
                .parse()
                .unwrap()
                .as_object(),
            Some(json!({"value": "Germany"}).as_object().unwrap())
        );

        assert_eq!(
            parser(json!(42), None).parse().unwrap().as_object(),
            Some(json!({"value": 42}).as_object().unwrap())
        );

        assert_eq!(
            parser(json!({"key": "value"}), None)
                .parse()
                .unwrap()
                .as_object(),
            Some(json!({"value": {"key": "value"}}).as_object().unwrap())
        );

        // Test clamping
        assert_eq!(
            parser(json!(9999999999i64), None)
                .parse()
                .unwrap()
                .as_object(),
            Some(json!({"value": i32::MAX}).as_object().unwrap())
        );
        assert_eq!(
            parser(json!(-9999999999i64), None)
                .parse()
                .unwrap()
                .as_object(),
            Some(json!({"value": i32::MIN}).as_object().unwrap())
        );
    }
}
