use actix_web::{error, post, web, HttpRequest, HttpResponse};

use crate::legacy_data_submission;
use crate::legacy_submit_data_schema::LegacySubmitDataSchema;
use crate::util::redis::RedisClusterPool;
use crate::validation::has_blocked_words;

#[post("/legacy/{software_url}")]
pub async fn legacy_submit_data(
    request: HttpRequest,
    redis_pool: web::Data<RedisClusterPool>,
    software_url: web::Path<String>,
    body: web::Bytes,
) -> actix_web::Result<HttpResponse> {
    // Convert bytes to string for word checking
    let json_str = std::str::from_utf8(&body)
        .map_err(|_| error::ErrorBadRequest("Invalid UTF-8 in request body"))?;

    // Check for blocked words on raw JSON before expensive deserialization
    if has_blocked_words(json_str) {
        // Block silently
        return Ok(HttpResponse::Ok().finish());
    }

    let data: LegacySubmitDataSchema = serde_json::from_str(json_str)
        .map_err(|e| error::ErrorBadRequest(format!("Invalid JSON: {}", e)))?;

    legacy_data_submission::handle_legacy_data_submission(
        &request,
        &redis_pool,
        software_url.as_str(),
        data,
    )
    .await
}
