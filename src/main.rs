use actix_web::{web, App, HttpServer};
use data_processor::{legacy_submit_data, submit_data, util::redis::get_redis_cluster_pool};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap();

    let pool = get_redis_cluster_pool().await;

    let mut http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(submit_data)
            .service(legacy_submit_data)
    });

    if let Ok(workers) = std::env::var("WORKERS") {
        http_server = http_server.workers(workers.parse().unwrap());
    }

    http_server.bind((host, port))?.run().await
}
