use std::collections::HashMap;
use std::str::FromStr;

use crate::chart_updater::update_chart;
use crate::models::charts;
use crate::models::service;
use crate::models::software;
use crate::parser;
use crate::ratelimits::is_ratelimited;
use crate::submit_data_schema::SubmitDataChartSchema;
use crate::submit_data_schema::SubmitDataSchema;
use crate::submit_data_schema::SubmitDataServiceSchema;
use crate::util::date::date_to_tms2000;
use crate::util::geo_ip;
use crate::util::ip_parser;
use crate::util::redis::RedisClusterPool;
use actix_web::{HttpRequest, HttpResponse, error, web};
use deadpool_redis::cluster::Connection;

pub async fn handle_data_submission(
    request: &HttpRequest,
    redis_pool: &web::Data<RedisClusterPool>,
    software_url: &str,
    data: &SubmitDataSchema,
    is_global_service: bool,
    connection: Option<&mut Connection>,
) -> actix_web::Result<HttpResponse> {
    let mut owned_con;
    let con = match connection {
        Some(c) => c,
        None => {
            owned_con = match redis_pool.get().await {
                Ok(con) => con,
                Err(e) => return Err(error::ErrorInternalServerError(e)),
            };
            &mut owned_con
        }
    };

    let software = match software::find_by_url(con, software_url).await {
        Ok(None) => return Err(error::ErrorNotFound("Software not found")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(Some(s)) => s,
    };

    // Use a fixed time in tests for consistent snapshots
    // TODO: We might want to use a more sophisticated way to control time in
    //  tests, so that we can also test time-advancing scenarios.
    #[cfg(feature = "integration-tests")]
    let tms2000 = date_to_tms2000(
        chrono::DateTime::parse_from_rfc3339("2069-07-21T00:37:33Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    );

    #[cfg(not(feature = "integration-tests"))]
    let tms2000 = date_to_tms2000(chrono::Utc::now());

    let ip = ip_parser::get_ip(request)?;

    let ratelimit = is_ratelimited(
        con,
        software_url,
        software.max_requests_per_ip,
        &data.server_uuid,
        &ip,
        data.service.id,
        tms2000,
    )
    .await;

    match ratelimit {
        Ok(true) => return Err(error::ErrorTooManyRequests("Too many requests")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(false) => {}
    }

    // Global services are "fake" requests. We just recursively call this method
    // again, but with the data for the global service. Ratelimits ensure that
    // this only happens once per server.
    if !is_global_service && software.global_plugin.is_some() {
        let global_plugin = software.global_plugin.unwrap();
        let global_plugin = service::find_by_id(con, global_plugin).await;
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
                Some(con),
            ))
            .await;
            match result {
                Ok(_) => {}
                Err(e) => {
                    if e.as_response_error().status_code() == 429 {
                        // Too many requests can be ignored
                    } else {
                        // TODO Use proper logging framework
                        println!("Error: {:?}", e);
                    }
                }
            }
        }
    }

    let service = match service::find_by_id(con, data.service.id).await {
        Ok(None) => return Err(error::ErrorNotFound("Service not found")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(Some(s)) => s,
    };

    if service.software_id != software.id {
        return Err(error::ErrorBadRequest(
            "Service does not belong to this software",
        ));
    }

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
                    data: parser.parse(data)?,
                    trusted: true,
                })
            })
        })
        .collect();

    let custom_charts = data.service.custom_charts.clone().unwrap_or_default();
    let chart_data = default_charts.iter().chain(custom_charts.iter());

    let resolved_charts: std::collections::HashMap<u64, Option<charts::Chart>> =
        charts::find_by_ids(con, service.charts).await.unwrap();

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
            con,
        )
        .await;
    }

    pipeline
        .query_async::<()>(con)
        .await
        .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}
