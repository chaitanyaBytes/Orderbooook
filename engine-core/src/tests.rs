use crate::engine::Engine;
use crossbeam_channel;
use protocol::{
    CancelOrder, CancelReason, Event, Order, OrderCommand, OrderType, RejectReason, Side,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_valid_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) = crossbeam_channel::unbounded::<OrderCommand>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 10,
            price: Some(50000),
        };

        order_tx.send(OrderCommand::PlaceOrder(order)).unwrap();
        drop(order_tx);

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        match event_rx.recv() {
            Ok(Event::OrderAck(ack)) => {
                assert_eq!(ack.order_id, 1);
                assert_eq!(ack.user_id, 100);
                assert_eq!(ack.symbol, "SOL/USD");
            }
            other => panic!("Expected OrderAck, got {:?}", other),
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_place_invalid_order_zero_quantity() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) = crossbeam_channel::unbounded::<OrderCommand>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let order = Order {
            order_id: 2,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 0,
            price: Some(50000),
        };

        order_tx.send(OrderCommand::PlaceOrder(order)).unwrap();
        drop(order_tx);

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        match event_rx.recv() {
            Ok(Event::OrderReject(reject)) => {
                assert_eq!(reject.order_id, 2);
                assert_eq!(reject.user_id, 100);
                assert!(matches!(reject.reason, RejectReason::InvalidQuantity));
            }
            other => panic!("Expected OrderReject, got {:?}", other),
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_cancel_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) = crossbeam_channel::unbounded::<OrderCommand>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let cancel = CancelOrder {
            order_id: 3,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
        };

        order_tx.send(OrderCommand::CancelOrder(cancel)).unwrap();
        drop(order_tx);

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        match event_rx.recv() {
            Ok(Event::OrderCancelled(cancelled)) => {
                assert_eq!(cancelled.order_id, 3);
                assert_eq!(cancelled.user_id, 100);
                assert!(matches!(cancelled.reason, CancelReason::UserRequested));
            }
            other => panic!("Expected OrderCancelled, got {:?}", other),
        }

        handle.join().unwrap();
    }
}
