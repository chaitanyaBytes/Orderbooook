use scylla::client::{session::Session, session_builder::SessionBuilder};

use crate::error::PersistenceError;

pub type Result<T> = std::result::Result<T, PersistenceError>;

pub struct ScyllaDb {
    session: Session,
    keyspace: String,
}

impl ScyllaDb {
    pub async fn new(hostname: &str, keyspace: &str) -> Result<Self> {
        let session = SessionBuilder::new()
            .known_node(format!("{}:{}", hostname, 9042))
            .build()
            .await
            .map_err(|e| PersistenceError::Connection(e.to_string()))?;

        let scylla_db = Self {
            session,
            keyspace: keyspace.to_string(),
        };

        scylla_db.initialize().await?;

        Ok(scylla_db)
    }

    pub async fn initialize(&self) -> Result<()> {
        self.create_keyspace().await?;
        self.create_user_table().await?;
        self.create_order_table().await?;
        self.create_cancel_order_table().await?;
        self.create_trade_table().await?;
        self.create_market_table().await?;
        self.create_ticker_table().await?;
        Ok(())
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn keyspace(&self) -> &str {
        &self.keyspace
    }

    async fn create_keyspace(&self) -> Result<()> {
        let create_keyspace_query = format!(
            r#"
            CREATE KEYSPACE IF NOT EXISTS {}
            WITH REPLICATION = {{
                'class': 'SimpleStrategy',
                'replication_factor': 1
            }}
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_keyspace_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn create_user_table(&self) -> Result<()> {
        let create_user_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.users (
                id bigint PRIMARY KEY,
                balance map<text, text>,
                locked_balance map<text, text>
            )
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_user_table_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn create_order_table(&self) -> Result<()> {
        let create_order_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.orders (
                order_id bigint,
                user_id bigint,
                symbol text,
                side text,
                order_type text,
                price bigint,
                initial_quantity bigint,
                filled_quantity bigint,
                remaining_quantity bigint,
                order_status text,
                timestamp bigint,
                PRIMARY KEY ((symbol), order_id, timestamp)
            ) WITH CLUSTERING ORDER BY (order_id DESC, timestamp DESC)
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_order_table_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn create_cancel_order_table(&self) -> Result<()> {
        let create_cancel_order_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.cancel_orders (
                order_id bigint,
                user_id bigint,
                symbol text,
                reason text,
                timestamp bigint,
                PRIMARY KEY ((symbol), order_id, timestamp)
            ) WITH CLUSTERING ORDER BY (order_id DESC, timestamp DESC)
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_cancel_order_table_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn create_trade_table(&self) -> Result<()> {
        let create_trade_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.trades (
                trade_id bigint,
                symbol text,
                maker_order_id bigint,
                maker_user_id bigint,
                taker_order_id bigint,
                taker_user_id bigint,
                price bigint,
                quantity bigint,
                timestamp bigint,
                PRIMARY KEY ((symbol), trade_id, timestamp)
            ) WITH CLUSTERING ORDER BY (trade_id DESC, timestamp DESC)
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_trade_table_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn create_market_table(&self) -> Result<()> {
        let create_market_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.markets (
                symbol text,
                base text,
                quote text,
                max_price text,
                min_price text,
                tick_size text,
                max_quantity text,
                min_quantity text,
                step_size text,
                PRIMARY KEY (symbol, base, quote)
            )
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_market_table_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }

    async fn create_ticker_table(&self) -> Result<()> {
        let create_ticker_table_query = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.tickers (
                symbol text PRIMARY KEY,
                base_volume bigint,
                quote_volume bigint,
                price_change text,
                price_change_percent text,
                high_price text,
                low_price text,
                last_price text
            )
            "#,
            self.keyspace
        );

        self.session
            .query_unpaged(create_ticker_table_query, &[])
            .await
            .map_err(|e| PersistenceError::Scylla(e.to_string()))?;

        Ok(())
    }
}
