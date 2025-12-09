use r2d2_redis::{RedisConnectionManager, r2d2::Pool, redis::Commands};

use crate::{
    publisher::publisher::Publisher,
    types::{Event, UserOrderUpdateEvent},
};

pub struct RedisPublisher {
    pool: Pool<RedisConnectionManager>,
}

impl RedisPublisher {
    pub fn new(url: &str, pool_size: u32) -> Result<Self, String> {
        let manager = RedisConnectionManager::new(url).map_err(|e| e.to_string())?;
        let pool = Pool::builder()
            .max_size(pool_size)
            .build(manager)
            .map_err(|e| e.to_string())?;

        Ok(Self { pool })
    }

    pub fn channel_for(&self, event: &Event) -> String {
        match event {
            Event::Trade(trade) => format!("market:trade:{}", trade.symbol),
            Event::Depth(depth) => format!("market:depth:{}", depth.symbol),
            Event::Ticker(ticker) => format!("market:ticker:{}", ticker.symbol),
            Event::OrderUpdate(update) => {
                let user_id = match update {
                    UserOrderUpdateEvent::Fill { user_id, .. } => user_id,
                    UserOrderUpdateEvent::Ack { user_id, .. } => user_id,
                    UserOrderUpdateEvent::Reject { user_id, .. } => user_id,
                    UserOrderUpdateEvent::Cancelled { user_id, .. } => user_id,
                };

                format!("market:order:user:{}", user_id)
            }
        }
    }
}

impl Publisher for RedisPublisher {
    fn publish(&self, event: &Event) {
        let channel = self.channel_for(event);
        let message = serde_json::to_string(event).expect("Failed to serialize event");

        match self.pool.get() {
            Ok(mut conn) => {
                if let Err(e) = conn.publish::<_, _, ()>(&channel, &message) {
                    eprintln!("[RedisPublisher] Publish error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[RedisPublisher] Get connection error: {}", e);
            }
        }
    }
}
