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

    #[validate(length(min = 1))]
    #[serde(rename = "metricsVersion")]
    pub metrics_version: Option<String>,

    pub service: SubmitDataServiceSchema,

    // There can be any arbitrary properties (used with default chart with parser position 'global')
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[skip_serializing_none]
#[derive(Debug, Validate, Deserialize)]
pub struct SubmitDataServiceSchema {
    pub id: u32,
    #[serde(rename = "customCharts")]
    pub custom_charts: Option<Vec<SubmitDataChartSchema>>,

    // There can be any arbitrary properties (used with default chart with parser position 'plugin')
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct SubmitDataChartSchema {
    #[validate(length(min = 1))]
    #[serde(rename = "chartId")]
    pub chart_id: String,
    pub data: Value,
    // Must not be send by the client. Is set by the server when creating charts.
    #[serde(default)]
    #[serde(skip)]
    pub trusted: bool,
}
