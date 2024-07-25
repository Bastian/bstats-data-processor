pub mod charts;
pub mod parser;
pub mod software;
pub mod submit_data_schema;

use actix_web::{error, get, post, web, Responder};
use charts::simple_pie::SimplePie;
use software::find_by_url;
use submit_data_schema::SubmitDataSchema;

#[get("/")]
async fn index() -> impl Responder {
    web::Json(SimplePie {
        value: String::from("Test"),
    })
}

#[post("/{software_url}")]
async fn handle_data_submission(
    redis: web::Data<redis::Client>,
    software_url: web::Path<String>,
    _data: web::Json<SubmitDataSchema>,
) -> actix_web::Result<impl Responder> {
    let mut con = redis
        .get_connection_manager()
        .await
        .map_err(error::ErrorInternalServerError)?;

    let software = find_by_url(&mut con, software_url.as_str()).await;

    // TODO: This does not make sense, it's only here for testing
    match software {
        Ok(Some(s)) => Ok(web::Json(s)),
        Ok(None) => Err(error::ErrorNotFound("Software not found")),
        Err(e) => Err(error::ErrorInternalServerError(e)),
    }
}
