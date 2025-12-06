use crate::http::handlers::{cancel_order, get_depth, ping, place_order};
use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .service(ping)
            .service(
                web::scope("/orders")
                    .service(place_order)
                    .service(cancel_order),
            )
            .service(get_depth),
    );
}
