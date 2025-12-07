use crate::engine::Engine;
use crossbeam_channel;
use net::http::models::orders::{CommandResponse, OrderResponse};
use oneshot;
use protocol::types::{
    CancelOrder, CancelReason, Event, Order, OrderCommand, OrderType, RejectReason, Side,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn send_order_and_get_response(
        order_tx: &crossbeam_channel::Sender<(OrderCommand, oneshot::Sender<CommandResponse>)>,
        order: Order,
    ) -> CommandResponse {
        let (reply_tx, reply_rx) = oneshot::channel();
        order_tx
            .send((OrderCommand::PlaceOrder(order), reply_tx))
            .unwrap();
        std::thread::spawn(move || runtime::RUNTIME.block_on(reply_rx))
            .join()
            .unwrap()
            .unwrap()
    }

    fn send_cancel_and_get_response(
        order_tx: &crossbeam_channel::Sender<(OrderCommand, oneshot::Sender<CommandResponse>)>,
        cancel: CancelOrder,
    ) -> CommandResponse {
        let (reply_tx, reply_rx) = oneshot::channel();
        order_tx
            .send((OrderCommand::CancelOrder(cancel), reply_tx))
            .unwrap();
        std::thread::spawn(move || runtime::RUNTIME.block_on(reply_rx))
            .join()
            .unwrap()
            .unwrap()
    }

    #[test]
    fn test_place_valid_limit_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        let order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 10,
            price: Some(50000),
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        order_tx
            .send((OrderCommand::PlaceOrder(order), reply_tx))
            .unwrap();

        // Wait for response from oneshot channel
        let response = std::thread::spawn(move || runtime::RUNTIME.block_on(reply_rx))
            .join()
            .unwrap()
            .unwrap();

        match response {
            CommandResponse::PlaceOrder(OrderResponse::Ack {
                order_id,
                user_id,
                symbol,
            }) => {
                assert_eq!(order_id, 1);
                assert_eq!(user_id, 100);
                assert_eq!(symbol, "SOL/USD");
            }
            CommandResponse::PlaceOrder(OrderResponse::Reject { .. }) => {
                panic!("Expected Ack, got Reject");
            }
            _ => panic!("Expected PlaceOrder, got {:?}", response),
        }

        // Also check event channel
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderAck(ack)) => {
                assert_eq!(ack.order_id, 1);
            }
            other => panic!("Expected OrderAck event, got {:?}", other),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_place_valid_market_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, _event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        let order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 10,
            price: None,
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        order_tx
            .send((OrderCommand::PlaceOrder(order), reply_tx))
            .unwrap();

        let response = std::thread::spawn(move || runtime::RUNTIME.block_on(reply_rx))
            .join()
            .unwrap()
            .unwrap();

        match response {
            CommandResponse::PlaceOrder(OrderResponse::Ack { order_id, .. }) => {
                assert_eq!(order_id, 1);
            }
            CommandResponse::PlaceOrder(OrderResponse::Reject { .. }) => {
                panic!("Expected Ack, got Reject")
            }
            _ => panic!("Expected PlaceOrder, got {:?}", response),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_reject_zero_quantity() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, _event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        let order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 0,
            price: Some(50000),
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        order_tx
            .send((OrderCommand::PlaceOrder(order), reply_tx))
            .unwrap();

        let response = std::thread::spawn(move || runtime::RUNTIME.block_on(reply_rx))
            .join()
            .unwrap()
            .unwrap();

        match response {
            CommandResponse::PlaceOrder(OrderResponse::Reject {
                order_id, reason, ..
            }) => {
                assert_eq!(order_id, 1);
                assert!(matches!(reason, RejectReason::InvalidQuantity));
            }
            CommandResponse::PlaceOrder(OrderResponse::Ack { .. }) => {
                panic!("Expected Reject, got Ack")
            }
            _ => panic!("Expected PlaceOrder, got {:?}", response),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_reject_limit_order_without_price() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, _event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        let order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 10,
            price: None, // Missing price for limit order
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        order_tx
            .send((OrderCommand::PlaceOrder(order), reply_tx))
            .unwrap();

        let response = std::thread::spawn(move || runtime::RUNTIME.block_on(reply_rx))
            .join()
            .unwrap()
            .unwrap();

        match response {
            CommandResponse::PlaceOrder(OrderResponse::Reject {
                order_id, reason, ..
            }) => {
                assert_eq!(order_id, 1);
                assert!(matches!(reason, RejectReason::InvalidOrder));
            }
            CommandResponse::PlaceOrder(OrderResponse::Ack { .. }) => {
                panic!("Expected Reject, got Ack")
            }
            _ => panic!("Expected PlaceOrder, got {:?}", response),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_reject_limit_order_with_zero_price() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, _event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        let order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 10,
            price: Some(0), // Zero price
        };

        let response = send_order_and_get_response(&order_tx, order);
        match response {
            CommandResponse::PlaceOrder(OrderResponse::Reject {
                order_id, reason, ..
            }) => {
                assert_eq!(order_id, 1);
                assert!(matches!(reason, RejectReason::InvalidOrder));
            }
            CommandResponse::PlaceOrder(OrderResponse::Ack { .. }) => {
                panic!("Expected Reject, got Ack")
            }
            _ => panic!("Expected PlaceOrder, got {:?}", response),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_buy_limit_matches_sell_limit_full_fill() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place sell order (maker)
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack event

        // Place buy order that matches (taker)
        let buy_order = Order {
            order_id: 2,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect events: OrderAck, Fill (maker), Fill (taker), Trade
        let mut events = Vec::new();
        for _ in 0..5 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        let mut has_ack = false;
        let mut fill_count = 0;
        let mut has_trade = false;

        for event in events {
            match event {
                Event::OrderAck(ack) => {
                    assert_eq!(ack.order_id, 2);
                    has_ack = true;
                }
                Event::Fill(fill) => {
                    fill_count += 1;
                    assert_eq!(fill.filled_quantity, 50);
                    assert_eq!(fill.filled_price, 50000);
                    if fill.order_id == 1 {
                        assert_eq!(fill.remaining_quantity, 0); // Maker fully filled
                    } else if fill.order_id == 2 {
                        assert_eq!(fill.remaining_quantity, 0); // Taker fully filled
                    }
                }
                Event::Trade(trade) => {
                    assert_eq!(trade.quantity, 50);
                    assert_eq!(trade.price, 50000);
                    assert_eq!(trade.maker_order_id, 1);
                    assert_eq!(trade.taker_order_id, 2);
                    has_trade = true;
                }
                _ => {}
            }
        }

        assert!(has_ack, "Should have OrderAck");
        assert_eq!(fill_count, 2, "Should have 2 fills");
        assert!(has_trade, "Should have Trade event");

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_sell_limit_matches_buy_limit_full_fill() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place buy order (maker)
        let buy_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack

        // Place sell order that matches (taker)
        let sell_order = Order {
            order_id: 2,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);

        // Collect events
        let mut events = Vec::new();
        for _ in 0..5 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        let mut has_trade = false;
        for event in events {
            if let Event::Trade(trade) = event {
                assert_eq!(trade.maker_order_id, 1);
                assert_eq!(trade.taker_order_id, 2);
                assert_eq!(trade.quantity, 50);
                has_trade = true;
            }
        }

        assert!(has_trade, "Should have Trade event");

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_partial_fill_maker_remaining() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place sell order for 100 units (maker)
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 100,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack

        // Place buy order for 30 units (taker, partial fill)
        let buy_order = Order {
            order_id: 2,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 30,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect events
        let mut events = Vec::new();
        for _ in 0..6 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        let mut maker_fill = None;
        let mut taker_fill = None;

        for event in events {
            match event {
                Event::Fill(fill) => {
                    if fill.order_id == 1 {
                        maker_fill = Some(fill);
                    } else if fill.order_id == 2 {
                        taker_fill = Some(fill);
                    }
                }
                _ => {}
            }
        }

        assert!(maker_fill.is_some(), "Should have maker fill");
        assert!(taker_fill.is_some(), "Should have taker fill");

        let maker = maker_fill.unwrap();
        assert_eq!(maker.filled_quantity, 30);
        assert_eq!(maker.remaining_quantity, 70); // 100 - 30

        let taker = taker_fill.unwrap();
        assert_eq!(taker.filled_quantity, 30);
        assert_eq!(taker.remaining_quantity, 0); // Fully filled

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_partial_fill_taker_remaining() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place sell order for 30 units (maker)
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 30,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack

        // Place buy order for 100 units (taker, will partially fill)
        let buy_order = Order {
            order_id: 2,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 100,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect events
        let mut events = Vec::new();
        for _ in 0..4 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        let mut taker_fill = None;

        for event in events {
            if let Event::Fill(fill) = event {
                if fill.order_id == 2 {
                    taker_fill = Some(fill);
                }
            }
        }

        assert!(taker_fill.is_some(), "Should have taker fill");
        let taker = taker_fill.unwrap();
        assert_eq!(taker.filled_quantity, 30);
        assert_eq!(taker.remaining_quantity, 70); // 100 - 30, taker partially filled

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_market_order_matches_limit_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place limit sell order (maker)
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack

        // Place market buy order (taker)
        let market_buy = Order {
            order_id: 2,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 30,
            price: None,
        };
        let _ = send_order_and_get_response(&order_tx, market_buy);

        // Collect events
        let mut events = Vec::new();
        for _ in 0..6 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        let mut has_trade = false;
        for event in events {
            if let Event::Trade(trade) = event {
                assert_eq!(trade.quantity, 30);
                assert_eq!(trade.price, 50000); // Matched at maker's price
                assert_eq!(trade.maker_order_id, 1);
                assert_eq!(trade.taker_order_id, 2);
                has_trade = true;
            }
        }

        assert!(has_trade, "Market order should have matched");

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_market_order_no_liquidity() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place market buy order with no liquidity
        let market_buy = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 30,
            price: None,
        };
        let _ = send_order_and_get_response(&order_tx, market_buy);

        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderAck(ack)) => {
                assert_eq!(ack.order_id, 1);
                assert_eq!(ack.user_id, 100);
                assert_eq!(ack.symbol, "SOL/USD");
            }
            other => panic!("Expected OrderAck, got {:?}", other),
        }

        // Should get OrderReject (no liquidity available)
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderReject(reject)) => {
                assert_eq!(reject.order_id, 1);
                assert_eq!(reject.user_id, 100);
                assert!(matches!(reject.reason, RejectReason::InvalidOrder));
                assert!(
                    reject.message.contains("liquidity") || reject.message.contains("No liquidity")
                );
            }
            other => panic!("Expected OrderReject, got {:?}", other),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_order_rests_on_book() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place sell order at high price (won't match)
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(60000), // High price
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);

        // Should only get OrderAck, no fills
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderAck(ack)) => {
                assert_eq!(ack.order_id, 1);
            }
            other => panic!("Expected OrderAck, got {:?}", other),
        }

        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::BookUpdate(b)) => {
                assert_eq!(b.asks[0].price, 60000);
                assert_eq!(b.asks[0].quantity, 50);
            }
            other => panic!("Expected OrderAck, got {:?}", other),
        }

        // No additional events
        match event_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(_) => panic!("Should not receive any additional events"),
            Err(_) => {} // Timeout expected
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_fifo_priority_same_price() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place 3 sell orders at same price (should match in order 1, 2, 3)
        for i in 1..=3 {
            let sell_order = Order {
                order_id: i,
                user_id: 100,
                symbol: "SOL/USD".to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                quantity: 10,
                price: Some(50000),
            };
            let _ = send_order_and_get_response(&order_tx, sell_order);
            let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack
        }

        // Place buy order that will match all 3
        let buy_order = Order {
            order_id: 4,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 30,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect all events
        let mut events = Vec::new();
        loop {
            match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }

        // Extract trades
        let trades: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Trade(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(trades.len(), 3, "Should have 3 trades");
        assert_eq!(trades[0].maker_order_id, 1, "First match should be order 1");
        assert_eq!(
            trades[1].maker_order_id, 2,
            "Second match should be order 2"
        );
        assert_eq!(trades[2].maker_order_id, 3, "Third match should be order 3");

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_price_priority_best_price_first() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place sell orders at different prices
        // Higher price first (worse)
        let sell1 = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 10,
            price: Some(51000), // Higher price
        };
        let _ = send_order_and_get_response(&order_tx, sell1);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1));

        // Lower price second (better)
        let sell2 = Order {
            order_id: 2,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 10,
            price: Some(50000), // Lower price (better for buyer)
        };
        let _ = send_order_and_get_response(&order_tx, sell2);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1));

        // Buy order that can match both
        let buy_order = Order {
            order_id: 3,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 10,
            price: Some(52000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect events
        let mut events = Vec::new();
        for _ in 0..7 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        // Should match at best price (50000, not 51000)
        let trades: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Trade(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(trades.len(), 1);
        assert_eq!(
            trades[0].price, 50000,
            "Should match at best (lowest) ask price"
        );
        assert_eq!(
            trades[0].maker_order_id, 2,
            "Should match order 2 (better price)"
        );

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_cancel_resting_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place an order that will rest on book
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 50,
            price: Some(60000), // High price, won't match
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack

        // Cancel it
        let cancel = CancelOrder {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
        };
        let _ = send_cancel_and_get_response(&order_tx, cancel);

        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::BookUpdate(b)) => {
                assert_eq!(b.asks[0].price, 60000);
                assert_eq!(b.asks[0].quantity, 50);
            }
            other => panic!("Expected BookUpdate, got {:?}", other),
        }

        // Should get OrderCancelled
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderCancelled(cancelled)) => {
                assert_eq!(cancelled.order_id, 1);
                assert_eq!(cancelled.user_id, 100);
                assert!(matches!(cancelled.reason, CancelReason::UserRequested));
            }
            other => panic!("Expected OrderCancelled, got {:?}", other),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_cancel_non_existent_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Try to cancel an order that doesn't exist
        let cancel = CancelOrder {
            order_id: 999,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
        };
        let _ = send_cancel_and_get_response(&order_tx, cancel);

        // Should get OrderReject
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderReject(reject)) => {
                assert_eq!(reject.order_id, 999);
                assert_eq!(reject.user_id, 100);
                assert!(matches!(reject.reason, RejectReason::InvalidOrder));
            }
            other => panic!("Expected OrderReject, got {:?}", other),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_cancel_partially_filled_order() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place sell order for 100 units
        let sell_order = Order {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            quantity: 100,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, sell_order);
        let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack

        // Place buy order that partially fills it
        let buy_order = Order {
            order_id: 2,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 30,
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Consume fill events
        let mut events = Vec::new();
        for _ in 0..6 {
            if let Ok(event) = event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                events.push(event);
            }
        }

        // Now cancel the partially filled order
        let cancel = CancelOrder {
            order_id: 1,
            user_id: 100,
            symbol: "SOL/USD".to_string(),
        };
        let _ = send_cancel_and_get_response(&order_tx, cancel);

        // Should get OrderCancelled
        match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(Event::OrderCancelled(cancelled)) => {
                assert_eq!(cancelled.order_id, 1);
                assert_eq!(cancelled.user_id, 100);
            }
            other => panic!("Expected OrderCancelled, got {:?}", other),
        }

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_single_order_matches_multiple_orders() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place 3 sell orders at same price
        for i in 1..=3 {
            let sell_order = Order {
                order_id: i,
                user_id: 100,
                symbol: "SOL/USD".to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                quantity: 20,
                price: Some(50000),
            };
            let _ = send_order_and_get_response(&order_tx, sell_order);
            let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack
        }

        // Place one large buy order that matches all 3
        let buy_order = Order {
            order_id: 4,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 60, // Matches all 3
            price: Some(50000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect all events
        let mut events = Vec::new();
        loop {
            match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }

        // Should have 3 trades (one per maker)
        let trades: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Trade(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(trades.len(), 3, "Should have 3 trades");

        // Should have 4 fills (3 makers + 1 taker with 3 partial fills)
        let fills: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Fill(f) = e {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(fills.len(), 6, "Should have 6 fills (3 makers + 3 taker)");

        drop(order_tx);
        handle.join().unwrap();
    }

    #[test]
    fn test_single_order_matches_multiple_levels() {
        let mut engine = Engine::new("SOL/USD");
        let (order_tx, order_rx) =
            crossbeam_channel::unbounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

        let handle = std::thread::spawn(move || {
            engine.run(order_rx, event_tx);
        });

        // Place 3 sell orders at same price
        for i in 1..=3 {
            let sell_order = Order {
                order_id: i,
                user_id: 100,
                symbol: "SOL/USD".to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                quantity: 20,
                price: Some(50000 + i as u64 * 1000),
            };
            let _ = send_order_and_get_response(&order_tx, sell_order);
            let _ = event_rx.recv_timeout(std::time::Duration::from_secs(1)); // Ack
        }

        // Place one large buy order that matches all 3
        let buy_order = Order {
            order_id: 4,
            user_id: 200,
            symbol: "SOL/USD".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            quantity: 80, // Matches all 3
            price: Some(53000),
        };
        let _ = send_order_and_get_response(&order_tx, buy_order);

        // Collect all events
        let mut events = Vec::new();
        loop {
            match event_rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }

        // Should have 3 trades (one per maker)
        let trades: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Trade(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(trades.len(), 3, "Should have 3 trades");

        // Should have 4 fills (3 makers + 1 taker with 3 partial fills)
        let fills: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Fill(f) = e {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(fills.len(), 6, "Should have 6 fills (3 makers + 3 taker)");

        drop(order_tx);
        handle.join().unwrap();
    }
}
