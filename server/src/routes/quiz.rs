mod quiz_id;

use actix_web::{http::Method, web, HttpMessage, HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use mongodb::bson::{doc, oid::ObjectId, to_bson, DateTime, Document};
use futures::TryStreamExt;
use mongodb::Database;
use mongodb::results::InsertOneResult;
use crate::libraries::{method_not_allowed, response_bad_request, response_internal_server_error, response_ok_builder};

pub const PATH: &str = "/api/quiz";

#[derive(Clone, Serialize, Deserialize)]
pub struct SlideQuizQuestion {
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
#[serde(tag = "question_type")]
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
    body: Option<web::Json<QuizCreation>>,
    db: web::Data<Database>,
) -> impl Responder {
    if let Some(response_error) = crate::middlewares::jwt::middleware(&req, &db).await {
        return response_error;
    }

    if let Some(user_data) = req.extensions().get::<crate::middlewares::jwt::RequestUser>() {
        let owner_id = ObjectId::parse_str(&user_data.user.user_id).unwrap();

        match *req.method() {
            Method::GET => {
                let cursor = db.collection::<Document>("quizzes").find(doc! {
                    "owner_id": owner_id.clone(),
                    "is_deleted": { "$ne": true },
                }).projection(doc! {
                    "owner_id": 0,
                }).await.unwrap();

                let quizzes: Vec<Document> = cursor.try_collect().await.unwrap();

                response_ok_builder().json(json!({ "quizzes": quizzes }))
            }
            Method::POST => {
                let quiz_data = match body.ok_or_else(response_bad_request) {
                    Ok(data) => data,
                    Err(response) => return response,
                };

                let mut session = match db.client().start_session().await {
                    Ok(session) => session,
                    Err(_) => return response_internal_server_error(),
                };

                let created_at = DateTime::now();

                let result: Result<InsertOneResult, mongodb::error::Error> = async {
                    session.start_transaction().await?;

                    let slides_bson = to_bson(&quiz_data.slides).map_err(|e| mongodb::error::Error::from(e))?;

                    let quiz_result = db.collection::<Document>("quizzes")
                        .insert_one(
                            doc! {
                                "owner_id": owner_id.clone(),
                                "title": quiz_data.title.clone(),
                                "description": quiz_data.description.clone(),
                                "slides": slides_bson, // Dùng Bson từ to_bson
                                "updated_at": created_at.clone(),
                                "created_at": created_at.clone(),
                            }
                        )
                        .session(&mut session)
                        .await?;

                    session.commit_transaction().await?;

                    Ok(quiz_result)
                }.await;

                match result {
                    Ok(quiz_result) => {
                        let quiz_id = quiz_result.inserted_id.as_object_id().unwrap().to_hex();

                        response_ok_builder().json(json!({
                            "quiz_id": format!("0x{}", quiz_id),
                            "created_at": created_at,
                        }))
                    }
                    Err(_) => {
                        let _ = session.abort_transaction().await;
                        response_internal_server_error()
                    }
                }
            }
            _ => method_not_allowed(),
        }
    } else {
        response_internal_server_error()
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource(PATH)
        .route(web::post().to(handler))
        .route(web::get().to(handler)));
    cfg.configure(quiz_id::configure);
}