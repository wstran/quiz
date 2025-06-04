use std::collections::HashMap;
use actix_web::{web, HttpRequest, Responder, http::Method, HttpMessage};
use serde::{Deserialize, Serialize};
use serde_json::json;
use mongodb::bson::{doc, DateTime, oid::ObjectId, to_document, Document};
use mongodb::Database;
use rand::Rng;

use crate::libraries::{method_not_allowed, response_bad_request, response_internal_server_error, response_not_found, response_ok_builder};
use crate::libraries::redis::with_transaction;
use crate::routes::quiz::QuizCreation;

pub const PATH: &str = "/api/quiz/{quiz_id}";

#[derive(Serialize, Deserialize)]
struct Request {
    quiz_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct QuizRoom {
    quiz_id: String,
    owner_id: String,
    room_code: String,
    created_at: i64,
    players: HashMap<String, String>,
    current_slide: i32,
    scores: HashMap<String, i64>,
    started: bool,
}

pub(self) async fn handler(
    path: web::Path<Request>,
    req: HttpRequest,
    body: Option<web::Json<QuizCreation>>,
    db: web::Data<Database>,
) -> impl Responder {
    if let Some(response_error) = crate::middlewares::jwt::middleware(&req, &db).await {
        return response_error;
    }

    if let Some(user_data) = req.extensions().get::<crate::middlewares::jwt::RequestUser>() {
        let owner_id = ObjectId::parse_str(&user_data.user.user_id).unwrap();
        let quiz_id = &path.quiz_id[2..].to_string();

        match *req.method() {
            Method::GET => {
                let quiz: Document = match db.collection("quizzes").find_one(doc! {
                    "_id": ObjectId::parse_str(quiz_id.clone()).unwrap(),
                    "owner_id": owner_id.clone(),
                    "is_deleted": { "$ne": true }
                }).projection(doc! { "_id": 0 }).await {
                    Ok(Some(doc)) => doc,
                    Ok(None) => return response_not_found(),
                    Err(_) => return response_internal_server_error(),
                };

                response_ok_builder().json(json!({
                    "quiz_id": path.quiz_id.clone(),
                    "title": quiz.get("title").unwrap(),
                    "description": quiz.get("description").unwrap_or(&mongodb::bson::Bson::Null),
                    "slides": quiz.get("slides").unwrap(),
                    "updated_at": quiz.get("updated_at").unwrap_or(&mongodb::bson::Bson::Null),
                    "created_at": quiz.get("created_at").unwrap(),
                }))
            }
            Method::POST => {
                let quiz_exists = match db.collection::<Document>("quizzes").find_one(doc! {
                    "_id": ObjectId::parse_str(quiz_id.clone()).unwrap(),
                    "owner_id": owner_id.clone(),
                    "is_deleted": { "$ne": true }
                }).await {
                    Ok(Some(_)) => true,
                    Ok(None) => return response_not_found(),
                    Err(_) => return response_internal_server_error(),
                };

                if !quiz_exists {
                    return response_not_found();
                }

                let room_code: String = rand::rng()
                    .random_range(10000000..99999999)
                    .to_string();

                let room_key = format!("quiz_room:{}", room_code);

                let created_at = DateTime::now();

                let room = QuizRoom {
                    quiz_id: quiz_id.to_string(),
                    owner_id: user_data.user.user_id[2..].to_string(),
                    room_code: room_code.clone(),
                    created_at: created_at.clone().timestamp_millis(),
                    players: HashMap::new(),
                    current_slide: 0,
                    scores: HashMap::new(),
                    started: false,
                };

                let result: redis::RedisResult<()> = with_transaction(|pipe| {
                    let serialized = serde_json::to_string(&room).unwrap();
                    pipe.set_ex(&room_key, serialized, 3600);
                    Ok(())
                });

                match result {
                    Ok(()) => {
                        response_ok_builder().json(json!({
                            "quiz_id": format!("0x{}", quiz_id),
                            "room_code": room_code,
                            "created_at": created_at.clone(),
                        }))
                    }
                    Err(_) => response_internal_server_error(),
                }
            }
            Method::PUT => {
                let mut session = match db.client().start_session().await {
                    Ok(session) => session,
                    Err(_) => return response_internal_server_error(),
                };

                let updated_at = DateTime::now();

                let result = async {
                    session.start_transaction().await?;

                    let body = body.ok_or_else(response_bad_request).unwrap();

                    let update_result = db.collection::<Document>("quizzes").update_one(
                            doc! {
                                "_id": ObjectId::parse_str(quiz_id).unwrap(),
                                "owner_id": owner_id.clone(),
                                "is_deleted": { "$ne": true },
                            },
                            doc! {
                                "$set": {
                                    "title": body.title.clone(),
                                    "description": body.description.clone(),
                                    "slides": to_document(&body.slides).unwrap(),
                                    "updated_at": updated_at.clone(),
                                }
                            },
                        )
                        .session(&mut session)
                        .await?;

                    if update_result.matched_count == 0 {
                        return Err(mongodb::error::Error::custom("Quiz not found or not owned by user"));
                    }

                    session.commit_transaction().await?;

                    Ok(())
                }.await;

                match result {
                    Ok(()) => {
                        response_ok_builder().json(json!({
                            "quiz_id": format!("0x{}", quiz_id),
                            "updated_at": updated_at,
                        }))
                    }
                    Err(_) => {
                        let _ = session.abort_transaction().await;

                        response_internal_server_error()
                    }
                }
            }
            Method::DELETE => {
                let mut session = match db.client().start_session().await {
                    Ok(session) => session,
                    Err(_) => return response_internal_server_error(),
                };

                let deleted_at = DateTime::now();

                let result = async {
                    session.start_transaction().await?;

                    let update_result = db.collection::<Document>("quizzes").update_one(
                            doc! {
                                "_id": ObjectId::parse_str(quiz_id).unwrap(),
                                "owner_id": owner_id.clone(),
                                "is_deleted": { "$ne": true },
                            },
                            doc! {
                                "$set": {
                                    "is_deleted": true,
                                    "deleted_at": deleted_at.clone(),
                                }
                            }
                        )
                        .session(&mut session)
                        .await?;

                    if update_result.matched_count == 0 {
                        return Err(mongodb::error::Error::custom("Quiz not found or not owned by user"));
                    }

                    session.commit_transaction().await?;

                    Ok(())
                }.await;

                match result {
                    Ok(()) => {
                        response_ok_builder().json(json!({
                            "quiz_id": format!("0x{}", quiz_id),
                            "deleted_at": deleted_at,
                        }))
                    }
                    Err(_) => {
                        let _ = session.abort_transaction().await;

                        response_internal_server_error()
                    }
                }
            }
            _ => method_not_allowed()
        }
    } else {
        response_internal_server_error()
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource(PATH)
            .route(web::get().to(handler))
            .route(web::post().to(handler))
            .route(web::put().to(handler))
            .route(web::delete().to(handler))
    );
}