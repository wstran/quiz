mod google;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(google::configure);
}