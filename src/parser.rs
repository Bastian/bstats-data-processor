use serde_json::Value;

use crate::{charts::chart::DefaultChartTemplate, submit_data_schema::SubmitDataSchema};

pub mod bukkit_minecraft_version;
pub mod bukkit_server_software;
pub mod bungeecord_version;
pub mod java_version;
pub mod os;

pub trait Parser {
    fn parse(&self, schema: &SubmitDataSchema) -> Option<Value>;
}

pub fn get_parser(template: &DefaultChartTemplate) -> Option<Box<dyn Parser>> {
    if let Some(_predefined_value) = template.request_parser.get("predefinedValue") {
        return None; // TODO
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
            ))
        }
        Some("bukkitServerSoftware") => {
            return Some(Box::new(bukkit_server_software::BukkitServerSoftwareParser))
        }
        Some("bungeecordVersion") => {
            return Some(Box::new(bungeecord_version::BungeecordVersionParser))
        }
        _ => (),
    }

    if let Some(_name_in_request) = template.request_parser.get("nameInRequest") {
        return None; // TODO
    }

    return None;
}
