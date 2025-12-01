use crate::{
    error::OrderBookError,
    orderbook::{
        price_levels::PriceLevel,
        types::{CACHE_LIMIT, CachedDepth, Depth, Fill, MatchResult, Trade},
    },
};
use chrono::Utc;
use protocol::{OrderId, Price, Quantity, Side, UserId};
use std::collections::{BTreeMap, HashMap, HashSet};

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

        self.remaining_quantity = self.remaining_quantity.saturating_sub(quantity);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    symbol: String,
    asks: BTreeMap<Price, PriceLevel>,
    bids: BTreeMap<Price, PriceLevel>,

    orders: HashMap<OrderId, OrderEntry>,

    depth_cache: CachedDepth,

    // Track best bid/ask for quick access
    // best_bid: Option<Price>,
    // best_ask: Option<Price>,
    next_trade_id: u64,
}

impl OrderBook {
    #[inline]
    pub(crate) fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            orders: HashMap::new(),
            depth_cache: CachedDepth::new(),
            // best_bid: None,
            // best_ask: None,
            next_trade_id: 1,
        }
    }

    pub(crate) fn match_market_order(
        &mut self,
        order: &mut OrderEntry,
    ) -> Result<MatchResult, OrderBookError> {
        order.validate_order()?;

        let mut fills = Vec::<Fill>::new();
        let mut trades = Vec::<Trade>::new();
    }

    pub(crate) fn match_limit_order(
        &mut self,
        taker_order: &mut OrderEntry,
    ) -> Result<MatchResult, OrderBookError> {
        taker_order.validate_order()?;

        let mut fills = Vec::<Fill>::new();
        let mut trades = Vec::<Trade>::new();

        let is_buy_order = matches!(taker_order.side, Side::Buy);

        let mut prices_to_remove = HashSet::<Price>::new();

        let levels = if is_buy_order {
            &mut self.asks
        } else {
            &mut self.bids
        };

        let price_keys: Vec<Price> = if is_buy_order {
            levels
                .range(..=taker_order.price)
                .map(|(price, _)| *price)
                .collect::<Vec<Price>>()
        } else {
            levels
                .range(taker_order.price..)
                .rev()
                .map(|(price, _)| *price)
                .collect::<Vec<Price>>()
        };

        let mut book_changed = false;

        for price in price_keys {
            if taker_order.remaining_quantity == 0 {
                break;
            }

            let Some(level) = levels.get_mut(&price) else {
                continue;
            };

            // Before matching, lazily pop any dead/cancelled orders at front
            // loop {
            //     if let Some(&front_id) = level.orders.front() {
            //         if let Some(front_entry) = self.orders.get(&front_id) {
            //             if front_entry.remaining_quantity == 0 {
            //                 level.orders.pop_front();
            //                 continue;
            //             }
            //         } else {
            //             level.orders.pop_front();
            //             continue;
            //         }
            //     }
            //     break;
            // }

            while taker_order.remaining_quantity > 0 && !level.is_empty() {
                let Some(&maker_order_id) = level.orders.front() else {
                    continue;
                };

                let maker_order = match self.orders.get_mut(&maker_order_id) {
                    Some(m) => m,
                    None => {
                        level.orders.pop_front();
                        continue;
                    }
                };

                let fill_quantity = maker_order
                    .remaining_quantity
                    .min(taker_order.remaining_quantity);

                maker_order.fill(fill_quantity)?;
                taker_order.fill(fill_quantity)?;
                level.remove_order(fill_quantity);

                fills.push(Fill {
                    order_id: maker_order_id,
                    user_id: maker_order.user_id,
                    side: maker_order.side.clone(),
                    filled_price: price,
                    filled_quantity: fill_quantity,
                    remaining_quantity: maker_order.remaining_quantity,
                });

                fills.push(Fill {
                    order_id: taker_order.order_id,
                    user_id: taker_order.user_id,
                    side: taker_order.side.clone(),
                    filled_price: price,
                    filled_quantity: fill_quantity,
                    remaining_quantity: taker_order.remaining_quantity,
                });

                trades.push(Trade {
                    trade_id: self.next_trade_id,
                    maker_order_id: maker_order_id,
                    maker_user_id: maker_order.user_id,
                    taker_order_id: taker_order.order_id,
                    taker_user_id: taker_order.user_id,
                    quantity: fill_quantity,
                    price: price,
                    timestamp: Utc::now().timestamp_millis(),
                });

                self.next_trade_id = self.next_trade_id.saturating_add(1);
                book_changed = true;

                if maker_order.remaining_quantity == 0 {
                    level.orders.pop_front();
                    self.orders.remove(&maker_order_id);

                    if level.is_empty() {
                        prices_to_remove.insert(price);
                    }
                }

                if taker_order.remaining_quantity == 0 {
                    break;
                }
            }
        }

        for price in &prices_to_remove {
            levels.remove(price);
        }

        if taker_order.remaining_quantity > 0 {
            self.add_order(taker_order.clone())?;
            book_changed = true;
        }

        if book_changed {
            self.depth_cache.is_latest = false;
        }

        // produce book_update optionally (you can build full Depth or top-N)
        let book_update = if book_changed {
            Some(self.get_depth(20)) // convert Depth->BookUpdate in engine later
        } else {
            None
        };

        Ok(MatchResult {
            fills,
            trades,
            book_update: book_update,
        })
    }

    #[inline]
    pub(crate) fn add_order(&mut self, order: OrderEntry) -> Result<(), OrderBookError> {
        order.validate_order()?;

        let order_id = order.order_id;
        let price = order.price;
        let side = order.side.clone();
        let quantity = order.remaining_quantity;

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
            }
            Side::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.add_order(order_id, quantity);
                } else {
                    let mut new_level = PriceLevel::new(price);
                    new_level.add_order(order_id, quantity);
                    self.asks.insert(price, new_level);
                }
            }
        }

        self.depth_cache.is_latest = false;
        Ok(())
    }

    #[inline]
    pub(crate) fn remove_order(&mut self, order_id: OrderId) -> Result<OrderEntry, OrderBookError> {
        let Some(order) = self.orders.remove(&order_id) else {
            return Err(OrderBookError::OrderNotFound(order_id));
        };

        let price = order.price;
        let side = order.side.clone();
        let remaining_quantity = order.remaining_quantity;

        match side {
            Side::Buy => {
                if let Some(level) = self.bids.get_mut(&price) {
                    level.remove_order(remaining_quantity);

                    if level.is_empty() {
                        self.bids.remove(&price);
                    }
                }
            }
            Side::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.remove_order(remaining_quantity);

                    if level.is_empty() {
                        self.asks.remove(&price);
                    }
                }
            }
        }

        self.depth_cache.is_latest = false;

        Ok(order)
    }

    pub(crate) fn get_depth(&mut self, limit: usize) -> Depth {
        if !self.depth_cache.is_latest {
            self.update_depth_cache();
        }

        let bids = self.depth_cache.bids[..self.depth_cache.bids.len().min(limit)].to_vec();
        let asks = self.depth_cache.asks[..self.depth_cache.asks.len().min(limit)].to_vec();

        Depth { bids, asks }
    }

    pub(crate) fn update_depth_cache(&mut self) {
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
    pub(crate) fn get_best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    #[inline]
    pub(crate) fn get_best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    #[inline]
    pub(crate) fn get_order(&self, order_id: OrderId) -> Option<&OrderEntry> {
        self.orders.get(&order_id)
    }

    #[inline]
    pub(crate) fn get_order_mut(&mut self, order_id: OrderId) -> Option<&mut OrderEntry> {
        self.orders.get_mut(&order_id)
    }

    #[inline(always)]
    pub(crate) fn get_symbol(&self) -> &str {
        &self.symbol
    }

    #[inline]
    pub(crate) fn get_next_matchable_order(&self, side: Side) -> Option<OrderId> {
        match side {
            Side::Buy => self.get_best_ask().and_then(|price| {
                self.asks
                    .get(&price)
                    .and_then(|level| level.orders.front().copied())
            }),
            Side::Sell => self.get_best_bid().and_then(|price| {
                self.bids
                    .get(&price)
                    .and_then(|level| level.orders.front().copied())
            }),
        }
    }

    pub(crate) fn update_order_quantity(&mut self, order_id: OrderId, filled_qty: Quantity) {
        if let Some(order) = self.orders.get_mut(&order_id) {
            let remaining_before_fill = order.remaining_quantity;

            if let Err(e) = order.fill(filled_qty) {
                eprintln!("[OrderBook] Failed to fill order: {}", e);
                return;
            }

            let price = order.price;
            match order.side {
                Side::Buy => {
                    if let Some(level) = self.bids.get_mut(&price) {
                        if remaining_before_fill == filled_qty {
                            level.remove_order(filled_qty);
                        } else {
                            level.total_quantity -= filled_qty;
                        }
                    }
                }
                Side::Sell => {
                    if let Some(level) = self.asks.get_mut(&price) {
                        if remaining_before_fill == filled_qty {
                            level.remove_order(filled_qty);
                        } else {
                            level.total_quantity -= filled_qty;
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn collect_to_fixed_array<I>(iter: I) -> [(Price, Quantity); CACHE_LIMIT]
where
    I: Iterator<Item = (Price, Quantity)>,
{
    let mut arr = [(0, 0); CACHE_LIMIT];
    for (i, item) in iter.take(CACHE_LIMIT).enumerate() {
        arr[i] = item;
    }
    arr
}
