use crate::date_util::date_to_tms2000;
use crate::ratelimits::is_ratelimited;
use crate::software;
use crate::submit_data_schema::SubmitDataSchema;
use crate::util::ip_parser;
use actix_web::{error, web, HttpRequest, Responder};

pub async fn handle_data_submission(
    request: HttpRequest,
    redis: web::Data<redis::Client>,
    software_url: web::Path<String>,
    data: web::Json<SubmitDataSchema>,
    _is_global_service: bool,
) -> actix_web::Result<impl Responder> {
    let mut con = redis
        .get_connection_manager()
        .await
        .map_err(error::ErrorInternalServerError)?;

    let software = software::find_by_url(&mut con, software_url.as_str()).await;

    if software.is_err() {
        return Err(error::ErrorInternalServerError(software.err().unwrap()));
    }

    let software = match software {
        Ok(None) => return Err(error::ErrorNotFound("Software not found")),
        Err(e) => return Err(error::ErrorInternalServerError(e)),
        Ok(Some(s)) => s,
    };

    let tms2000 = date_to_tms2000(chrono::Utc::now());

    let ip = ip_parser::get_ip(request)?;

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

    // TODO: This does not make sense, it's only here for testing
    Ok(web::Json(software))
}
