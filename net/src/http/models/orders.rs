use actix_web::HttpResponse;
use protocol::types::{OrderType, Price, Quantity, RejectReason, Side, UserId};
use serde::{Deserialize, Serialize};

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
