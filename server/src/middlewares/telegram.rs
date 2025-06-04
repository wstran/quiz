// key: TELEGRAM_ONLY
// use actix_web::{HttpRequest, HttpResponse, http::Method, HttpMessage};
// use chrono::{TimeZone, Utc};
// use hmac::{Hmac, Mac};
// use md5;
// use mongodb::bson::{doc, DateTime};
// use sha2::Sha256;
// use url::form_urlencoded;
// type HmacSha256 = Hmac<Sha256>;
//
// use crate::env::BOT_TOKEN;
// use crate::libraries::mongodb::update_one;
// use crate::libraries::{response_bad_request, response_forbidden, response_internal_server_error};
//
// pub struct TeleUser {
//     pub tele_id: String,
//     pub name: String,
//     pub username: String,
//     pub auth_date: chrono::DateTime<Utc>
// }
//
// pub struct RequestUser {
//     pub user: TeleUser,
// }
//
// pub async fn middleware(req: &HttpRequest) -> Option<HttpResponse> {
//     let headers = req.headers();
//     let webapp_init = headers.get("--webapp-init").and_then(|v| v.to_str().ok());
//     let webapp_hash = headers.get("--webapp-hash").and_then(|v| v.to_str().ok());
//
//     if webapp_init.is_none() || webapp_hash.is_none() {
//         return Some(response_bad_request());
//     }
//
//     let webapp_init = webapp_init?;
//     let webapp_hash = webapp_hash?;
//
//     let parts: Vec<&str> = webapp_hash.split(':').collect();
//
//     if parts.len() != 2 {
//         return Some(response_bad_request());
//     }
//
//     let timestamp_str = parts[0];
//     let request_hash = parts[1];
//
//     let timestamp: i64 = timestamp_str.parse().unwrap_or(0);
//     let now = Utc::now().timestamp_millis();
//
//     if timestamp + 4000 < now {
//         return Some(response_bad_request());
//     }
//
//     let mut data_to_sign = format!("timestamp={}&initData={}", timestamp_str, webapp_init);
//
//     if req.method() == Method::GET {
//         let qs = req.query_string();
//         if !qs.is_empty() {
//             data_to_sign.push_str(&format!("&params={}", qs));
//         }
//     }
//
//     let md5_input = format!("{}", data_to_sign);
//     let server_signature = format!("{:x}", md5::compute(md5_input));
//
//     if server_signature != request_hash {
//         return Some(response_bad_request());
//     }
//
//     let mut params: Vec<(String, String)> =
//         form_urlencoded::parse(webapp_init.as_bytes()).into_owned().collect();
//
//     let expected_hmac_opt = params.iter().find(|(k, _)| k == "hash").map(|(_, v)| v.clone());
//
//     if expected_hmac_opt.is_none() {
//         return Some(response_bad_request());
//     }
//     let expected_hmac = expected_hmac_opt?;
//
//     params.retain(|(k, _)| k != "hash");
//     params.sort_by(|a, b| a.0.cmp(&b.0));
//
//     let data_check_string = params
//         .into_iter()
//         .map(|(k, v)| format!("{}={}", k, v))
//         .collect::<Vec<_>>()
//         .join("\n");
//
//     let derived_secret = {
//         let mut mac = HmacSha256::new_from_slice("WebAppData".as_bytes())
//             .expect("HMAC can take key of any size");
//         mac.update(BOT_TOKEN.as_bytes());
//         mac.finalize().into_bytes()
//     };
//
//     let mut mac = HmacSha256::new_from_slice(&derived_secret)
//         .expect("HMAC can take key of any size");
//     mac.update(data_check_string.as_bytes());
//
//     let computed_hmac = hex::encode(mac.finalize().into_bytes());
//
//     if computed_hmac != expected_hmac {
//         return Some(response_forbidden());
//     }
//
//     let params: Vec<(String, String)> =
//         form_urlencoded::parse(webapp_init.as_bytes()).into_owned().collect();
//     let user_param = params.iter().find(|(k, _)| k == "user").map(|(_, v)| v.clone());
//     let auth_date = params.iter().find(|(k, _)| k == "auth_date").map(|(_, v)| v.clone());
//
//     if user_param.is_none() || auth_date.is_none() {
//         return Some(response_bad_request());
//     }
//
//     let parsed_user: serde_json::Value =
//         serde_json::from_str(&user_param.unwrap()).unwrap_or_default();
//
//     let tele_id = parsed_user.get("id")?.to_string();
//     let first_name = parsed_user
//         .get("first_name")
//         .and_then(|v| v.as_str())
//         .unwrap_or("");
//     let last_name = parsed_user
//         .get("last_name")
//         .and_then(|v| v.as_str())
//         .unwrap_or("");
//     let username = parsed_user
//         .get("username")
//         .and_then(|v| v.as_str())
//         .unwrap_or("")
//         .to_string();
//     let auth_date_ms: i64 = auth_date.unwrap().parse().unwrap_or(0) * 1000;
//
//     let user = TeleUser {
//         tele_id: tele_id.clone(),
//         name: format!("{} {}", first_name, last_name).trim().to_string(),
//         username,
//         auth_date: Utc.timestamp_millis_opt(auth_date_ms).single()?
//     };
//
//     match update_one(
//         "users",
//         doc! { "tele_id": user.tele_id.clone() },
//         doc! {
//             "$set": {
//                 "name": user.name.clone(),
//                 "username": user.username.clone(),
//                 "auth_date": DateTime::from_millis(user.auth_date.clone().timestamp_millis()),
//                 "last_active": DateTime::now()
//             },
//             "$setOnInsert": {
//                 "tele_id": user.tele_id.clone()
//             }
//         },
//         true
//     ).await {
//         Ok(_) => {
//             req.extensions_mut().insert(RequestUser { user });
//
//             None
//         },
//         Err(_) => Some(response_internal_server_error())
//     }
// }