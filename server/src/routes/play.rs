use std::collections::HashMap;
use actix_web::{web, HttpRequest, HttpResponse, Error, HttpMessage};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use serde_json::json;
use mongodb::bson::{doc, oid::ObjectId, Document};
use mongodb::Database;
use sha2::{Sha256, Digest};
use actix::{Actor, Addr, AsyncContext, Context, Message, Handler};
use std::sync::Arc;
use std::thread;
use tokio::sync::Mutex;
use actix_rt;
use crate::env::JWT_SECRET;
use crate::libraries::redis::{RedisConn, with_transaction};

#[derive(Message)]
#[rtype(result = "()")]
struct Register {
    room_code: String,
    unique_id: String,
    addr: Addr<QuizWebSocket>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    room_code: String,
    unique_id: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct WsMessage(String);

#[derive(Clone, Deserialize)]
pub(self) struct QuizRequest {
    room_code: String,
    nickname: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QuizRoom {
    quiz_id: String,
    owner_id: String,
    room_code: String,
    created_at: i64,
    players: HashMap<String, String>,
    scores: HashMap<String, i64>,
    current_slide: i32,
    started: bool,
}

struct QuizWebSocket {
    unique_id: String,
    user_id: String,
    nickname: String,
    room_key: String,
    db: web::Data<Database>,
    manager: Addr<ConnectionManager>,
}

pub(self) struct ConnectionManager {
    connections: Arc<Mutex<HashMap<String, HashMap<String, Addr<QuizWebSocket>>>>>,
}

impl Actor for ConnectionManager {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        let connections = self.connections.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut redis_connect = match RedisConn::get_connection() {
                    Ok(conn) => conn,
                    Err(_) => return,
                };

                let mut pubsub = match redis_connect.pubsub() {
                    Ok(pubsub) => pubsub,
                    Err(_) => return,
                };

                pubsub.subscribe("disconnect").unwrap();

                loop {
                    match pubsub.get_message() {
                        Ok(msg) => {
                            let payload: String = msg.get_payload().unwrap_or_default();

                            if let Some((room_code, unique_id)) = payload.split_once(':') {
                                let room_code = room_code.to_string();
                                let unique_id = unique_id.to_string();
                                let connections = connections.clone();

                                tokio::spawn(async move {
                                    let mut conns = connections.lock().await;

                                    if let Some(room_conns) = conns.get_mut(&room_code) {
                                        if let Some(addr) = room_conns.remove(&unique_id) {
                                            addr.do_send(WsMessage(json!({ "error": "Disconnected due to new connection." }).to_string()));
                                            addr.do_send(WsMessage("close".to_string()));
                                        }
                                    }
                                });
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        });
    }
}

impl Handler<Register> for ConnectionManager {
    type Result = actix::ResponseFuture<()>;

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        let connections = self.connections.clone();

        Box::pin(async move {
            let mut conns = connections.lock().await;
            let room_conns = conns.entry(msg.room_code).or_insert_with(HashMap::new);

            room_conns.insert(msg.unique_id, msg.addr);
        })
    }
}

impl Handler<Disconnect> for ConnectionManager {
    type Result = actix::ResponseFuture<()>;

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) -> Self::Result {
        let connections = self.connections.clone();

        Box::pin(async move {
            let mut conns = connections.lock().await;

            if let Some(room_conns) = conns.get_mut(&msg.room_code) {
                if let Some(addr) = room_conns.remove(&msg.unique_id) {

                    addr.do_send(WsMessage(json!({ "error": "Disconnected." }).to_string()));
                    addr.do_send(WsMessage("close".to_string()));
                }
            }
        })
    }
}

fn generate_unique_id(req: &HttpRequest) -> String {
    let conn_info = req.connection_info();
    let ip = conn_info.realip_remote_addr().unwrap_or("unknown");
    let user_agent = req.headers().get("user-agent").map_or("unknown", |h| h.to_str().unwrap());
    let cf_warp_tag_id = req.headers().get("cf-warp-tag-id").map_or("unknown", |h| h.to_str().unwrap());

    let mut hasher = Sha256::new();

    hasher.update(format!("{}:{}:{}:{}", ip, user_agent, cf_warp_tag_id, JWT_SECRET.clone()));

    let result = hasher.finalize();

    format!("{:x}", result)
}

async fn handler(
    req: HttpRequest,
    query: web::Query<QuizRequest>,
    db: web::Data<Database>,
    stream: web::Payload,
    manager: web::Data<Addr<ConnectionManager>>,
) -> Result<HttpResponse, Error> {
    crate::middlewares::jwt::middleware(&req, &db).await;

    let user_id = req
        .extensions()
        .get::<crate::middlewares::jwt::RequestUser>()
        .map(|data| data.user.user_id.clone())
        .unwrap_or(String::new());

    let unique_id = generate_unique_id(&req);
    let room_code = query.room_code.clone();
    let nickname = query.nickname.clone();
    let room_key = format!("quiz_room:{}", room_code);

    // let mut redis_connect = RedisConn::get_connection()
    //     .map_err(|_| crate::libraries::response_internal_server_error()).unwrap();
    // let room_exists: bool = redis_connect.exists(&room_key).unwrap_or(false);
    // if !room_exists {
    //     let mock_quiz_id = "507f1f77bcf86cd799439011";
    //     let mock_room = QuizRoom {
    //         quiz_id: mock_quiz_id.to_string(),
    //         owner_id: "507f191e810c19729de860ea".to_string(),
    //         room_code: room_code.clone(),
    //         created_at: chrono::Utc::now().timestamp_millis(),
    //         players: HashMap::new(),
    //         current_slide: 0,
    //         scores: HashMap::new(),
    //         started: false,
    //     };
    //
    //     let room_str = serde_json::to_string(&mock_room)?;
    //
    //     let result: redis::RedisResult<()> = with_transaction(|pipe| {
    //         pipe.set_ex(&room_key, &room_str, 3600);
    //         Ok(())
    //     });
    //
    //     result.map_err(|_| crate::libraries::response_internal_server_error()).unwrap();
    //
    //     let quiz_collection = db.collection::<Document>("quizzes");
    //     let quiz_exists = quiz_collection
    //         .find_one(doc! { "_id": ObjectId::parse_str(mock_quiz_id).unwrap() })
    //         .await
    //         .map_err(|_| crate::libraries::response_internal_server_error()).unwrap()
    //         .is_some();
    //
    //     if !quiz_exists {
    //         let mock_quiz = doc! {
    //             "_id": ObjectId::parse_str(mock_quiz_id).unwrap(),
    //             "owner_id": ObjectId::parse_str("507f191e810c19729de860ea").unwrap(),
    //             "title": "Mock Quiz",
    //             "description": "This is a mock quiz for testing",
    //             "slides": [
    //                 {
    //                     "question": "What is 2 + 2?",
    //                     "options": ["3", "4", "5", "6"],
    //                     "correct_answer": 1
    //                 }
    //             ],
    //             "created_at": mongodb::bson::DateTime::now(),
    //             "updated_at": mongodb::bson::DateTime::now(),
    //             "is_deleted": false
    //         };
    //         quiz_collection
    //             .insert_one(mock_quiz)
    //             .await
    //             .map_err(|_| crate::libraries::response_internal_server_error()).unwrap();
    //     }
    // }

    let ws = ws::start(
        QuizWebSocket {
            unique_id,
            user_id,
            nickname,
            room_key,
            db,
            manager: manager.get_ref().clone(),
        },
        &req,
        stream,
    )?;

    Ok(ws)
}

impl Actor for QuizWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db = self.db.clone();
        let unique_id = self.unique_id.clone();
        let room_key = self.room_key.clone();
        let nickname = self.nickname.clone();
        let addr = ctx.address();
        let manager = self.manager.clone();
        let room_code = room_key.replace("quiz_room:", "");

        ctx.spawn(actix::fut::wrap_future(async move {
            let mut redis_connect = match RedisConn::get_connection() {
                Ok(conn) => conn,
                Err(_) => {
                    addr.do_send(WsMessage(json!({ "error": "Internal server error." }).to_string()));
                    addr.do_send(WsMessage("close".to_string()));
                    return;
                }
            };

            let room_exists: bool = redis_connect.exists(&room_key).unwrap_or(false);

            if !room_exists {
                addr.do_send(WsMessage(json!({ "error": "Not found." }).to_string()));
                addr.do_send(WsMessage("close".to_string()));
                return;
            }

            let room_str: Option<String> = redis_connect.get(&room_key).unwrap_or(None);

            let mut room: QuizRoom = match room_str {
                Some(r) => match serde_json::from_str(&r) {
                    Ok(room) => room,
                    Err(_) => {
                        addr.do_send(WsMessage(json!({ "error": "Bot found." }).to_string()));
                        addr.do_send(WsMessage("close".to_string()));
                        return;
                    }
                },
                None => {
                    addr.do_send(WsMessage(json!({ "error": "Not found." }).to_string()));
                    addr.do_send(WsMessage("close".to_string()));
                    return;
                }
            };

            let quiz_exists = db.collection::<Document>("quizzes")
                .find_one(doc! {
                    "_id": ObjectId::parse_str(&room.quiz_id).unwrap(),
                    "is_deleted": { "$ne": true }
                })
                .await
                .is_ok_and(|doc| doc.is_some());

            if !quiz_exists {
                addr.do_send(WsMessage(json!({ "error": "Not found" }).to_string()));
                addr.do_send(WsMessage("close".to_string()));
                return;
            }

            if room.players.contains_key(&unique_id) {
                manager.do_send(Disconnect {
                    room_code: room_code.clone(),
                    unique_id: unique_id.clone(),
                });

                redis_connect.publish("disconnect", &format!("{}:{}", room_code, unique_id)).unwrap_or(0);

                room.players.remove(&unique_id);

                room.scores.remove(&unique_id);
            }

            room.players.insert(unique_id.clone(), nickname.clone());

            room.scores.insert(unique_id.clone(), 0);

            manager.do_send(Register {
                room_code: room_code.clone(),
                unique_id: unique_id.clone(),
                addr: addr.clone(),
            });

            let room_str = serde_json::to_string(&room).unwrap();

            let result: redis::RedisResult<()> = with_transaction(|pipe| {
                pipe.set_ex(&room_key, &room_str, 3600);
                Ok(())
            });

            if result.is_err() {
                addr.do_send(WsMessage(json!({ "error": "Bad request." }).to_string()));
                addr.do_send(WsMessage("close".to_string()));
                return;
            }

            let players_list: Vec<(String, String)> = room.players.clone().into_iter().collect();

            let scores_list: Vec<(String, i64)> = room.scores.clone().into_iter().collect();

            addr.do_send(WsMessage(json!({
                "action": "update_data",
                "room_code": room.room_code,
                "current_slide": room.current_slide,
                "players": players_list,
                "scores": scores_list,
                "started": room.started,
            }).to_string()));
        }));
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let room_key = self.room_key.clone();
        let unique_id = self.unique_id.clone();
        let manager = self.manager.clone();
        let room_code = room_key.replace("quiz_room:", "");

        ctx.spawn(actix::fut::wrap_future(async move {
            if let Ok(mut redis_connect) = RedisConn::get_connection() {
                if let Some(room_str) = redis_connect.get(&room_key).unwrap_or(None) {
                    if let Ok(mut room) = serde_json::from_str::<QuizRoom>(&room_str) {

                        room.players.remove(&unique_id);

                        room.scores.remove(&unique_id);

                        if let Ok(room_str) = serde_json::to_string(&room) {
                            let _: redis::RedisResult<()> = with_transaction(|pipe| {
                                pipe.set_ex(&room_key, &room_str, 3600);
                                Ok(())
                            });
                        }
                    }
                }
            }

            manager.do_send(Disconnect {
                room_code,
                unique_id,
            });
        }));
    }
}

impl Handler<WsMessage> for QuizWebSocket {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        if msg.0 == "close" {
            ctx.close(None);
        } else {
            ctx.text(msg.0);
        }
    }
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for QuizWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                let message = text.trim().to_string();
                ctx.text(json!({"message": format!("Echo: {}", message)}).to_string());
            }
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(_)) => ctx.close(None),
            _ => (),
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    let manager = ConnectionManager {
        connections: Arc::new(Mutex::new(HashMap::new())),
    }.start();
    cfg.app_data(web::Data::new(manager));
    cfg.service(web::resource("/api/play").route(web::get().to(handler)));
}