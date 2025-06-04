use actix_web::{HttpRequest, HttpResponse, HttpMessage};
use actix_web::web::Data;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, DateTime, Document};
use mongodb::Database;
use crate::env::JWT_SECRET;
use crate::libraries::{response_bad_request, response_forbidden, response_internal_server_error};

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtUser {
    pub user_id: String,
    pub method: String,
}

#[derive(Debug)]
pub struct RequestUser {
    pub user: JwtUser,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    method: String,
    exp: usize,
}

pub async fn middleware(req: &HttpRequest, db: &Data<Database>) -> Option<HttpResponse> {
    let token = match req.cookie("--auth-token") {
        Some(cookie) => cookie.value().to_string(),
        None => return Some(response_bad_request()),
    };

    let token_data = match decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(_) => return Some(response_forbidden()),
    };

    let jwt_user = JwtUser {
        user_id: (&token_data.claims.sub[2..]).to_string(),
        method: token_data.claims.method,
    };

    match db.collection::<Document>("users").update_one(
        doc! { "google_id": jwt_user.user_id.clone() },
        doc! {
            "$set": {
                "last_active": DateTime::now()
            }
        },
    ).await {
        Ok(_) => {
            req.extensions_mut().insert(RequestUser { user: jwt_user });
            None
        }
        Err(_) => Some(response_internal_server_error()),
    }
}