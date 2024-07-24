use crate::{charts::simple_pie::SimplePie, submit_data_schema::SubmitDataSchema};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{json, Value};

use super::Parser;

pub struct BukkitMinecraftVersionParser;

impl Parser for BukkitMinecraftVersionParser {
    fn parse(&self, schema: SubmitDataSchema) -> Option<Value> {
        let version = parse_bukkit_minecraft_version(
            schema
                .extra
                .get("bukkitVersion")
                .and_then(|v| v.as_str())
                .as_deref(),
        )?;
        Some(json!(SimplePie { value: version }))
    }
}

static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"MC: ([\d\\.]+)").unwrap());

fn parse_bukkit_minecraft_version(bukkit_version: Option<&str>) -> Option<String> {
    bukkit_version.map(|bukkit_version| {
        if bukkit_version.contains("MC:") {
            RE.captures(bukkit_version)
                .and_then(|captures| captures.get(1).map(|version| version.as_str()))
                .unwrap_or(bukkit_version)
                .to_string()
        } else {
            // If it does not contain "MC: ", it's from a legacy bStats Metrics class
            // Legacy Metrics classes did the extracting of the version number themselves
            // and sent it directly. Newer versions send the full result of Bukkit#getVersion()
            bukkit_version.to_string()
        }
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_bukkit_minecraft_version() {
        struct TestCase {
            name: String,
            bukkit_version: String,
            expected: String,
        }

        let test_cases = vec![
            TestCase {
                name: String::from("Paper 1.21"),
                bukkit_version: String::from("1.21-38-1f5db50 (MC: 1.21)"),
                expected: String::from("1.21"),
            },
            TestCase {
                name: String::from("Paper 1.20"),
                bukkit_version: String::from("git-Paper-196 (MC: 1.20.1)"),
                expected: String::from("1.20.1"),
            },
            TestCase {
                name: String::from("Spigot 1.21"),
                bukkit_version: String::from("4226-Spigot-146439e-2889b3a (MC: 1.21)"),
                expected: String::from("1.21"),
            },
            TestCase {
                name: String::from("\"CraftBukkit\" 1.21"),
                bukkit_version: String::from("4226-Bukkit-2889b3a (MC: 1.21)"),
                expected: String::from("1.21"),
            },
            TestCase {
                name: String::from("Legacy Metrics Class"),
                bukkit_version: String::from("1.8"),
                expected: String::from("1.8"),
            },
        ];

        for test_case in test_cases {
            let result = parse_bukkit_minecraft_version(Some(&test_case.bukkit_version));

            assert_eq!(
                result.unwrap(),
                test_case.expected,
                "Wrong result for {}",
                test_case.name
            );
        }
    }
}
