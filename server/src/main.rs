use actix_web::{App, HttpServer};
use actix_cors::Cors;
use actix_web::http::header::HeaderName;
use mongodb::bson::doc;
use tracing::{info, Level};
use tracing_subscriber;
use crate::libraries::mongodb::{get_db, init_mongodb, create_index};
use crate::libraries::redis::init_redis;

mod env;
mod routes;
mod libraries;
mod middlewares;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let config = env::AppConfig::from_env();

    init_mongodb().await;

    init_redis().await;

    // key: TELEGRAM_ONLY
    // match create_index("users", doc! { "tele_id": 1 }, true, true).await {
    //     Ok(index_name) => println!("Index created: {}", index_name.index_name),
    //     Err(e) => eprintln!("Failed to create index: {}", e),
    // }

    match create_index("users", doc! { "email": 1 }, true, true).await {
        Ok(index_name) => println!("Index created: {}", index_name.index_name),
        Err(e) => eprintln!("Failed to create index: {}", e),
    }

    match create_index("users", doc! { "google_id": 1 }, true, true).await {
        Ok(index_name) => println!("Index created: {}", index_name.index_name),
        Err(e) => eprintln!("Failed to create index: {}", e),
    }

    match create_index("quizzes", doc! { "owner_id": 1 }, true, false).await {
        Ok(index_name) => println!("Index created: {}", index_name.index_name),
        Err(e) => eprintln!("Failed to create index: {}", e),
    }

    info!(
        "Starting server at {}:{} with {} workers",
        config.app_host, config.app_port, config.worker_count
    );

    HttpServer::new(|| {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    // key: TELEGRAM_ONLY
                    // .allowed_header(HeaderName::from_static("--webapp-hash"))
                    // .allowed_header(HeaderName::from_static("--webapp-init"))
                    .allowed_header(HeaderName::from_static("--auth-token"))
                    .supports_credentials()
            )
            .app_data(actix_web::web::Data::new(get_db().clone()))
            .configure(routes::register_routes)
    })
    .bind((config.app_host.as_str(), config.app_port))?
    .workers(config.worker_count)
    .run()
    .await
}
