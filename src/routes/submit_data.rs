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

    /// Helper function to snapshot the changes made to the Redis database by a
    /// data submission request with the given payload.
    ///
    /// These snapshots allow us to see if a change to the codebase alters the
    /// data stored in Redis.
    async fn snapshot_state(name: &str, payload: serde_json::Value) {
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
            .set_payload(payload.to_string())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);

        let body = test::read_body(resp).await;
        assert_eq!(body, "");

        let redis_state_after =
            redis_dump::capture(&mut test_environment.redis_connection().await).await;

        let diff = redis_dump::diff(&redis_state_before, &redis_state_after);
        insta::with_settings!({
            description => "Redis state changes after data submission",
            snapshot_path => "__snapshots__",
            prepend_module_to_snapshot => false,
        }, {
            insta::assert_yaml_snapshot!(name, diff);
        });
    }

    #[actix_web::test]
    async fn processes_normal_request() {
        snapshot_state(
            "normal_request",
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
            }),
        )
        .await;
    }

    #[actix_web::test]
    async fn ignores_unknown_top_level_fields() {
        snapshot_state(
            "unknown_fields",
            json!({
                "unknownField1": "some value",
                "unknownField2": {
                    "nestedUnknownField": 123
                },
                "service": {
                    "id": 27400,
                    "unknownServiceField": 456,
                },
                "serverUUID": "7386d410-f71e-447c-b356-ee809c7db098",
                "metricsVersion": "3.0.2",
            }),
        )
        .await;
    }

    #[actix_web::test]
    async fn clamps_too_high_player_count() {
        snapshot_state(
            "too_high_player_count",
            json!({
                "playerAmount": 9999999,
                "service": {
                    "id": 27400,
                },
                "serverUUID": "7386d410-f71e-447c-b356-ee809c7db098",
                "metricsVersion": "3.0.2"
            }),
        )
        .await;
    }

    #[actix_web::test]
    async fn processes_bar_charts() {
        snapshot_state(
            "bar_charts",
            json!({
                "service": {
                    "id": 27400,
                    "customCharts": [
                        {
                            "chartId": "custom_advanced_bar_chart",
                            "data": {
                                "values": {
                                    "Feature A": [0, 1],
                                    "Feature B": [1, 0]
                                }
                            }
                        },
                        {
                            "chartId": "custom_simple_bar_chart",
                            "data": {
                                "values": {
                                    "Feature A": [1]
                                }
                            }
                        }
                    ]
                },
                "serverUUID": "7386d410-f71e-447c-b356-ee809c7db098",
                "metricsVersion": "3.0.2"
            }),
        )
        .await;
    }

    #[actix_web::test]
    async fn ignores_custom_charts_for_default_charts() {
        // For the backend, default charts are almost identical to custom
        // charts. Malicious clients could try to exploit this by sending
        // default chart data as custom chart data. These should be ignored.
        snapshot_state(
            "default_charts_in_custom_charts",
            json!({
                "service": {
                    "id": 27400,
                    "customCharts": [
                        {
                            "chartId": "servers",
                            "data": {
                                "value": 456
                            }
                        },
                        {
                            "chartId": "players",
                            "data": {
                                "value": 123
                            }
                        }
                    ]
                },
                "serverUUID": "7386d410-f71e-447c-b356-ee809c7db098",
                "metricsVersion": "3.0.2"
            }),
        )
        .await;
    }

    #[actix_web::test]
    async fn rejects_service_for_wrong_platform() {
        // Service 27400 belongs to software ID 1 (Bukkit), so submitting
        // it to the bungeecord endpoint should be rejected.
        let test_environment = TestEnvironment::with_data().await;
        let redis_pool = test_environment.redis_pool();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(redis_pool.clone()))
                .service(submit_data),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/bungeecord")
            .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 1111))
            .insert_header(ContentType::json())
            .set_payload(
                json!({
                    "service": {
                        "id": 27400,
                    },
                    "serverUUID": "7386d410-f71e-447c-b356-ee809c7db098",
                    "metricsVersion": "3.0.2"
                })
                .to_string(),
            )
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);

        let body = test::read_body(resp).await;
        assert_eq!(body, "Service does not belong to this software");
    }

    #[actix_web::test]
    async fn accepts_fabric_and_updates_global_rollup() {
        snapshot_state(
            "fabric_global_rollup",
            json!({
                "playerAmount": 12,
                "onlineMode": 1,
                "minecraftVersion": "1.21.6",
                "fabricVersion": "0.16.14",
                "pluginVersion": "1.0.0-SNAPSHOT",
                "javaVersion": "21.0.2",
                "osName": "Linux",
                "osArch": "amd64",
                "osVersion": "6.8.0",
                "coreCount": 8,
                "service": {
                    "id": 27401,
                },
                "serverUUID": "7386d410-f71e-447c-b356-ee809c7db099",
                "metricsVersion": "3.0.2"
            }),
        )
        .await;
    }
}
