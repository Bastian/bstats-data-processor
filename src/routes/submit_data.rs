use actix_web::{HttpRequest, HttpResponse, error, post, web};

use crate::data_submission;
use crate::submit_data_schema::SubmitDataSchema;
use crate::util::redis::RedisClusterPool;
use crate::validation::has_blocked_words;

#[post("/{software_url}")]
pub async fn submit_data(
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

#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use crate::test_support::{redis_dump, test_environment::TestEnvironment};
    use actix_web::{App, http::header::ContentType, test, web};
    use serde_json::json;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[actix_web::test]
    async fn test_submit_data() {
        let test_environment = TestEnvironment::with_data().await;
        let redis_pool = test_environment.redis_pool();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(redis_pool.clone()))
                .service(submit_data),
        )
        .await;

        let redis_state_before =
            redis_dump::capture(&mut test_environment.redis_connection().await).await;

        let req = test::TestRequest::post()
            .uri("/bukkit")
            .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 1111))
            .insert_header(ContentType::json())
            .set_payload(
                json!({
                    "playerAmount": 25,
                    "onlineMode": 1,
                    "bukkitVersion": "1.21-38-1f5db50 (MC: 1.21)",
                    "bukkitName": "Paper",
                    "javaVersion": "21.0.2",
                    "osName": "Windows 11",
                    "osArch": "amd64",
                    "osVersion": "10.0",
                    "coreCount": 24,
                    "service": {
                        "pluginVersion": "1.0.0-SNAPSHOT",
                        "id": 27400,
                        "customCharts": [
                            {
                                "chartId": "custom_simple_pie_chart",
                                "data": {
                                    "value": "Simple Pie Value"
                                }
                            }
                        ]
                    },
                    "serverUUID": "7386d410-f71e-447c-b356-ee809c7db098",
                    "metricsVersion": "3.0.2"
                })
                .to_string(),
            )
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);

        let body = test::read_body(resp).await;
        assert_eq!(body, "");

        let redis_state_after =
            redis_dump::capture(&mut test_environment.redis_connection().await).await;

        let diff = redis_dump::diff(&redis_state_before, &redis_state_after);
        insta::with_settings!({description => "Redis state changes after data submission"}, {
            insta::assert_yaml_snapshot!(diff);
        });
    }
}
