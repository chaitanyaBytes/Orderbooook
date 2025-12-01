use crossbeam_channel::{Receiver, Sender};
use protocol::{
    BookUpdate, CancelOrder, CancelReason, Event, Fill, Order, OrderAck, OrderCancelled,
    OrderCommand, OrderReject, OrderType, PriceLevel, RejectReason, Trade,
};

use crate::orderbook::orderbook::{OrderBook, OrderEntry};

/// synchronous matching engine
/// runs in a dedicated thread, no async, deteministic, locks free
///
/// The engine is responsible for:
/// - Receiving events from the client
/// - Matching orders
/// - Sending events to the client
/// - Handling errors
/// - Logging
/// - Metrics

#[derive(Debug, Clone)]
pub struct Engine {
    orderbook: OrderBook,
}

impl Engine {
    pub fn new(symbol: &str) -> Self {
        Self {
            orderbook: OrderBook::new(symbol),
        }
    }

    pub fn run(&mut self, order_rx: Receiver<OrderCommand>, event_tx: Sender<Event>) {
        println!("[Engine] Starting matching engine...");

        loop {
            match order_rx.recv() {
                Ok(OrderCommand::PlaceOrder(order)) => {
                    println!("[Engine] Placing order: {order:?}");
                    self.handle_place_order(order, &event_tx);
                }
                Ok(OrderCommand::CancelOrder(cancel_order)) => {
                    println!("[Engine] Cancelling order: {cancel_order:?}");
                    self.handle_cancel_order(cancel_order, &event_tx);
                }
                Err(e) => {
                    println!("[Engine] Error receiving order command: {e}");
                    break;
                }
            }
        }

        println!("[Engine] Engine shutting down");
    }

    fn handle_place_order(&mut self, order: Order, event_tx: &Sender<Event>) {
        println!(
            "[Engine] Processing order: {} from user {}",
            order.order_id, order.user_id
        );

        if order.quantity <= 0 {
            let reject = Event::OrderReject(OrderReject {
                order_id: order.order_id,
                user_id: order.user_id,
                reason: RejectReason::InvalidQuantity,
                message: "Quantity must be greater than 0".to_string(),
            });

            if let Err(e) = event_tx.send(reject) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };
            return;
        }

        if order.order_type == OrderType::Limit && order.price.unwrap_or(0) == 0 {
            let reject = Event::OrderReject(OrderReject {
                order_id: order.order_id,
                user_id: order.user_id,
                reason: RejectReason::InvalidOrder,
                message: "Price is required for limit orders".to_string(),
            });

            if let Err(e) = event_tx.send(reject) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };
            return;
        }

        let ack = Event::OrderAck(OrderAck {
            order_id: order.order_id,
            user_id: order.user_id,
            symbol: order.symbol,
        });

        if let Err(e) = event_tx.send(ack) {
            eprintln!("[Engine] Failed to send event: {}", e);
            return;
        }

        let mut order_entry = OrderEntry::new(
            order.order_id,
            order.user_id,
            order.side,
            order.price.unwrap_or(0),
            order.quantity,
        );

        let result = match order.order_type {
            OrderType::Market => self.orderbook.match_market_order(&mut order_entry),
            OrderType::Limit => self.orderbook.match_limit_order(&mut order_entry),
        };

        let result = match result {
            Ok(r) => r,
            Err(e) => {
                let reject = Event::OrderReject(OrderReject {
                    order_id: order.order_id,
                    user_id: order.user_id,
                    reason: RejectReason::InvalidOrder,
                    message: e.to_string(),
                });

                if let Err(e) = event_tx.send(reject) {
                    eprintln!("[Engine] Failed to send event: {}", e);
                };
                return;
            }
        };

        let symbol = self.orderbook.get_symbol();

        for fill in result.fills {
            let event = Event::Fill(fill.into_protocol(symbol));
            if let Err(e) = event_tx.send(event) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };
        }

        for trade in result.trades {
            let event = Event::Trade(trade.into_protocol(symbol));
            if let Err(e) = event_tx.send(event) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };
        }

        if let Some(depth) = result.book_update {
            let event = Event::BookUpdate(depth.into_protocol(symbol));
            if let Err(e) = event_tx.send(event) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };
        }

        return;
    }

    fn handle_cancel_order(&self, cancel_order: CancelOrder, event_tx: &Sender<Event>) {
        println!(
            "[Engine] Cancelling order: {} from user {}",
            cancel_order.order_id, cancel_order.user_id
        );

        let cancelled = Event::OrderCancelled(OrderCancelled {
            order_id: cancel_order.order_id,
            user_id: cancel_order.user_id,
            symbol: cancel_order.symbol,
            reason: CancelReason::UserRequested,
        });

        if let Err(e) = event_tx.send(cancelled) {
            eprint!("[Engine] Failed to send event: {}", e);
        };

        return;

        // TODO: cancel order in the orderbook later
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new("SOL/USD")
    }
}
