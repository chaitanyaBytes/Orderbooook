use protocol::types::{OrderId, Price, PriceLevel, Quantity, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Trade(TradeEvent),
    Depth(DepthEvent),
    Ticker(TickerEvent),
    OrderUpdate(UserOrderUpdateEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEvent {
    pub trade_id: u64,
    pub symbol: String,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: i64,
    // pub side: Side,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthEvent {
    pub symbol: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp: i64,
    pub last_price: Option<Price>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerEvent {
    pub symbol: String,
    pub last_price: Price,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub volume: Quantity,
    pub price_change: Price,
    pub price_change_percent: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserOrderUpdateEvent {
    Fill {
        order_id: OrderId,
        user_id: UserId,
        symbol: String,
        filled_quantity: Quantity,
        filled_price: Price,
        remaining_quantity: Quantity,
        timestamp: i64,
    },
    Ack {
        order_id: OrderId,
        user_id: UserId,
        symbol: String,
        timestamp: i64,
    },
    Reject {
        order_id: OrderId,
        user_id: UserId,
        symbol: String,
        reason: String,
        message: String,
        timestamp: i64,
    },
    Cancelled {
        order_id: OrderId,
        user_id: UserId,
        symbol: String,
        timestamp: i64,
    },
}

impl Event {
    pub fn is_public(&self) -> bool {
        matches!(self, Event::Trade(_) | Event::Depth(_) | Event::Ticker(_))
    }

    pub fn user_id(&self) -> Option<UserId> {
        match self {
            Event::Trade(_) | Event::Depth(_) | Event::Ticker(_) => None,
            Event::OrderUpdate(update) => match update {
                UserOrderUpdateEvent::Fill { user_id, .. } => Some(*user_id),
                UserOrderUpdateEvent::Ack { user_id, .. } => Some(*user_id),
                UserOrderUpdateEvent::Reject { user_id, .. } => Some(*user_id),
                UserOrderUpdateEvent::Cancelled { user_id, .. } => Some(*user_id),
            },
        }
    }
}
