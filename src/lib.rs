pub mod charts;
pub mod data_submission;
pub mod date_util;
pub mod parser;
pub mod ratelimits;
pub mod software;
pub mod submit_data_schema;
pub mod util;

use actix_web::{get, post, web, HttpRequest, Responder};
use charts::simple_pie::SimplePie;
use submit_data_schema::SubmitDataSchema;

#[get("/")]
async fn index() -> impl Responder {
    web::Json(SimplePie {
        value: String::from("Test"),
    })
}

#[post("/{software_url}")]
async fn submit_data(
    request: HttpRequest,
    redis: web::Data<redis::Client>,
    software_url: web::Path<String>,
    data: web::Json<SubmitDataSchema>,
) -> actix_web::Result<impl Responder> {
    data_submission::handle_data_submission(request, redis, software_url, data, false).await
}
