use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use actix_web::{http::header::ContentType, test, web, App};
use data_processor::submit_data;
use serde_json::{json, Value};

use crate::helper::test_environment::TestEnvironment;

#[actix_web::test]
async fn test_submit_data() {
    let test_environment = TestEnvironment::with_data().await;

    let redis = test_environment.redis_client();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(redis.clone()))
            .service(submit_data),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/bukkit")
        .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 1111))
        .insert_header(ContentType::json())
        .set_payload(
            json!({
                "playerAmount": 0,
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
                    "id": 3,
                    "customCharts": [
                    {
                        "chartId": "chart_id",
                        "data": {
                        "value": "My value"
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
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Bukkit / Spigot");
}
