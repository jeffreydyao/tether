//! # tether-server
//!
//! HTTP server for the tether phone proximity tracking system.
//!
//! This binary provides:
//! - REST API for proximity checking, pass management, and configuration
//! - OpenAPI documentation via Swagger UI
//! - Structured logging to file and stdout
//!
//! ## Running
//!
//! ```bash
//! # Development
//! cargo run --package tether-server
//!
//! # Production (on Raspberry Pi)
//! ./tether-server
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::net::SocketAddr;

use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

mod api;
mod logging;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    logging::init()?;

    info!("Starting tether-server");

    // Build the application router
    let app = Router::new();
    // TODO: Add routes from api module

    // Bind to address
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr).await?;

    info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
