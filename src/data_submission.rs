use std::collections::HashMap;
use std::str::FromStr;

use crate::chart_buffer::{ChartBuffer, ChartOps};
use crate::chart_updater::update_chart;
use crate::models::charts;
use crate::models::service;
use crate::models::service::Service;
use crate::models::software;
use crate::models::software::Software;
use crate::parser;
use crate::parser::ParserInput;
use crate::ratelimits::is_ratelimited;
use crate::submit_data_schema::SubmitDataChartSchema;
use crate::submit_data_schema::SubmitDataSchema;
use crate::util::date::date_to_tms2000;
use crate::util::geo_ip;
use crate::util::ip_parser;
use crate::util::redis::RedisClusterPool;
use actix_web::{HttpRequest, HttpResponse, error, web};
use deadpool_redis::cluster::Connection;
use serde_json::Value;

/// Everything about a request that does not depend on the service being
/// processed. Determined once so that the global service does not repeat the
/// GeoIP lookup, the IP parsing or the software lookup.
struct RequestContext<'a> {
    software: Software,
    /// The URL the request was sent to. Used for ratelimit keys, so that every
    /// service of a request shares the same namespace.
    software_url: &'a str,
    tms2000: i64,
    ip: String,
    country_iso: Option<String>,
    country_name: Option<String>,
    data: &'a SubmitDataSchema,
}

pub async fn handle_data_submission(
    request: &HttpRequest,
    redis_pool: &web::Data<RedisClusterPool>,
    chart_buffer: &ChartBuffer,
    software_url: &str,
    data: &SubmitDataSchema,
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

    if service.global {
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

    let context = RequestContext {
        software,
        software_url,
        tms2000,
        ip,
        country_iso,
        country_name,
        data,
    };

    let mut ops = ChartOps::new();

    collect_service_charts(
        &context,
        &service,
        &context.data.service.extra,
        context.data.service.custom_charts.as_deref(),
        &mut ops,
        con,
    )
    .await?;

    // Global services are "fake" submissions that every request contributes to.
    // Ratelimits ensure that this only happens once per server and interval.
    if let Some(global_service_id) = context.software.global_plugin {
        collect_global_service_charts(&context, global_service_id, &mut ops, con).await?;
    }

    // The response does not wait for the chart writes: the deltas of both
    // services are summed in the shared buffer and flushed in the background.
    chart_buffer.merge(ops);

    Ok(HttpResponse::Ok().finish())
}

/// Adds the chart updates of a single service to `pipeline`.
///
/// `service_extra` and `custom_charts` are passed separately because the global
/// service is processed with the same request but without any service specific
/// data.
async fn collect_service_charts(
    context: &RequestContext<'_>,
    service: &Service,
    service_extra: &HashMap<String, Value>,
    custom_charts: Option<&[SubmitDataChartSchema]>,
    ops: &mut ChartOps,
    con: &mut Connection,
) -> actix_web::Result<()> {
    let parser_input = ParserInput {
        global: &context.data.extra,
        service: service_extra,
    };

    let default_charts: Vec<_> = context
        .software
        .default_charts
        .iter()
        .filter_map(|template| {
            parser::get_parser(template, context.country_name.clone()).and_then(|parser| {
                Some(SubmitDataChartSchema {
                    chart_id: template.id.clone(),
                    data: parser.parse(&parser_input)?,
                    trusted: true,
                })
            })
        })
        .collect();

    let resolved_charts = charts::find_by_ids(con, &service.charts)
        .await
        .map_err(error::ErrorInternalServerError)?;

    let chart_data = default_charts
        .iter()
        .chain(custom_charts.unwrap_or_default().iter());

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
            context.tms2000,
            context.country_iso.as_deref(),
            ops,
        );
    }

    Ok(())
}

/// Adds the chart updates of the software's global service to `pipeline`.
///
/// A broken global service is never a normal case, so its errors are propagated
/// like any other. Only a misconfigured `global_plugin` is skipped.
async fn collect_global_service_charts(
    context: &RequestContext<'_>,
    global_service_id: u32,
    ops: &mut ChartOps,
    con: &mut Connection,
) -> actix_web::Result<()> {
    let ratelimited = is_ratelimited(
        con,
        context.software_url,
        context.software.max_requests_per_ip,
        &context.data.server_uuid,
        &context.ip,
        global_service_id,
        context.tms2000,
    )
    .await
    .map_err(error::ErrorInternalServerError)?;

    if ratelimited {
        // The server already contributed to the global service in this interval
        return Ok(());
    }

    let global_service = match service::find_by_id(con, global_service_id)
        .await
        .map_err(error::ErrorInternalServerError)?
    {
        Some(s) => s,
        // The software points at a global service that does not exist
        None => return Ok(()),
    };

    if global_service.software_id != context.software.id {
        return Ok(());
    }

    // The global service has no service specific data of its own
    let no_service_extra: HashMap<String, Value> = HashMap::new();
    collect_service_charts(context, &global_service, &no_service_extra, None, ops, con).await
}
