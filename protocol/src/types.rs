use serde::{Deserialize, Serialize};

pub type OrderId = u64;
pub type UserId = u64;
pub type Price = u64;
pub type Quantity = u64;
pub type TradeId = u64;

// Inbound events (client -> engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    PlaceOrder(Order),
    CancelOrder(CancelOrder),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub quantity: Quantity,
    pub price: Option<Price>, // None if market order
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrder {
    pub order_id: OrderId,
    pub user_id: UserId,
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
    pub order_id: OrderId,
    pub user_id: UserId,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReject {
    pub order_id: OrderId,
    pub user_id: UserId,
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
    pub order_id: OrderId,
    pub user_id: UserId,
    pub symbol: String,
    pub side: Side,
    pub filled_quantity: Quantity,
    pub filled_price: Price,
    pub remaining_quantity: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: TradeId,
    pub maker_order_id: OrderId,
    pub maker_user_id: UserId,
    pub taker_order_id: OrderId,
    pub taker_user_id: UserId,
    pub symbol: String,
    pub quantity: Quantity,
    pub price: Price,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelled {
    pub order_id: OrderId,
    pub user_id: UserId,
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
    pub last_price: Option<Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Price,
    pub quantity: Quantity,
}

impl Order {
    pub fn new(
        order_id: OrderId,
        user_id: UserId,
        symbol: String,
        side: Side,
        order_type: OrderType,
        quantity: Quantity,
        price: Option<Price>,
    ) -> Self {
        Self {
            order_id,
            user_id,
            symbol,
            side,
            order_type,
            quantity,
            price,
        }
    }
}
