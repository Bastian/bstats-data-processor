use actix_web::{web, App, HttpServer};
use data_processor::{handle_data_submission, index};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let redis = redis::Client::open("redis://127.0.0.1:6379").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(redis.clone()))
            .service(index)
            .service(handle_data_submission)
    })
    .bind(("127.0.0.1", 1234))?
    .run()
    .await
}
