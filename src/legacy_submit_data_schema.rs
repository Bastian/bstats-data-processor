use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use validator::Validate;

use crate::submit_data_schema::SubmitDataChartSchema;

#[skip_serializing_none]
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct LegacySubmitDataSchema {
    #[validate(length(min = 1))]
    #[serde(rename = "serverUUID")]
    pub server_uuid: String,

    // In 1.x Metrics classes, one plugin sent the data for all plugins on the
    // same server in a single request.
    pub plugins: Vec<LegacySubmitDataServiceSchema>,

    // There can be any arbitrary properties (used with default chart with parser position 'global')
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[skip_serializing_none]
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct LegacySubmitDataServiceSchema {
    // In older Metrics classes, the id was optional and instead the name was sent
    pub id: Option<u32>,

    #[serde(rename = "pluginName")]
    pub plugin_name: Option<String>,

    #[serde(rename = "customCharts")]
    pub custom_charts: Option<Vec<SubmitDataChartSchema>>,

    // There can be any arbitrary properties (used with default chart with parser position 'plugin')
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
