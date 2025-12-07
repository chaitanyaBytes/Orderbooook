use actix_web::HttpResponse;
use protocol::types::{OrderId, OrderType, Price, Quantity, RejectReason, Side, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResponse {
    PlaceOrder(OrderResponse),
    CancelOrder(CancelOrderResponse),
    Depth(DepthResponse),
}

impl CommandResponse {
    pub fn into_http_response(self) -> HttpResponse {
        match self {
            CommandResponse::PlaceOrder(resp) => resp.into_http_response(),
            CommandResponse::CancelOrder(resp) => resp.into_http_response(),
            CommandResponse::Depth(resp) => HttpResponse::Ok().json(resp),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub user_id: UserId,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: Quantity,
    pub price: Option<Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderResponse {
    Ack {
        order_id: u64,
        user_id: u64,
        symbol: String,
    },
    Reject {
        order_id: u64,
        reason: RejectReason,
        message: String,
    },
}

impl OrderResponse {
    pub fn into_http_response(self) -> HttpResponse {
        match self {
            OrderResponse::Ack { .. } => HttpResponse::Ok().json(self),
            OrderResponse::Reject { .. } => HttpResponse::BadRequest().json(self),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    pub user_id: UserId,
    pub symbol: String,
    pub order_id: OrderId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CancelOrderResponse {
    Ack {
        order_id: OrderId,
        user_id: UserId,
        symbol: String,
    },
    Reject {
        order_id: OrderId,
        reason: RejectReason,
        message: String,
    },
}

impl CancelOrderResponse {
    pub fn into_http_response(self) -> HttpResponse {
        match self {
            CancelOrderResponse::Ack { .. } => HttpResponse::Ok().json(self),
            CancelOrderResponse::Reject { .. } => HttpResponse::BadRequest().json(self),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthQuery {
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthResponse {
    pub bids: Vec<(Price, Quantity)>,
    pub asks: Vec<(Price, Quantity)>,
}
