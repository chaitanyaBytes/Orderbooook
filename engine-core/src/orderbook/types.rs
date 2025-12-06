use protocol::types::{OrderId, Price, Quantity, Side, UserId};

pub const CACHE_LIMIT: usize = 25;

#[derive(Debug, Clone)]
pub(crate) struct Trade {
    pub(crate) trade_id: u64,
    pub(crate) maker_order_id: OrderId,
    pub(crate) maker_user_id: UserId,
    pub(crate) taker_order_id: OrderId,
    pub(crate) taker_user_id: UserId,
    pub(crate) quantity: Quantity,
    pub(crate) price: Price,
    pub(crate) timestamp: i64,
}

impl Trade {
    #[inline]
    pub(crate) fn into_protocol(self, symbol: &str) -> protocol::types::Trade {
        protocol::types::Trade {
            symbol: symbol.to_string(),
            trade_id: self.trade_id,
            maker_order_id: self.maker_order_id,
            maker_user_id: self.maker_user_id,
            taker_order_id: self.taker_order_id,
            taker_user_id: self.taker_user_id,
            quantity: self.quantity,
            price: self.price,
            timestamp: self.timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Fill {
    pub(crate) order_id: OrderId,
    pub(crate) user_id: UserId,
    pub(crate) side: Side,
    pub(crate) filled_price: Price,
    pub(crate) filled_quantity: Quantity,
    pub(crate) remaining_quantity: Quantity,
}

impl Fill {
    #[inline]
    pub(crate) fn into_protocol(self, symbol: &str) -> protocol::types::Fill {
        protocol::types::Fill {
            order_id: self.order_id,
            user_id: self.user_id,
            symbol: symbol.to_string(),
            side: self.side,
            filled_price: self.filled_price,
            filled_quantity: self.filled_quantity,
            remaining_quantity: self.remaining_quantity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Depth {
    pub(crate) bids: Vec<(Price, Quantity)>,
    pub(crate) asks: Vec<(Price, Quantity)>,
}

impl Depth {
    #[inline]
    pub(crate) fn into_protocol(self, symbol: &str) -> protocol::types::BookUpdate {
        let bids = self
            .bids
            .into_iter()
            .map(|(price, quantity)| protocol::types::PriceLevel { price, quantity })
            .collect();

        let asks = self
            .asks
            .into_iter()
            .map(|(price, quantity)| protocol::types::PriceLevel { price, quantity })
            .collect();

        protocol::types::BookUpdate {
            symbol: symbol.to_string(),
            bids,
            asks,
            last_price: None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct CachedDepth {
    pub(crate) bids: [(Price, Quantity); CACHE_LIMIT],
    pub(crate) asks: [(Price, Quantity); CACHE_LIMIT],
    pub(crate) is_latest: bool,
}

impl CachedDepth {
    pub(crate) fn new() -> Self {
        Self {
            bids: [(0, 0); CACHE_LIMIT],
            asks: [(0, 0); CACHE_LIMIT],
            is_latest: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MatchResult {
    pub(crate) fills: Vec<Fill>,
    pub(crate) trades: Vec<Trade>,
    pub(crate) book_update: Option<Depth>,
}
