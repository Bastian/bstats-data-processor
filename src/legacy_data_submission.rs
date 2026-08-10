use actix_web::{HttpRequest, HttpResponse, error, web};

use crate::{
    data_submission::handle_data_submission,
    legacy_submit_data_schema::LegacySubmitDataSchema,
    models::service,
    submit_data_schema::{SubmitDataSchema, SubmitDataServiceSchema},
    util::redis::RedisClusterPool,
};

pub async fn handle_legacy_data_submission(
    request: &HttpRequest,
    redis_pool: &web::Data<RedisClusterPool>,
    software_url: &str,
    data: LegacySubmitDataSchema,
) -> actix_web::Result<HttpResponse> {
    let mut con = match redis_pool.get().await {
        Ok(con) => con,
        Err(e) => return Err(error::ErrorInternalServerError(e)),
    };

    for plugin in data.plugins {
        let plugin_id = match plugin.id {
            Some(id) => id,
            None => {
                // Find the plugin by name
                let plugin_name = match plugin.plugin_name {
                    Some(name) => name,
                    None => continue,
                };

                match service::find_by_software_url_and_name(&mut con, software_url, &plugin_name)
                    .await
                {
                    Ok(None) => continue,
                    Ok(Some(plugin)) => plugin.id,
                    Err(e) => return Err(error::ErrorInternalServerError(e)),
                }
            }
        };

        let _ = handle_data_submission(
            request,
            redis_pool,
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
            Some(&mut con),
        )
        .await;
    }

    Ok(HttpResponse::Ok().finish())
}
