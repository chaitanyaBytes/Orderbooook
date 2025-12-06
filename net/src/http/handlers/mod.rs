use crate::http::{
    app::HttpServerAppState,
    models::orders::{OrderRequest, OrderResponse},
};
use actix_web::{HttpResponse, Responder, get, post, web};
use protocol::types::{Order, OrderCommand};
use serde_json::json;
use std::{sync::atomic::Ordering, time::Instant};

#[get("/ping")]
pub async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

#[post("/place")]
pub async fn place_order(
    req: web::Json<OrderRequest>,
    app_state: web::Data<HttpServerAppState>,
) -> impl Responder {
    let order_place_time = Instant::now();
    let body = req.into_inner();
    let order_id = app_state.order_id.fetch_add(1, Ordering::SeqCst);
    let order = Order::new(
        order_id,
        body.user_id,
        body.symbol,
        body.side,
        body.order_type,
        body.quantity,
        body.price,
    );

    let (tx, rx) = oneshot::channel::<OrderResponse>();
    if let Err(e) = app_state
        .order_tx
        .send((OrderCommand::PlaceOrder(order), tx))
    {
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to send order to engine",
            "message": e.to_string()
        }));
    }

    println!(
        "placed order in {}ms",
        order_place_time.elapsed().as_millis()
    );

    let received_time = Instant::now();

    match rx.await {
        Ok(response) => {
            println!(
                "order response received in {}ms",
                received_time.elapsed().as_millis(),
            );
            response.into_http_response()
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
        })),
    }
}

#[post("/cancel")]
pub async fn cancel_order() -> impl Responder {
    HttpResponse::Ok().body("order cancelled")
}

#[get("/depth")]
pub async fn get_depth() -> impl Responder {
    HttpResponse::Ok().body("depth")
}
