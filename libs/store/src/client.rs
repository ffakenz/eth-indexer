use eyre::Result;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub struct Client {
    pool: SqlitePool,
}

impl Client {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new().max_connections(5).connect(database_url).await?;

        Ok(Self { pool })
    }

    pub async fn init(database_url: &str) -> Result<Self> {
        let client = Client::new(database_url).await?;

        // Run migrations or create tables on startup
        sqlx::query(include_str!("../resources/create_tables.sql")).execute(client.pool()).await?;

        Ok(client)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
