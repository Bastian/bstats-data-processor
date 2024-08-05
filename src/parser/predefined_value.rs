use serde_json::{json, Value};

use crate::submit_data_schema::SubmitDataSchema;

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
            return Some(json!(country_name));
        }
        return Some(self.value.clone());
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_os() {
        let parser = PredefinedValueParser {
            value: json!("%country.name%"),
            country_name: Some(String::from("Germany")),
        };

        let result = parser.parse();
        assert_eq!(result.unwrap().as_str(), Some("Germany"));

        let parser = PredefinedValueParser {
            value: json!({"key": "value"}),
            country_name: None,
        };

        let result = parser.parse();
        assert_eq!(
            result.unwrap().as_object(),
            Some(json!({"key": "value"}).as_object().unwrap())
        );
    }
}
