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

use actix_web::{post, web, HttpRequest, Responder};
use legacy_data_submission::LegacySubmitDataSchema;
use submit_data_schema::SubmitDataSchema;
use util::redis::RedisClusterPool;

#[post("/{software_url}")]
async fn submit_data(
    request: HttpRequest,
    redis_pool: web::Data<RedisClusterPool>,
    software_url: web::Path<String>,
    data: web::Json<SubmitDataSchema>,
) -> actix_web::Result<impl Responder> {
    data_submission::handle_data_submission(
        &request,
        &redis_pool,
        software_url.as_str(),
        &data.0,
        false,
    )
    .await
}

#[post("/legacy/{software_url}")]
async fn legacy_submit_data(
    request: HttpRequest,
    redis_pool: web::Data<RedisClusterPool>,
    software_url: web::Path<String>,
    data: web::Json<LegacySubmitDataSchema>,
) -> actix_web::Result<impl Responder> {
    legacy_data_submission::handle_legacy_data_submission(
        &request,
        &redis_pool,
        software_url.as_str(),
        data.0,
    )
    .await
}
