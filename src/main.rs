use actix_web::{web, App, HttpServer};
use data_processor::{legacy_submit_data, submit_data};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis_url = match std::env::var("REDIS_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Please set the REDIS_URL environment variable");
            std::process::exit(1);
        }
    };
    let redis = redis::Client::open(redis_url).unwrap();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(redis.clone()))
            .service(submit_data)
            .service(legacy_submit_data)
    })
    .bind((host, port))?
    .run()
    .await
}
