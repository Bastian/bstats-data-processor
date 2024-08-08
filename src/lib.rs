pub mod chart_updater;
pub mod charts;
pub mod data_submission;
pub mod date_util;
pub mod legacy_data_submission;
pub mod parser;
pub mod ratelimits;
pub mod service;
pub mod software;
pub mod submit_data_schema;
pub mod util;

use actix_web::{error, post, web, HttpRequest, Responder};
use legacy_data_submission::LegacySubmitDataSchema;
use submit_data_schema::SubmitDataSchema;

#[post("/{software_url}")]
async fn submit_data(
    request: HttpRequest,
    redis: web::Data<redis::Client>,
    software_url: web::Path<String>,
    data: web::Json<SubmitDataSchema>,
) -> actix_web::Result<impl Responder> {
    let mut con: redis::aio::ConnectionManager = redis
        .get_connection_manager()
        .await
        .map_err(error::ErrorInternalServerError)?;

    data_submission::handle_data_submission(
        &mut con,
        &request,
        &redis,
        software_url.as_str(),
        &data.0,
        false,
    )
    .await
}

#[post("/legacy/{software_url}")]
async fn legacy_submit_data(
    request: HttpRequest,
    redis: web::Data<redis::Client>,
    software_url: web::Path<String>,
    data: web::Json<LegacySubmitDataSchema>,
) -> actix_web::Result<impl Responder> {
    let mut con: redis::aio::ConnectionManager = redis
        .get_connection_manager()
        .await
        .map_err(error::ErrorInternalServerError)?;

    legacy_data_submission::handle_legacy_data_submission(
        &mut con,
        &request,
        &redis,
        software_url.as_str(),
        data.0,
    )
    .await
}
