use crossbeam_channel;
use engine_core::engine::Engine;
use protocol::{Event, Order, OrderCommand, OrderType, Side};
use runtime::RUNTIME;

fn main() {
    let (order_tx, order_rx) = crossbeam_channel::bounded::<OrderCommand>(1000);
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

    let engine_handle = std::thread::spawn(move || {
        let mut engine = Engine::new();
        engine.run(order_rx, event_tx);
    });

    for i in 1..=5 {
        let order = Order {
            order_id: i,
            user_id: 1,
            symbol: "ETH/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 10 * i,
            price: Some(3000),
        };

        order_tx.send(OrderCommand::PlaceOrder(order)).unwrap();
    }

    for i in 1..=5 {
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderAck(ack)) => {
                assert_eq!(ack.order_id, i);
            }
            Ok(other) => panic!("Expected OrderAck for order {}, got {:?}", i, other),
            Err(e) => panic!("Failed to receive event for order {}: {}", i, e),
        }
    }

    drop(order_tx);
    engine_handle.join().unwrap();
}
