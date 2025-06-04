use actix_web::web;

mod auth;
mod index;
mod quiz;
mod play;

mod user;

pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure);
    cfg.configure(index::configure);
    cfg.configure(quiz::configure);
    cfg.configure(play::configure);
    cfg.configure(user::configure);
}