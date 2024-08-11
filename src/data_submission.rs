use std::collections::HashMap;
use std::str::FromStr;

use crate::chart_updater::update_chart;
use crate::charts;
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
use crate::util::redis::RedisClusterPool;
use actix_web::{error, web, HttpRequest, Responder};
use once_cell::sync::Lazy;

pub async fn handle_data_submission(
    request: &HttpRequest,
    redis_pool: &web::Data<RedisClusterPool>,
    software_url: &str,
    data: &SubmitDataSchema,
    is_global_service: bool,
) -> actix_web::Result<impl Responder> {
    if has_blocked_words(&data) {
        // Block silently
        return Ok("");
    }

    let mut con = match redis_pool.get().await {
        Ok(con) => con,
        Err(e) => return Err(error::ErrorInternalServerError(e)),
    };

    let software = match software::find_by_url(&mut con, software_url).await {
        Ok(None) => return Err(error::ErrorNotFound("Software not found")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(Some(s)) => s,
    };

    let tms2000 = date_to_tms2000(chrono::Utc::now());

    let ip = ip_parser::get_ip(&request)?;

    let ratelimit = is_ratelimited(
        &mut con,
        software_url,
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
                redis_pool,
                software_url,
                &SubmitDataSchema {
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

    let (country_iso, country_name) = match country {
        Some((iso, country)) => (Some(iso), country),
        None => (None, None),
    };

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

    let custom_charts = data.service.custom_charts.clone().unwrap_or(Vec::new());
    let chart_data = default_charts.iter().chain(custom_charts.iter());

    let resolved_charts: std::collections::HashMap<u64, Option<charts::Chart>> =
        charts::find_by_ids(&mut con, service.charts).await.unwrap();

    let mut pipeline = redis::pipe();

    for chart_data in chart_data {
        let service_chart: &charts::Chart = match resolved_charts
            .values()
            .filter_map(|c| c.as_ref())
            .find(|c| c.id_custom == chart_data.chart_id)
        {
            Some(c) => c,
            None => continue,
        };

        if !chart_data.trusted && service_chart.default {
            // The service is trying to trick us and sent a default chart as a custom chart
            continue;
        }

        let _ = update_chart(
            service_chart,
            chart_data,
            tms2000,
            country_iso.as_deref(),
            &mut pipeline,
            &mut con,
        )
        .await;
    }

    pipeline
        .query_async(&mut con)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok("")
}

static WORD_BLOCKLIST: Lazy<Vec<String>> = Lazy::new(|| {
    let word_blocklist = std::env::var("WORD_BLOCKLIST").unwrap_or(String::from("[]"));
    match serde_json::from_str(&word_blocklist) {
        Ok(blocklist) => blocklist,
        Err(_) => Vec::new(),
    }
});

fn has_blocked_words(data: &SubmitDataSchema) -> bool {
    let mut blocked = false;
    for word in WORD_BLOCKLIST.iter() {
        // TODO: This is a very inefficient way to check for blocked words
        if serde_json::to_string(&data).unwrap().contains(word) {
            blocked = true;
            break;
        }
    }
    blocked
}
