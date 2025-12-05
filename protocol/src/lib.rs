use serde::{Deserialize, Serialize};

// Inbound events (client -> engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    PlaceOrder(Order),
    CancelOrder(CancelOrder),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: u64,
    pub user_id: u64,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: u64,
    pub price: Option<u64>, // None if market order
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrder {
    pub order_id: u64,
    pub user_id: u64,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Limit,
    Market,
}

// Outbound events (engine -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    OrderAck(OrderAck),
    OrderReject(OrderReject),
    Fill(Fill),
    Trade(Trade),
    OrderCancelled(OrderCancelled),
    BookUpdate(BookUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAck {
    pub order_id: u64,
    pub user_id: u64,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReject {
    pub order_id: u64,
    pub user_id: u64,
    pub reason: RejectReason,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RejectReason {
    InvalidPrice,
    InvalidOrder,
    InvalidQuantity,
    InsufficientBalance,
    SymbolNotFound,
    MarketClosed,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: u64,
    pub user_id: u64,
    pub symbol: String,
    pub side: Side,
    pub filled_quantity: u64,
    pub filled_price: u64,
    pub remaining_quantity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: u64,
    pub maker_order_id: u64,
    pub maker_user_id: u64,
    pub taker_order_id: u64,
    pub taker_user_id: u64,
    pub symbol: String,
    pub quantity: u64,
    pub price: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelled {
    pub order_id: u64,
    pub user_id: u64,
    pub symbol: String,
    pub reason: CancelReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CancelReason {
    UserRequested,
    SystemCancelled,
    Expired,
    Liquidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookUpdate {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub last_price: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: u64,
    pub quantity: u64,
}

pub type OrderId = u64;
pub type UserId = u64;
pub type Price = u64;
pub type Quantity = u64;
