use serde_json::Value;

use crate::submit_data_schema::SubmitDataSchema;

pub mod bukkit_minecraft_version;
pub mod bukkit_server_software;
pub mod bungeecord_version;
pub mod java_version;
pub mod os;

pub trait Parser {
    fn parse(&self, schema: SubmitDataSchema) -> Option<Value>;
}
