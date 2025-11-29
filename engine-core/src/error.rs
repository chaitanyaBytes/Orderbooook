use protocol::OrderId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrderBookError {
    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    #[error("Invalid Order: {0}")]
    InvalidOrder(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
