use actix_web::{web, HttpResponse, Responder};
use actix_web::cookie::Cookie;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl, AuthorizationCode, CsrfToken, TokenResponse, Client, StandardRevocableToken, EndpointSet, EndpointNotSet, Scope};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, DateTime, Document};
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::Utc;
use mongodb::Database;
use oauth2::basic::{BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse, BasicTokenResponse};
use crate::env::{APP_URL, GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, JWT_SECRET};
use crate::libraries::{response_bad_request, response_internal_server_error, response_ok_builder};

#[derive(Deserialize, Serialize)]
struct GoogleUser {
    #[serde(rename = "sub")]
    id: String,
    email: String,
    name: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    picture: Option<String>,
    email_verified: bool,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    method: String,
    exp: usize,
}

fn google_oauth_client() -> Client<BasicErrorResponse, BasicTokenResponse, BasicTokenIntrospectionResponse, StandardRevocableToken, BasicRevocationErrorResponse, EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet> {
    let client_id = ClientId::new(GOOGLE_CLIENT_ID.to_string());
    let client_secret = ClientSecret::new(GOOGLE_CLIENT_SECRET.to_string());
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
        .expect("Invalid token endpoint URL");

    BasicClient::new(client_id)
        .set_client_secret(client_secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(
            RedirectUrl::new(format!("{}/auth/google/callback", APP_URL.clone()).to_string())
                .expect("Invalid redirect URL"),
        )
}

pub async fn google_auth_redirect() -> impl Responder {
    let client = google_oauth_client();

    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .url();

    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

#[derive(Deserialize)]
pub struct GoogleCallbackQuery {
    code: String,
}

async fn google_auth_callback(
    query: web::Query<GoogleCallbackQuery>,
    db: web::Data<Database>,
) -> impl Responder {
    let client = google_oauth_client();
    let code = AuthorizationCode::new(query.code.clone());
    let client_secret = ClientSecret::new(GOOGLE_CLIENT_SECRET.to_string());

    let token_response = reqwest::Client::new()
        .post(client.token_uri().to_string())
        .form(&[
            ("code", code.secret()),
            ("client_id", client.client_id()),
            ("client_secret", client_secret.secret()),
            ("redirect_uri", client.redirect_uri().unwrap()),
            ("grant_type", &"authorization_code".to_string()),
        ])
        .send()
        .await;

    match token_response {
        Ok(response) => {
            let token = match response.json::<BasicTokenResponse>().await {
                Ok(t) => t,
                Err(_) => return response_internal_server_error(),
            };

            let user_info: GoogleUser = match ReqwestClient::new()
                .get("https://www.googleapis.com/oauth2/v3/userinfo")
                .bearer_auth(token.access_token().secret())
                .send()
                .await
            {
                Ok(response) => match response.json().await {
                    Ok(info) => info,
                    Err(_) => return response_internal_server_error(),
                },
                Err(_) => return response_bad_request(),
            };

            let existing_user = match db.collection::<Document>("users").find_one(
                doc! { "google_id": user_info.id.clone() },
            ).await {
                Ok(user) => user,
                Err(_) => return response_internal_server_error(),
            };

            let user_id = if let Some(user) = existing_user {
                match db.collection::<Document>("users").update_one(
                    doc! { "google_id": user_info.id.clone() },
                    doc! {
                        "$set": {
                            "last_active": DateTime::now(),
                            "last_auth": DateTime::now()
                        }
                    }
                ).await {
                    Ok(_) => {
                        user.get("_id").unwrap().as_object_id().unwrap().to_string()
                    },
                    Err(_) => return response_internal_server_error(),
                }
            } else {
                let new_user = doc! {
                    "google_id": user_info.id.clone(),
                    "email": user_info.email.clone(),
                    "name": user_info.name.clone().unwrap_or_default(),
                    "method": "google",
                    "created_at": DateTime::now(),
                    "last_active": DateTime::now(),
                    "last_auth": DateTime::now()
                };
                let insert_result = match db.collection::<Document>("users").insert_one(new_user).await {
                    Ok(result) => result,
                    Err(_) => return response_internal_server_error(),
                };

                insert_result.inserted_id.as_object_id().unwrap().to_string()
            };

            let claims = Claims {
                sub: format!("0x{}", user_id),
                method: "google".to_string(),
                exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            };
            let token = match encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(JWT_SECRET.as_ref()),
            ) {
                Ok(t) => t,
                Err(_) => return response_internal_server_error(),
            };

            response_ok_builder()
                .cookie(
                    Cookie::build("--auth-token", token.clone())
                        .path("/")
                        .secure(APP_URL.clone().starts_with("https"))
                        .http_only(true)
                        .max_age(time::Duration::days(30))
                        .finish()
                )
                .content_type("text/html")
                .body(format!(
                    r#"<script>
                        window.opener.postMessage({{ status: 'success' }}, '{}');
                        window.close();
                    </script>"#,
                    APP_URL.clone().to_string()
                ))
        }
        Err(_) => response_bad_request(),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/auth/google").route(web::get().to(google_auth_redirect)),
    );
    cfg.service(
        web::resource("/auth/google/callback").route(web::get().to(google_auth_callback)),
    );
}