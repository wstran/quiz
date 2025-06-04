use actix_web::{web, Responder};
use serde_json::json;
use crate::libraries::response_ok_builder;

pub const PATH: &str = "/api/health";

pub async fn handler() -> impl Responder {
    response_ok_builder().json(json!({ "message": "I'm fine!" }))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource(PATH)
            .route(web::post().to(handler))
    );
}