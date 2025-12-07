use std::{net::TcpListener, sync::atomic::AtomicU64};

use actix_web::{self, App, HttpServer, web};
use crossbeam_channel::Sender;
use oneshot;
use protocol::types::OrderCommand;

use crate::http::{models::orders::CommandResponse, routes::config};

pub struct HttpServerApp {
    pub port: u16,
    pub server: actix_web::dev::Server,
}

pub struct HttpServerAppState {
    pub order_tx: Sender<(OrderCommand, oneshot::Sender<CommandResponse>)>,
    pub order_id: AtomicU64,
}

impl HttpServerApp {
    pub fn build(
        host: &str,
        port: &str,
        order_tx: Sender<(OrderCommand, oneshot::Sender<CommandResponse>)>,
    ) -> Result<Self, std::io::Error> {
        let address = format!("{}:{}", host, port);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        let app_state = web::Data::new(HttpServerAppState {
            order_tx,
            order_id: AtomicU64::new(1),
        });

        let server =
            HttpServer::new(move || App::new().app_data(app_state.clone()).configure(config))
                .listen(listener)
                .unwrap()
                .run();

        Ok(Self { port, server })
    }

    pub fn get_port(self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
