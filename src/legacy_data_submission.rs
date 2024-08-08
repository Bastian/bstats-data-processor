use std::collections::HashMap;

use actix_web::{error, web, HttpRequest, Responder};
use serde::{Deserialize, Serialize};

use serde_json::Value;
use serde_with::skip_serializing_none;
use validator::Validate;

use crate::{
    data_submission::handle_data_submission,
    service,
    submit_data_schema::{SubmitDataChartSchema, SubmitDataSchema, SubmitDataServiceSchema},
};

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

pub async fn handle_legacy_data_submission(
    con: &mut redis::aio::ConnectionManager,
    request: &HttpRequest,
    redis: &web::Data<redis::Client>,
    software_url: &str,
    data: LegacySubmitDataSchema,
) -> actix_web::Result<impl Responder> {
    for plugin in data.plugins {
        let plugin_id = match plugin.id {
            Some(id) => id,
            None => {
                // Find the plugin by name
                let plugin_name = match plugin.plugin_name {
                    Some(name) => name,
                    None => continue,
                };

                match service::find_by_software_url_and_name(con, software_url, &plugin_name).await
                {
                    Ok(None) => continue,
                    Ok(Some(plugin)) => plugin.id,
                    Err(e) => return Err(error::ErrorInternalServerError(e)),
                }
            }
        };

        let _ = handle_data_submission(
            con,
            &request,
            &redis,
            software_url,
            &SubmitDataSchema {
                server_uuid: data.server_uuid.clone(),
                metrics_version: None,
                extra: data.extra.clone(),
                service: SubmitDataServiceSchema {
                    id: plugin_id,
                    custom_charts: plugin.custom_charts,
                    extra: plugin.extra,
                },
            },
            false,
        )
        .await;
    }

    Ok("")
}
