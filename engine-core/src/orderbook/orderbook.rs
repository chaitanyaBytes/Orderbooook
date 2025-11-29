use crate::{error::OrderBookError, orderbook::price_levels::PriceLevel};
use chrono::Utc;
use protocol::{OrderId, Price, Quantity, Side, UserId};
use std::collections::{BTreeMap, HashMap};

const CACHE_LIMIT: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderEntry {
    order_id: OrderId,
    user_id: UserId,
    side: Side,
    price: Price,
    quantity: Quantity,
    remaining_quantity: Quantity,
    timestamp: i64,
}

impl OrderEntry {
    #[inline]
    pub(crate) fn new(
        order_id: OrderId,
        user_id: UserId,
        side: Side,
        price: Price,
        quantity: Quantity,
    ) -> Self {
        Self {
            order_id,
            user_id,
            side,
            price,
            quantity,
            remaining_quantity: quantity,
            timestamp: Utc::now().timestamp_millis(),
        }
    }

    #[inline]
    pub(crate) fn validate_order(&self) -> Result<(), OrderBookError> {
        if self.quantity == 0 {
            return Err(OrderBookError::InvalidOrder(
                "Quantity must be greater than 0".into(),
            ));
        }

        if self.price <= 0 {
            return Err(OrderBookError::InvalidOrder(
                "Price must be greater than 0".into(),
            ));
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn fill(&mut self, quantity: Quantity) -> Result<(), OrderBookError> {
        if quantity > self.remaining_quantity {
            return Err(OrderBookError::InvalidOrder(
                "Quantity must be less than or equal to the remaining quantity".into(),
            ));
        }

        self.remaining_quantity -= quantity;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Trade {
    trade_id: u64,
    maker_order_id: OrderId,
    maker_user_id: UserId,
    taker_order_id: OrderId,
    taker_user_id: UserId,
    symbol: String,
    quantity: Quantity,
    price: Price,
    timestamp: i64,
}

#[derive(Debug)]
pub struct Fill {
    order_id: OrderId,
    user_id: UserId,
    symbol: String,
    side: Side,
    filled_price: Price,
    filled_quantity: Quantity,
    remaining_quantity: Quantity,
}

#[derive(Debug)]
pub struct Depth {
    bids: Vec<(Price, Quantity)>,
    asks: Vec<(Price, Quantity)>,
}

#[derive(Debug)]
pub struct CachedDepth {
    bids: [(Price, Quantity); CACHE_LIMIT],
    asks: [(Price, Quantity); CACHE_LIMIT],
    is_latest: bool,
}

impl CachedDepth {
    pub fn new() -> Self {
        Self {
            bids: [(0, 0); CACHE_LIMIT],
            asks: [(0, 0); CACHE_LIMIT],
            is_latest: false,
        }
    }
}

pub struct OrderBook {
    symbol: String,
    asks: BTreeMap<Price, PriceLevel>,
    bids: BTreeMap<Price, PriceLevel>,

    orders: HashMap<OrderId, OrderEntry>,

    depth_cache: CachedDepth,

    // Track best bid/ask for quick access
    best_bid: Option<Price>,
    best_ask: Option<Price>,
}

impl OrderBook {
    #[inline]
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            orders: HashMap::new(),
            depth_cache: CachedDepth::new(),
            best_bid: None,
            best_ask: None,
        }
    }

    #[inline]
    pub fn add_order(&mut self, order: OrderEntry) -> Result<(), OrderBookError> {
        order.validate_order()?;

        let order_id = order.order_id;
        let price = order.price;
        let side = order.side.clone();
        let quantity = order.quantity;

        self.orders.insert(order_id, order);

        match side {
            Side::Buy => {
                if let Some(level) = self.bids.get_mut(&price) {
                    level.add_order(order_id, quantity);
                } else {
                    let mut new_level = PriceLevel::new(price);
                    new_level.add_order(order_id, quantity);
                    self.bids.insert(price, new_level);
                }

                // update the best bid
                if self.best_bid.is_none() || price > self.best_bid.unwrap_or(0) {
                    self.best_bid = Some(price);
                }
            }
            Side::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.add_order(order_id, quantity);
                } else {
                    let mut new_level = PriceLevel::new(price);
                    new_level.add_order(order_id, quantity);
                    self.asks.insert(price, new_level);
                }

                // update the best ask
                if self.best_ask.is_none() || price < self.best_ask.unwrap_or(0) {
                    self.best_ask = Some(price);
                }
            }
        }

        self.depth_cache.is_latest = false;
        Ok(())
    }

    #[inline]
    pub fn remove_order(
        &mut self,
        order_id: OrderId,
    ) -> Result<Option<OrderEntry>, OrderBookError> {
        let Some(order) = self.orders.remove(&order_id) else {
            return Err(OrderBookError::OrderNotFound(order_id));
        };

        let price = order.price;
        let side = order.side.clone();
        let quantity = order.quantity;

        match side {
            Side::Buy => {
                if let Some(level) = self.bids.get_mut(&price) {
                    level.remove_order(order_id, quantity);

                    if level.is_empty() {
                        self.bids.remove(&price);

                        if self.best_bid == Some(price) {
                            self.best_bid = self.bids.keys().next_back().copied();
                        }
                    }
                }
            }
            Side::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.remove_order(order_id, quantity);

                    if level.is_empty() {
                        self.asks.remove(&price);

                        if self.best_ask == Some(price) {
                            self.best_ask = self.asks.keys().next().copied();
                        }
                    }
                }
            }
        }

        self.depth_cache.is_latest = false;

        Ok(Some(order))
    }

    pub fn get_depth(&mut self, limit: usize) -> Depth {
        if !self.depth_cache.is_latest {
            self.update_depth_cache();
        }

        let bids = self.depth_cache.bids[..self.depth_cache.bids.len().min(limit)].to_vec();
        let asks = self.depth_cache.asks[..self.depth_cache.asks.len().min(limit)].to_vec();

        Depth { bids, asks }
    }

    pub fn update_depth_cache(&mut self) {
        if self.depth_cache.is_latest {
            return;
        }

        let bids = collect_to_fixed_array(
            self.bids
                .iter()
                .rev()
                .map(|(price, level)| (*price, level.get_total_quantity())),
        );

        let asks = collect_to_fixed_array(
            self.asks
                .iter()
                .map(|(price, level)| (*price, level.get_total_quantity())),
        );

        self.depth_cache = CachedDepth {
            bids,
            asks,
            is_latest: true,
        };
    }

    #[inline]
    pub fn best_bid(&self) -> Option<Price> {
        self.best_bid
    }

    #[inline]
    pub fn best_ask(&self) -> Option<Price> {
        self.best_ask
    }

    #[inline]
    pub fn get_order(&self, order_id: OrderId) -> Option<&OrderEntry> {
        self.orders.get(&order_id)
    }

    #[inline]
    pub fn get_order_mut(&mut self, order_id: OrderId) -> Option<&mut OrderEntry> {
        self.orders.get_mut(&order_id)
    }
}

fn collect_to_fixed_array<I>(iter: I) -> [(Price, Quantity); CACHE_LIMIT]
where
    I: Iterator<Item = (Price, Quantity)>,
{
    let mut arr = [(0, 0); CACHE_LIMIT];
    for (i, item) in iter.take(CACHE_LIMIT).enumerate() {
        arr[i] = item;
    }
    arr
}
