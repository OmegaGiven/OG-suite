mod db;
mod error;
mod models;
mod repository;
mod routes;
mod state;
mod transcription;

use crate::{db::run_migrations_from_env, routes::router, state::AppState};
use axum::{response::Redirect, routing::get};
use std::{net::SocketAddr, path::PathBuf};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    run_migrations_from_env().await?;

    let bind = std::env::var("OG_SUITE_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let addr: SocketAddr = bind.parse()?;
    let mut app = router(AppState::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());
    if let Ok(notes_static_dir) = std::env::var("OG_SUITE_NOTES_STATIC_DIR") {
        let notes_static_dir = PathBuf::from(notes_static_dir);
        let index = notes_static_dir.join("index.html");
        app = app
            .route("/notes", get(|| async { Redirect::permanent("/notes/") }))
            .nest_service(
                "/notes/",
                ServeDir::new(notes_static_dir).not_found_service(ServeFile::new(index)),
            );
    }
    if let Ok(static_dir) = std::env::var("OG_SUITE_STATIC_DIR") {
        let static_dir = PathBuf::from(static_dir);
        let index = static_dir.join("index.html");
        app = app
            .fallback_service(ServeDir::new(static_dir).not_found_service(ServeFile::new(index)));
    }

    tracing::info!("OG Suite backend listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
