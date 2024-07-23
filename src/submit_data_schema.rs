use std::collections::HashMap;

use serde::Deserialize;

use serde_json::Value;
use serde_with::skip_serializing_none;
use validator::Validate;

#[skip_serializing_none]
#[derive(Debug, Validate, Deserialize)]
pub struct SubmitDataSchema {
    #[validate(length(min = 1))]
    #[serde(rename = "serverUUID")]
    pub server_uuid: String,

    /* TODO
         @ValidateNested()
         @Type(() => SubmitDataServiceDto)
         service: SubmitDataServiceDto;
    */
    #[validate(length(min = 1))]
    #[serde(rename = "osName")]
    pub os_name: Option<String>,

    #[validate(length(min = 1))]
    #[serde(rename = "osVersion")]
    pub os_version: Option<String>,

    #[validate(length(min = 1))]
    #[serde(rename = "javaVersion")]
    pub java_version: Option<String>,

    #[validate(length(min = 1))]
    #[serde(rename = "bukkitVersion")]
    pub bukkit_version: Option<String>,

    #[validate(length(min = 1))]
    #[serde(rename = "bukkitName")]
    pub bukkit_name: Option<String>,

    #[validate(length(min = 1))]
    #[serde(rename = "bungeecordVersion")]
    pub bungeecord_version: Option<String>,

    // There can be any arbitrary properties (used with default chart with parser position 'global')
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct Chart {
    #[validate(length(min = 1))]
    #[serde(rename = "chartId")]
    pub chart_id: String,
    pub data: Value,
    // Must not be send by the client. Is set by the server when creating charts.
    #[serde(default)]
    #[serde(skip)]
    pub trusted: bool,
}
