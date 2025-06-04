use actix_web::{HttpResponse, HttpResponseBuilder};
use serde_json::json;

pub mod mongodb;

pub mod redis;

pub fn response_ok_builder() -> HttpResponseBuilder {
    HttpResponse::Ok()
}

pub fn method_not_allowed() -> HttpResponse {
    HttpResponse::MethodNotAllowed().json(json!({"error": "Method not allowed."}))
}

pub fn response_bad_request() -> HttpResponse {
    HttpResponse::BadRequest().json(json!({ "message": "Bad request." }))
}

pub fn response_not_found() -> HttpResponse {
    HttpResponse::NotFound().json(json!({ "message": "Not found." }))
}

pub fn response_forbidden() -> HttpResponse {
    HttpResponse::Forbidden().json(json!({ "message": "Forbidden." }))
}

pub fn response_internal_server_error() -> HttpResponse {
    HttpResponse::InternalServerError().json(json!({ "message": "Internal server error." }))
}