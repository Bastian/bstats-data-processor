use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

fn deserialize_number_or_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(serde_json::Number),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::Number(n) => Ok(n.to_string()),
    }
}

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct SimplePie {
    #[validate(length(min = 1))]
    // Needed for "coreCount" (and probably others) from the NameInRequestParser
    #[serde(deserialize_with = "deserialize_number_or_string")]
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_number_as_string() {
        let pie: SimplePie = serde_json::from_value(json!({"value": 42})).unwrap();
        assert_eq!(pie.value, "42");
    }
}
