use crossbeam_channel::{Receiver, Sender};
use oneshot;
use protocol::types::{
    CancelOrder, CancelReason, Event, Order, OrderAck, OrderCancelled, OrderCommand, OrderReject,
    OrderType, RejectReason,
};

use crate::orderbook::orderbook::{OrderBook, OrderEntry};
use net::http::models::orders::OrderResponse;

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

    pub fn run(
        &mut self,
        order_rx: Receiver<(OrderCommand, oneshot::Sender<OrderResponse>)>,
        event_tx: Sender<Event>,
    ) {
        println!("[Engine] Starting matching engine...");

        loop {
            match order_rx.recv() {
                Ok((order_command, reply_tx)) => match order_command {
                    OrderCommand::PlaceOrder(order) => {
                        println!("[Engine] Placing order: {order:?}");
                        self.handle_place_order(order, reply_tx, &event_tx);
                    }
                    OrderCommand::CancelOrder(cancel_order) => {
                        println!("[Engine] Cancelling order: {cancel_order:?}");
                        self.handle_cancel_order(cancel_order, reply_tx, &event_tx);
                    }
                },
                Err(e) => {
                    println!("[Engine] Error receiving order command: {e}");
                    break;
                }
            }
        }

        println!("[Engine] Engine shutting down");
    }

    fn handle_place_order(
        &mut self,
        order: Order,
        reply_tx: oneshot::Sender<OrderResponse>,
        event_tx: &Sender<Event>,
    ) {
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

            if let Err(e) = reply_tx.send(OrderResponse::Reject {
                order_id: order.order_id,
                reason: RejectReason::InvalidQuantity,
                message: "Quantity must be greater than 0".to_string(),
            }) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };

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

            if let Err(e) = reply_tx.send(OrderResponse::Reject {
                order_id: order.order_id,
                reason: RejectReason::InvalidOrder,
                message: "Price is required for limit orders".to_string(),
            }) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };

            if let Err(e) = event_tx.send(reject) {
                eprintln!("[Engine] Failed to send event: {}", e);
            };
            return;
        }

        let ack = Event::OrderAck(OrderAck {
            order_id: order.order_id,
            user_id: order.user_id,
            symbol: order.symbol.clone(),
        });

        if let Err(e) = reply_tx.send(OrderResponse::Ack {
            order_id: order.order_id,
            user_id: order.user_id,
            symbol: order.symbol,
        }) {
            eprintln!("[Engine] Failed to send event: {}", e);
            return;
        }

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

        println!("[Engine] Order processed: {:?}", order.order_id);
        return;
    }

    fn handle_cancel_order(
        &mut self,
        cancel_order: CancelOrder,
        reply_tx: oneshot::Sender<OrderResponse>,
        event_tx: &Sender<Event>,
    ) {
        println!(
            "[Engine] Cancelling order: {} from user {}",
            cancel_order.order_id, cancel_order.user_id
        );

        if let Err(e) = reply_tx.send(OrderResponse::Ack {
            order_id: cancel_order.order_id,
            user_id: cancel_order.user_id,
            symbol: cancel_order.symbol,
        }) {
            eprintln!("[Engine] Failed to send event: {}", e);
            return;
        }

        let cancelled_order = match self.orderbook.remove_order(cancel_order.order_id) {
            Ok(order) => order,
            Err(e) => {
                eprintln!("[Engine] Failed to remove order: {}", e);

                let reject = Event::OrderReject(OrderReject {
                    order_id: cancel_order.order_id,
                    user_id: cancel_order.user_id,
                    reason: RejectReason::InvalidOrder,
                    message: e.to_string(),
                });

                if let Err(e) = event_tx.send(reject) {
                    eprintln!("[Engine] Failed to send event: {}", e);
                }
                return;
            }
        };

        let cancelled = Event::OrderCancelled(OrderCancelled {
            order_id: cancelled_order.order_id,
            user_id: cancelled_order.user_id,
            symbol: self.orderbook.get_symbol().to_string(),
            reason: CancelReason::UserRequested,
        });

        if let Err(e) = event_tx.send(cancelled) {
            eprintln!("[Engine] Failed to send event: {}", e);
        }

        let depth = self.orderbook.get_depth(20);
        let event = Event::BookUpdate(depth.into_protocol(self.orderbook.get_symbol()));
        if let Err(e) = event_tx.send(event) {
            eprintln!("[Engine] Failed to send event: {}", e);
        };
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new("SOL/USD")
    }
}
