use actix_web::{get, post, web, App, HttpServer, Responder};
use data_processor::{charts::simple_pie::SimplePie, submit_data_schema::SubmitDataSchema};

#[get("/")]
async fn index() -> impl Responder {
    web::Json(SimplePie {
        value: String::from("Test"),
    })
}

#[post("/{software_url}")]
async fn handle_data_submission(
    _software_url: web::Path<String>,
    _data: web::Json<SubmitDataSchema>,
) -> impl Responder {
    web::Json(SimplePie {
        value: String::from("Test"),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(handle_data_submission))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::ContentType, test, App};
    use serde_json::json;

    use super::*;

    #[actix_web::test]
    async fn test_submit_data() {
        let app = test::init_service(App::new().service(handle_data_submission)).await;
        let req = test::TestRequest::post()
            .uri("/bukkit")
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
                        "id": 1234,
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
    }

    #[actix_web::test]
    async fn test_index_get() {
        let app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_index_post() {
        let app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::post().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
    }
}
