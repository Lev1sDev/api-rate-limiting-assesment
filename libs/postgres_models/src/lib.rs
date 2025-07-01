pub mod models;
pub mod schema;

use bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use std::time::Duration;

pub type DbPool = Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;
pub type DbConnection = bb8::PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database pool error: {0}")]
    Pool(#[from] bb8::RunError<diesel::ConnectionError>),
    
    #[error("Database query error: {0}")]
    Query(#[from] diesel::result::Error),
    
    #[error("Connection error: {0}")]
    Connection(String),
}

pub async fn create_pool(database_url: &str) -> Result<DbPool, DbError> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    
    Pool::builder()
        .max_size(20)
        .min_idle(Some(5))
        .connection_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(600)))
        .test_on_check_out(true)
        .build(config)
        .await
        .map_err(|e| DbError::Connection(e.to_string()))
}