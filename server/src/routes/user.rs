use actix_web::{http::Method, web, HttpMessage, HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use mongodb::bson::{doc, oid::ObjectId, Document};
use futures::TryStreamExt;
use mongodb::Database;
use crate::libraries::{method_not_allowed, response_internal_server_error, response_ok_builder};

pub const PATH: &str = "/api/user";

#[derive(Clone, Serialize, Deserialize)]
pub struct SlideQuizQuestion {
    question_type: String,
    theme: String,
    time_limit: Option<u32>,
    points: Option<u32>,
    answer_options: String,
    image_reveal: String,
    image_path: String,
    question: String,
    answers: Option<Vec<String>>,
    correct_answers: Option<Vec<bool>>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SlideQuizTrueOrFalse {
    question_type: String,
    theme: String,
    time_limit: Option<u32>,
    points: Option<u32>,
    image_reveal: String,
    image_path: String,
    question: String,
    answers: Option<Vec<String>>,
    correct_answers: Option<Vec<bool>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Slide {
    #[serde(rename = "question")]
    Question(SlideQuizQuestion),
    #[serde(rename = "true_or_false")]
    TrueOrFalse(SlideQuizTrueOrFalse),
}

#[derive(Serialize, Deserialize)]
pub struct QuizCreation {
    title: String,
    description: Option<String>,
    slides: Vec<Slide>,
}

pub async fn handler(
    req: HttpRequest,
    db: web::Data<Database>,
) -> impl Responder {
    if let Some(response_error) = crate::middlewares::jwt::middleware(&req, &db).await {
        return response_error;
    }

    if let Some(user_data) = req.extensions().get::<crate::middlewares::jwt::RequestUser>() {
        let user_id = ObjectId::parse_str(&user_data.user.user_id).unwrap();

        match *req.method() {
            Method::GET => {
                let cursor = db.collection::<Document>("users").find_one(doc! {
                    "_id": user_id.clone(),
                }).projection(doc! {
                    "_id": 0,
                    "name": 1,
                    "auth_method": 1
                }).await;

                let result = cursor.unwrap().unwrap();

                response_ok_builder().json(json!({
                    "name": result.get("name"),
                    "auth_method": result.get("auth_method"),
                }))
            }
            _ => method_not_allowed(),
        }
    } else {
        response_internal_server_error()
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource(PATH).route(web::get().to(handler)));
}