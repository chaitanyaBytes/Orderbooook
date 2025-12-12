use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::Utc;
use crossbeam_channel::{Receiver, RecvTimeoutError};
use protocol::types::{Event, Fill, OrderAck, OrderCancelled, OrderId, OrderReject, Side, Trade};

use crate::{
    error::PersistenceError,
    models::{CancelOrderRow, OrderRow, TradeRow},
    scylla_db::ScyllaDb,
};

type Result<T> = std::result::Result<T, PersistenceError>;

const BATCH_SIZE: usize = 100;
const BATCH_TIMEOUT_MS: u64 = 100;

pub struct PersistenceWriter {
    db: ScyllaDb,
    order_state: HashMap<OrderId, OrderState>,
}

#[derive(Debug, Clone)]
struct OrderState {
    side: Option<Side>,
    initial_quantity: u64,
    filled_quantity: u64,
}

impl PersistenceWriter {
    pub async fn new(uri: &str, keyspace: &str) -> Result<Self> {
        let db = ScyllaDb::new(uri, keyspace).await?;

        Ok(Self {
            db,
            order_state: HashMap::new(),
        })
    }

    pub async fn run(&mut self, event_rx: Receiver<Event>) {
        println!(
            "[PersistenceWriter] Starting persistence writer with batch writes (size={}, timeout={}ms)...",
            BATCH_SIZE, BATCH_TIMEOUT_MS
        );

        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let mut last_flush = Instant::now();
        let timeout = Duration::from_millis(BATCH_TIMEOUT_MS);

        loop {
            match event_rx.recv_timeout(timeout) {
                Ok(event) => {
                    batch.push(event);

                    if batch.len() >= BATCH_SIZE {
                        if let Err(e) = self.flush_batch(&mut batch).await {
                            eprintln!("[PersistenceWriter] Error flushing batch: {}", e);
                        }
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if !batch.is_empty() && last_flush.elapsed() >= timeout {
                        if let Err(e) = self.flush_batch(&mut batch).await {
                            eprintln!("[PersistenceWriter] Error flushing batch: {}", e);
                        }
                        last_flush = Instant::now();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    if !batch.is_empty() {
                        if let Err(e) = self.flush_batch(&mut batch).await {
                            eprintln!("[PersistenceWriter] Error flushing final batch: {}", e);
                        }
                    }
                    break;
                }
            }
        }

        println!("[PersistenceWriter] Event writer stopped");
    }

    async fn flush_batch(&mut self, batch: &mut Vec<Event>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let batch_size = batch.len();
        let start = Instant::now();

        for event in batch.drain(..) {
            if let Err(e) = self.handle_event(event).await {
                eprintln!("[PersistenceWriter] Error handling event in batch: {}", e);
                // Continue processing other events even if one fails
            }
        }

        let elapsed = start.elapsed();
        println!(
            "[PersistenceWriter] Flushed batch of {} events in {:?} ({:.2} events/sec)",
            batch_size,
            elapsed,
            batch_size as f64 / elapsed.as_secs_f64()
        );

        Ok(())
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::OrderAck(order_ack) => self.persist_order_ack(order_ack).await,
            Event::OrderReject(order_reject) => self.persist_order_reject(order_reject).await,
            Event::Fill(fill) => self.persist_fill(fill).await,
            Event::Trade(trade) => self.persist_trade(trade).await,
            Event::OrderCancelled(order_cancelled) => {
                self.persist_order_cancelled(order_cancelled).await
            }
            Event::BookUpdate(_) => Ok(()), // optional
        }
    }

    async fn persist_order_ack(&mut self, order_ack: OrderAck) -> Result<()> {
        let timestamp = Utc::now().timestamp_millis();

        self.order_state.insert(
            order_ack.order_id,
            OrderState {
                side: None,
                initial_quantity: 0,
                filled_quantity: 0,
            },
        );

        let row = OrderRow::from_ack(
            order_ack.order_id,
            order_ack.user_id,
            order_ack.symbol,
            timestamp,
        );

        let query = format!(
            r#"
            INSERT INTO {}.orders (
                order_id, user_id, symbol, side, order_type, price, initial_quantity,
                filled_quantity, remaining_quantity, order_status, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.db.keyspace()
        );

        self.db
            .session()
            .query_unpaged(
                query,
                (
                    row.order_id as i64,
                    row.user_id as i64,
                    row.symbol,
                    row.side
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    row.order_type
                        .as_ref()
                        .map(|o| format!("{:?}", o))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    row.price.unwrap_or(0) as i64,
                    row.initial_quantity as i64,
                    row.filled_quantity as i64,
                    row.remaining_quantity as i64,
                    row.order_status,
                    row.timestamp,
                ),
            )
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn persist_order_reject(&mut self, order_reject: OrderReject) -> Result<()> {
        let timestamp = Utc::now().timestamp_millis();

        let row = OrderRow::from_reject(
            order_reject.order_id,
            order_reject.user_id,
            order_reject.symbol,
            format!("{:?}", order_reject.reason),
            timestamp,
        );

        let query = format!(
            r#"
            INSERT INTO {}.orders (
                order_id, user_id, symbol, side, order_type, price, initial_quantity,
                filled_quantity, remaining_quantity, order_status, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.db.keyspace()
        );

        self.db
            .session()
            .query_unpaged(
                query,
                (
                    row.order_id as i64,
                    row.user_id as i64,
                    row.symbol,
                    row.side
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    row.order_type
                        .as_ref()
                        .map(|o| format!("{:?}", o))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    row.price.unwrap_or(0) as i64,
                    row.initial_quantity as i64,
                    row.filled_quantity as i64,
                    row.remaining_quantity as i64,
                    row.order_status,
                    row.timestamp,
                ),
            )
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        // Remove state if present
        self.order_state.remove(&order_reject.order_id);

        Ok(())
    }

    async fn persist_fill(&mut self, fill: Fill) -> Result<()> {
        let timestamp = Utc::now().timestamp_millis();

        // Update or initialize state
        let state = self
            .order_state
            .entry(fill.order_id)
            .or_insert_with(|| OrderState {
                side: Some(fill.side.clone()),
                initial_quantity: fill.filled_quantity + fill.remaining_quantity,
                filled_quantity: 0,
            });

        // If we didn't have initial quantity, set it now
        if state.initial_quantity == 0 {
            state.initial_quantity = fill.filled_quantity + fill.remaining_quantity;
        }

        state.side = Some(fill.side.clone());
        state.filled_quantity += fill.filled_quantity;

        let remaining = fill.remaining_quantity;
        let order_status = if remaining == 0 {
            "Filled"
        } else {
            "PartiallyFilled"
        };

        let row = OrderRow::from_fill(
            fill.order_id,
            fill.user_id,
            fill.symbol,
            fill.side.clone(),
            fill.filled_price,
            state.initial_quantity,
            state.filled_quantity,
            remaining,
            order_status.to_string(),
            timestamp,
        );

        let query = format!(
            r#"
            INSERT INTO {}.orders (
                order_id, user_id, symbol, side, order_type, price, initial_quantity,
                filled_quantity, remaining_quantity, order_status, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.db.keyspace()
        );

        self.db
            .session()
            .query_unpaged(
                query,
                (
                    row.order_id as i64,
                    row.user_id as i64,
                    row.symbol,
                    row.side
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    row.order_type
                        .as_ref()
                        .map(|o| format!("{:?}", o))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    row.price.unwrap_or(0) as i64,
                    row.initial_quantity as i64,
                    row.filled_quantity as i64,
                    row.remaining_quantity as i64,
                    row.order_status,
                    row.timestamp,
                ),
            )
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn persist_trade(&mut self, trade: Trade) -> Result<()> {
        let row = TradeRow::new(
            trade.trade_id,
            trade.symbol,
            trade.maker_order_id,
            trade.maker_user_id,
            Some(trade.taker_order_id),
            Some(trade.taker_user_id),
            trade.price,
            trade.quantity,
            trade.timestamp,
        );

        let query = format!(
            r#"
            INSERT INTO {}.trades (
                trade_id, symbol, maker_order_id, maker_user_id, taker_order_id,
                taker_user_id, price, quantity, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.db.keyspace()
        );

        self.db
            .session()
            .query_unpaged(
                query,
                (
                    row.trade_id as i64,
                    row.symbol,
                    row.maker_order_id as i64,
                    row.maker_user_id as i64,
                    row.taker_order_id.unwrap_or(0) as i64,
                    row.taker_user_id.unwrap_or(0) as i64,
                    row.price as i64,
                    row.quantity as i64,
                    row.timestamp,
                ),
            )
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn persist_order_cancelled(&mut self, order_cancelled: OrderCancelled) -> Result<()> {
        let timestamp = Utc::now().timestamp_millis();

        let row = CancelOrderRow::new(
            order_cancelled.order_id,
            order_cancelled.user_id,
            order_cancelled.symbol.clone(),
            format!("{:?}", order_cancelled.reason),
            timestamp,
        );

        let query = format!(
            r#"
            INSERT INTO {}.cancel_orders (
                order_id, user_id, symbol, reason, timestamp
            ) VALUES (?, ?, ?, ?, ?)
            "#,
            self.db.keyspace()
        );

        self.db
            .session()
            .query_unpaged(
                query,
                (
                    row.order_id as i64,
                    row.user_id as i64,
                    row.symbol,
                    row.reason,
                    row.timestamp,
                ),
            )
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        // Remove state for the cancelled order
        self.order_state.remove(&order_cancelled.order_id);

        Ok(())
    }
}
