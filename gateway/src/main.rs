use crossbeam_channel;
use engine_core::engine::Engine;
use net::http::app::HttpServerApp;
use net::http::models::orders::OrderResponse;
use oneshot;
use protocol::types::{Event, OrderCommand};
use runtime::RUNTIME;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() {
    let (order_tx, order_rx) =
        crossbeam_channel::bounded::<(OrderCommand, oneshot::Sender<OrderResponse>)>(1000);
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

    // Start engine
    let engine_handle = std::thread::spawn(move || {
        let mut engine = Engine::new("SOL/USD");
        engine.run(order_rx, event_tx);
    });

    // Build and start HTTP server
    let http_server = HttpServerApp::build("127.0.0.1", "8080", order_tx.clone())
        .unwrap_or_else(|e| panic!("Failed to build HTTP server: {}", e));

    println!("Gateway starting...");
    println!("HTTP server: http://127.0.0.1:{}", http_server.port);
    println!("Engine: Running");

    // Run HTTP server in async runtime
    let http_handle = RUNTIME.spawn(async move {
        if let Err(e) = http_server.run_until_stopped().await {
            eprintln!("HTTP server error: {}", e);
        }
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
    RUNTIME.block_on(http_handle).unwrap();

    println!("Gateway stopped");
}
