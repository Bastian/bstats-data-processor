use serde_json::Value;

use crate::{models::charts::chart::DefaultChartTemplate, submit_data_schema::SubmitDataSchema};

pub mod auth_mode;
pub mod bukkit_minecraft_version;
pub mod bukkit_server_software;
pub mod bungeecord_version;
pub mod hytale_version;
pub mod java_version;
pub mod name_in_request;
pub mod os;
pub mod predefined_value;
pub mod semver;

pub trait Parser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value>;
}

pub fn get_parser(
    template: &DefaultChartTemplate,
    country_name: Option<String>,
) -> Option<Box<dyn Parser>> {
    if let Some(predefined_value) = template.request_parser.get("predefinedValue") {
        return Some(Box::new(predefined_value::PredefinedValueParser {
            value: predefined_value.clone(),
            country_name,
        }));
    }

    let hardcoded_parser = template
        .request_parser
        .get("useHardcodedParser")
        .and_then(|v| v.as_str());

    match hardcoded_parser {
        Some("os") => return Some(Box::new(os::OsParser)),
        Some("javaVersion") => return Some(Box::new(java_version::JavaVersionParser)),
        Some("bukkitMinecraftVersion") => {
            return Some(Box::new(
                bukkit_minecraft_version::BukkitMinecraftVersionParser,
            ));
        }
        Some("bukkitServerSoftware") => {
            return Some(Box::new(bukkit_server_software::BukkitServerSoftwareParser));
        }
        Some("bungeecordVersion") => {
            return Some(Box::new(bungeecord_version::BungeecordVersionParser));
        }
        Some("hytaleAuthMode") => {
            return Some(Box::new(auth_mode::HytaleAuthModeParser));
        }
        Some("hytaleVersion") => {
            return Some(Box::new(hytale_version::HytaleVersionParser));
        }
        Some("phpVersion") => {
            return Some(Box::new(semver::SemVerParser {
                field_name: "phpVersion".to_string(),
                label_prefix: "PHP".to_string(),
            }));
        }
        _ => (),
    }

    if let Some(name_in_request) = template.request_parser.get("nameInRequest") {
        return Some(Box::new(name_in_request::NameInRequestParser {
            name_in_request: name_in_request.as_str()?.to_string(),
            position: template
                .request_parser
                .get("position")?
                .as_str()?
                .to_string(),
            data_type: template
                .request_parser
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            true_value: template
                .request_parser
                .get("trueValue")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            false_value: template
                .request_parser
                .get("falseValue")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }));
    }

    None
}
