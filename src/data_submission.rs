use std::collections::HashMap;
use std::str::FromStr;

use crate::date_util::date_to_tms2000;
use crate::parser;
use crate::ratelimits::is_ratelimited;
use crate::service;
use crate::software;
use crate::submit_data_schema::SubmitDataChartSchema;
use crate::submit_data_schema::SubmitDataSchema;
use crate::submit_data_schema::SubmitDataServiceSchema;
use crate::util::geo_ip;
use crate::util::ip_parser;
use actix_web::{error, web, HttpRequest, Responder};

pub async fn handle_data_submission(
    request: HttpRequest,
    redis: web::Data<redis::Client>,
    software_url: web::Path<String>,
    data: SubmitDataSchema,
    is_global_service: bool,
) -> actix_web::Result<impl Responder> {
    let mut con = redis
        .get_connection_manager()
        .await
        .map_err(error::ErrorInternalServerError)?;

    let software = match software::find_by_url(&mut con, software_url.as_str()).await {
        Ok(None) => return Err(error::ErrorNotFound("Software not found")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(Some(s)) => s,
    };

    let tms2000 = date_to_tms2000(chrono::Utc::now());

    let ip = ip_parser::get_ip(&request)?;

    let ratelimit = is_ratelimited(
        &mut con,
        software_url.as_str(),
        software.max_requests_per_ip,
        &data.server_uuid,
        &ip,
        data.service.id,
        tms2000,
    )
    .await;

    if ratelimit.is_err() {
        return Err(error::ErrorTooManyRequests("Too many requests"));
    }

    // Global services are "fake" requests. We just recursively call this method
    // again, but with the data for the global service. Ratelimits ensure that
    // this only happens once per server.
    if !is_global_service && software.global_plugin.is_some() {
        let global_plugin = software.global_plugin.unwrap();
        let global_plugin = service::find_by_id(&mut con, global_plugin).await;
        let global_plugin = match global_plugin {
            Ok(o) => o,
            Err(e) => return Err(error::ErrorInternalServerError(e)),
        };

        if let Some(global_plugin) = global_plugin {
            let result = Box::pin(handle_data_submission(
                request,
                redis,
                software_url,
                SubmitDataSchema {
                    server_uuid: data.server_uuid.clone(),
                    metrics_version: data.metrics_version.clone(),
                    extra: data.extra.clone(),
                    service: SubmitDataServiceSchema {
                        id: global_plugin.id,
                        custom_charts: None,
                        extra: HashMap::new(),
                    },
                },
                true,
            ))
            .await;
            match result {
                Ok(_) => {}
                Err(e) => {
                    if e.as_response_error().status_code() != 429 {
                        // Too many requests can be ignored
                    } else {
                        // TODO Use proper logging framework
                        println!("Error: {:?}", e);
                    }
                }
            }
        }
    }

    let service = match service::find_by_id(&mut con, data.service.id).await {
        Ok(None) => return Err(error::ErrorNotFound("Service not found")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(Some(s)) => s,
    };

    if service.global && !is_global_service {
        return Err(error::ErrorBadRequest(
            "You must not send data for global services",
        ));
    }

    let country = match FromStr::from_str(&ip) {
        Ok(ip) => geo_ip::get_country(ip),
        _ => None,
    };

    let country_name = country.and_then(|(_, c)| c);

    let default_charts: Vec<_> = software
        .default_charts
        .iter()
        .filter_map(|template| {
            parser::get_parser(template, country_name.clone()).and_then(|parser| {
                Some(SubmitDataChartSchema {
                    chart_id: template.id.clone(),
                    data: parser.parse(&data)?,
                    trusted: true,
                })
            })
        })
        .collect();

    let custom_charts = data.service.custom_charts.unwrap_or(Vec::new());
    let charts = default_charts.iter().chain(custom_charts.iter());

    for _chart in charts {
        // TODO: Implement this
    }

    // TODO: This does not make sense, it's only here for testing
    Ok(web::Json(software))
}
