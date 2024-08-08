use actix_web::{web, App, HttpServer};
use data_processor::{legacy_submit_data, submit_data};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis = redis::Client::open("redis://127.0.0.1:6379").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(redis.clone()))
            .service(submit_data)
            .service(legacy_submit_data)
    })
    .bind(("127.0.0.1", 1234))?
    .run()
    .await
}
