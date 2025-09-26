pub mod cache;
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

use std::collections::HashSet;

use actix_web::{error, post, web, HttpRequest, HttpResponse};
use legacy_data_submission::LegacySubmitDataSchema;
use once_cell::sync::Lazy;
use submit_data_schema::SubmitDataSchema;
use util::redis::RedisClusterPool;

#[post("/{software_url}")]
async fn submit_data(
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

    let data: SubmitDataSchema = serde_json::from_str(json_str)
        .map_err(|e| error::ErrorBadRequest(format!("Invalid JSON: {}", e)))?;

    data_submission::handle_data_submission(
        &request,
        &redis_pool,
        software_url.as_str(),
        &data,
        false,
        None,
    )
    .await
}

#[post("/legacy/{software_url}")]
async fn legacy_submit_data(
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

static WORD_BLOCKLIST: Lazy<HashSet<String>> = Lazy::new(|| {
    let word_blocklist = std::env::var("WORD_BLOCKLIST").unwrap_or(String::from("[]"));
    let words: Vec<String> = serde_json::from_str(&word_blocklist).unwrap_or_default();
    words.into_iter().map(|w| w.to_lowercase()).collect()
});

pub fn has_blocked_words(str: &str) -> bool {
    if WORD_BLOCKLIST.is_empty() {
        return false;
    }

    let json_lower = str.to_lowercase();
    WORD_BLOCKLIST.iter().any(|word| json_lower.contains(word))
}
