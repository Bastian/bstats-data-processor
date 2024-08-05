use serde_json::{json, Value};

use crate::{charts::simple_pie::SimplePie, submit_data_schema::SubmitDataSchema};

use super::Parser;

pub struct BungeecordVersionParser;

impl Parser for BungeecordVersionParser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value> {
        let version = schema
            .extra
            .get("bungeecordVersion")
            .and_then(|v| v.as_str())
            .as_deref()
            .map(parse_bungeecord_version)?;
        Some(json!(SimplePie { value: version }))
    }
}

fn parse_bungeecord_version(bungeecord_version: &str) -> String {
    let split: Vec<&str> = bungeecord_version.split(':').collect();
    if split.len() > 2 {
        split[2].to_string()
    } else {
        bungeecord_version.to_string()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_bungeecord_version() {
        struct TestCase {
            name: String,
            bungeecord_version: String,
            expected: String,
        }

        let test_cases = vec![
            // TODO Collect some real-word examples.
            //  This is just made-up on the spot and not very useful.
            TestCase {
                name: String::from("Made up example"),
                bungeecord_version: String::from("Abc:Xyz:1.2.3"),
                expected: String::from("1.2.3"),
            },
        ];

        for test_case in test_cases {
            let result = parse_bungeecord_version(&test_case.bungeecord_version);
            assert_eq!(
                result, test_case.expected,
                "Wrong result for {}",
                test_case.name
            );
        }
    }
}
