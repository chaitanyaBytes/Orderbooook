use crate::http::{
    app::HttpServerAppState,
    models::orders::{CancelOrderRequest, CommandResponse, DepthQuery, OrderRequest},
};
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use protocol::types::{CancelOrder, Order, OrderCommand};
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

    let (tx, rx) = oneshot::channel::<CommandResponse>();
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
        order_place_time.elapsed().as_micros()
    );

    let received_time = Instant::now();

    match rx.await {
        Ok(response) => {
            println!(
                "order response received in {}ms",
                received_time.elapsed().as_micros(),
            );
            response.into_http_response()
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
        })),
    }
}

#[delete("/cancel")]
pub async fn cancel_order(
    req: web::Json<CancelOrderRequest>,
    app_state: web::Data<HttpServerAppState>,
) -> impl Responder {
    let cancel_order_time = Instant::now();
    let body = req.into_inner();
    let cancel_order = CancelOrder::new(body.order_id, body.user_id, body.symbol);
    let (tx, rx) = oneshot::channel::<CommandResponse>();

    if let Err(e) = app_state
        .order_tx
        .send((OrderCommand::CancelOrder(cancel_order), tx))
    {
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to send cancel order to engine",
            "message": e.to_string()
        }));
    }

    let received_time = Instant::now();
    match rx.await {
        Ok(response) => {
            println!(
                "cancel order response received in {}ms",
                received_time.elapsed().as_millis()
            );
            response.into_http_response()
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
        })),
    }
}

#[get("/depth/{symbol}")]
pub async fn get_depth(
    path: web::Path<String>,
    query: web::Query<DepthQuery>,
    app_state: web::Data<HttpServerAppState>,
) -> impl Responder {
    let _symbol = path.into_inner();
    let _limit = query.into_inner().limit;

    let (tx, rx) = oneshot::channel::<CommandResponse>();
    if let Err(e) = app_state.order_tx.send((OrderCommand::GetDepth, tx)) {
        return HttpResponse::InternalServerError().json(json!({
            "error": "Failed to send get depth to engine",
            "message": e.to_string()
        }));
    }

    let received_time = Instant::now();
    match rx.await {
        Ok(response) => {
            println!(
                "get depth response received in {}ms",
                received_time.elapsed().as_millis()
            );
            response.into_http_response()
        }

        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
        })),
    }
}
