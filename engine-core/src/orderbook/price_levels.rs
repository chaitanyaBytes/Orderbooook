use std::collections::VecDeque;

use protocol::{OrderId, Price, Quantity};

#[derive(Debug, Clone, Default)]
pub struct PriceLevel {
    pub(crate) price: Price,
    pub(crate) orders: VecDeque<OrderId>,
    pub(crate) total_quantity: Quantity,
}

impl PriceLevel {
    #[inline]
    pub(crate) fn new(price: Price) -> Self {
        Self {
            price,
            orders: VecDeque::new(),
            total_quantity: 0,
        }
    }

    #[inline]
    pub(crate) fn add_order(&mut self, order_id: OrderId, quantity: Quantity) {
        self.orders.push_back(order_id);
        self.total_quantity = self.total_quantity.saturating_add(quantity);
    }

    #[inline]
    pub(crate) fn remove_order(&mut self, quantity: Quantity) {
        self.total_quantity = self.total_quantity.saturating_sub(quantity);
    }

    #[inline]
    pub(crate) fn get_total_quantity(&self) -> Quantity {
        self.total_quantity
    }

    #[inline]
    pub(crate) fn get_orders(&self) -> &VecDeque<OrderId> {
        &self.orders
    }

    #[inline]
    pub(crate) fn get_price(&self) -> Price {
        self.price
    }

    #[inline]
    pub(crate) fn get_orders_count(&self) -> usize {
        self.orders.len()
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.total_quantity == 0 || self.orders.is_empty()
    }
}
