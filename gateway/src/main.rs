use crossbeam_channel;
use engine_core::engine::Engine;
use market_data::{
    pipeline::MarketDataPipeline, publisher::publisher::Publisher, publisher::redis::RedisPublisher,
};
use net::http::app::HttpServerApp;
use net::http::models::orders::CommandResponse;
use net::ws::app::WsServerApp;
use oneshot;
use persistence::writer::PersistenceWriter;
use protocol::types::{Event, OrderCommand};
use runtime::RUNTIME;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::vec;
fn main() {
    let (order_tx, order_rx) =
        crossbeam_channel::bounded::<(OrderCommand, oneshot::Sender<CommandResponse>)>(1000);
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

    let (market_data_tx, market_data_rx) = crossbeam_channel::unbounded::<Event>();
    let (persistence_tx, persistence_rx) = crossbeam_channel::bounded::<Event>(10_000);

    // Broadcast events from engine to both market data and persistence
    let broadcaster_handle = std::thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            if let Err(e) = market_data_tx.send(event.clone()) {
                eprintln!("[Broadcaster] Market data channel closed: {}", e);
                break;
            }

            if let Err(e) = persistence_tx.send(event) {
                eprintln!("[Broadcaster] Persistence channel closed: {}", e);
                break;
            }
        }
    });

    // Start engine
    let engine_handle = std::thread::spawn(move || {
        let mut engine = Engine::new("SOL_USDC");
        engine.run(order_rx, event_tx);
    });

    // Build and start HTTP server
    let http_server = HttpServerApp::build("127.0.0.1", "8080", order_tx.clone())
        .unwrap_or_else(|e| panic!("Failed to build HTTP server: {}", e));

    // Build and start WebSocket server
    let ws_server = RUNTIME.block_on(async {
        WsServerApp::build("127.0.0.1", "8081")
            .await
            .unwrap_or_else(|e| panic!("Failed to build WS server: {}", e))
    });
    println!("WebSocket server: ws://127.0.0.1:{}", ws_server.port);

    // Build and start Redis publisher
    let redis_pub = RedisPublisher::new("redis://127.0.0.1:6379", 10).expect("redis pool");
    let publishers: Vec<Box<dyn Publisher>> = vec![Box::new(redis_pub)];

    let mut market_data_pipeline = MarketDataPipeline::new(publishers);

    println!("Gateway starting...");
    println!("HTTP server: http://127.0.0.1:{}", http_server.port);
    println!("Engine: Running");

    // Run HTTP server in async runtime
    let http_handle = RUNTIME.spawn(async move {
        if let Err(e) = http_server.run_until_stopped().await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    // Run WebSocket server in async runtime
    let ws_handle = RUNTIME.spawn(async move {
        if let Err(e) = ws_server.run_until_stopped().await {
            eprintln!("WebSocket server error: {}", e);
        }
    });

    // Running market data pipeline to publish data to redis
    let market_data_handle = std::thread::spawn(move || {
        market_data_pipeline.run(market_data_rx);
    });

    // Initialize and run persistence writer
    let persistence_handle = RUNTIME.spawn(async move {
        let mut persistence_writer = PersistenceWriter::new("127.0.0.1", "orderbook")
            .await
            .unwrap_or_else(|e| panic!("Failed to initialize persistence writer: {}", e));
        persistence_writer.run(persistence_rx).await;
    });

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Keep running until shutdown
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    drop(order_tx);

    engine_handle.join().unwrap();
    broadcaster_handle.join().unwrap();
    market_data_handle.join().unwrap();
    RUNTIME.block_on(persistence_handle).unwrap();
    RUNTIME.block_on(http_handle).unwrap();
    RUNTIME.block_on(ws_handle).unwrap();

    println!("Gateway stopped");
}
