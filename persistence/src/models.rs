use protocol::types::{OrderId, OrderType, Price, Quantity, Side, Symbol, TradeId, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    id: UserId,
    balance: HashMap<Symbol, Quantity>,
    locked_balance: HashMap<Symbol, Quantity>,
}

impl User {
    pub fn new(
        id: UserId,
        balance: HashMap<Symbol, Quantity>,
        locked_balance: HashMap<Symbol, Quantity>,
    ) -> Self {
        Self {
            id,
            balance,
            locked_balance,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderRow {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub symbol: Symbol,
    pub side: Option<Side>,
    pub order_type: Option<OrderType>,
    pub price: Option<Price>,
    pub quantity: Option<Quantity>,
    pub initial_quantity: Quantity,
    pub filled_quantity: Quantity,
    pub remaining_quantity: Quantity,
    pub order_status: String,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CancelOrderRow {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub symbol: Symbol,
    pub reason: String,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TradeRow {
    pub trade_id: TradeId,
    pub symbol: Symbol,
    pub maker_order_id: OrderId,
    pub maker_user_id: UserId,
    pub taker_order_id: Option<OrderId>,
    pub taker_user_id: Option<UserId>,
    pub price: Price,
    pub quantity: Quantity,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MarketRow {
    pub symbol: Symbol,
    pub base: Symbol,
    pub quote: Symbol,
    pub max_price: Price,
    pub min_price: Price,
    pub tick_size: Price,
    pub max_quantity: Quantity,
    pub min_quantity: Quantity,
    pub step_size: Quantity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TickerRow {
    pub symbol: Symbol,
    pub base_volume: Quantity,
    pub quote_volume: Quantity,
    pub price_change: Price,
    pub price_change_percent: Price,
    pub high_price: Price,
    pub low_price: Price,
    pub last_price: Price,
}

impl TickerRow {
    pub fn new(
        symbol: Symbol,
        base_volume: Quantity,
        quote_volume: Quantity,
        price_change: Price,
        price_change_percent: Price,
        high_price: Price,
        low_price: Price,
        last_price: Price,
    ) -> Self {
        Self {
            symbol,
            base_volume,
            quote_volume,
            price_change,
            price_change_percent,
            high_price,
            low_price,
            last_price,
        }
    }
}

impl MarketRow {
    pub fn new(
        symbol: Symbol,
        base: Symbol,
        quote: Symbol,
        max_price: Price,
        min_price: Price,
        tick_size: Price,
        max_quantity: Quantity,
        min_quantity: Quantity,
        step_size: Quantity,
    ) -> Self {
        Self {
            symbol,
            base,
            quote,
            max_price,
            min_price,
            tick_size,
            max_quantity,
            min_quantity,
            step_size,
        }
    }
}

impl TradeRow {
    pub fn new(
        trade_id: TradeId,
        symbol: Symbol,
        maker_order_id: OrderId,
        maker_user_id: UserId,
        taker_order_id: Option<OrderId>,
        taker_user_id: Option<UserId>,
        price: Price,
        quantity: Quantity,
        timestamp: i64,
    ) -> Self {
        Self {
            trade_id,
            symbol,
            maker_order_id,
            maker_user_id,
            taker_order_id,
            taker_user_id,
            price,
            quantity,
            timestamp,
        }
    }
}

impl CancelOrderRow {
    pub fn new(
        order_id: OrderId,
        user_id: UserId,
        symbol: Symbol,
        reason: String,
        timestamp: i64,
    ) -> Self {
        Self {
            order_id,
            user_id,
            symbol,
            reason,
            timestamp,
        }
    }
}

impl OrderRow {
    pub fn from_ack(order_id: OrderId, user_id: UserId, symbol: Symbol, timestamp: i64) -> Self {
        Self {
            order_id,
            user_id,
            symbol,
            side: None,
            order_type: None,
            price: None,
            quantity: None,
            initial_quantity: 0,
            filled_quantity: 0,
            remaining_quantity: 0,
            order_status: "Pending".to_string(),
            timestamp,
        }
    }

    pub fn from_reject(
        order_id: OrderId,
        user_id: UserId,
        symbol: Symbol,
        reason: String,
        timestamp: i64,
    ) -> Self {
        Self {
            order_id,
            user_id,
            symbol,
            side: None,
            order_type: None,
            price: None,
            quantity: None,
            initial_quantity: 0,
            filled_quantity: 0,
            remaining_quantity: 0,
            order_status: reason,
            timestamp,
        }
    }

    pub fn from_fill(
        order_id: OrderId,
        user_id: UserId,
        symbol: Symbol,
        side: Side,
        price: Price,
        initial_quantity: Quantity,
        filled_quantity: Quantity,
        remaining_quantity: Quantity,
        status: String,
        timestamp: i64,
    ) -> Self {
        Self {
            order_id,
            user_id,
            symbol,
            side: Some(side),
            order_type: Some(OrderType::Limit),
            price: Some(price),
            quantity: Some(initial_quantity),
            initial_quantity,
            filled_quantity,
            remaining_quantity,
            order_status: status,
            timestamp,
        }
    }
}
