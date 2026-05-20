use crate::{error::AppError, error::AppResult};

const INITIAL_SCHEMA: &str = include_str!("../migrations/001_initial.sql");

pub async fn run_migrations_from_env() -> AppResult<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        tracing::warn!("DATABASE_URL is not set; backend will run with in-memory repositories");
        return Ok(());
    };
    run_migrations(&database_url).await
}

pub async fn run_migrations(database_url: &str) -> AppResult<()> {
    let (client, connection) = tokio_postgres::connect(database_url, tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!("postgres connection error: {error}");
        }
    });
    client
        .batch_execute(INITIAL_SCHEMA)
        .await
        .map_err(AppError::from)
}

#[cfg(test)]
pub fn initial_schema() -> &'static str {
    INITIAL_SCHEMA
}
