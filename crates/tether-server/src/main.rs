//! # tether-server
//!
//! HTTP server for the tether phone proximity tracking system.
//!
//! This binary provides:
//! - REST API for proximity checking, pass management, and configuration
//! - OpenAPI documentation via Swagger UI
//! - Structured logging to file and stdout
//!
//! ## Environment Variables
//!
//! - `TETHER_ENV`: `production` or `development` (default: `production`)
//! - `TETHER_CONFIG_PATH`: Path to config file (default: platform-specific)
//! - `TETHER_LOG_LEVEL`: Log level filter (default: `info`)
//! - `TETHER_HOST`: Bind address (default: `0.0.0.0`)
//! - `TETHER_PORT`: Bind port (default: `8080`)
//!
//! ## Running
//!
//! ```bash
//! # Development
//! TETHER_ENV=development cargo run --package tether-server
//!
//! # Production (on Raspberry Pi)
//! ./tether-server
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use axum::http::{header, Method};
use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{info, warn, Level};

use tether_core::{default_data_dir, Config, PassManager};

mod api;
mod logging;
mod state;

use state::{AppState, SharedState};

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Step 1: Determine environment (production vs development)
    let is_production = env::var("TETHER_ENV")
        .map(|v| v.to_lowercase() != "development")
        .unwrap_or(true);

    // Step 2: Initialize logging/tracing
    logging::init(is_production)?;

    info!(
        env = if is_production { "production" } else { "development" },
        "Starting tether server"
    );

    // Step 3: Load configuration
    let (config_path, passes_path) = resolve_data_paths(is_production);

    info!(config_path = %config_path.display(), "Loading configuration");

    let config = match Config::load(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            if config_path.exists() {
                return Err(anyhow::anyhow!("Failed to load config: {}", e));
            }
            // Create default config if not found
            info!("Config not found, using defaults");
            let cfg = Config::default();
            if let Err(save_err) = cfg.save(&config_path) {
                warn!(error = %save_err, "Could not save default config");
            }
            cfg
        }
    };

    // Step 4: Initialize pass manager
    info!(passes_path = %passes_path.display(), "Loading pass data");
    let passes_per_month = config.passes.per_month;
    let pass_manager = PassManager::load_or_create(&passes_path, passes_per_month.into())?;

    // Step 5: Initialize Bluetooth scanner (optional)
    let bluetooth = init_bluetooth(&config).await;

    // Step 6: Create shared state
    let state = AppState::new(config, pass_manager, bluetooth, config_path, passes_path).into_shared();

    // Step 7: Build the router
    let app = build_router(state, is_production);

    // Step 8: Determine bind address
    let host = env::var("TETHER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("TETHER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    info!(%addr, "Server listening");

    // Step 9: Start server with graceful shutdown
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

// ============================================================================
// Configuration and Data Path Resolution
// ============================================================================

/// Resolves the configuration and data file paths based on environment.
fn resolve_data_paths(is_production: bool) -> (PathBuf, PathBuf) {
    let config_path = env::var("TETHER_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            if is_production {
                PathBuf::from("/etc/tether/config.toml")
            } else {
                // Development: use platform-specific data dir
                let data_dir = default_data_dir();
                data_dir.join("config.toml")
            }
        });

    let passes_path = if is_production {
        PathBuf::from("/var/lib/tether/passes.json")
    } else {
        let data_dir = default_data_dir();
        data_dir.join("passes.json")
    };

    // Ensure parent directories exist
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Some(parent) = passes_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    (config_path, passes_path)
}

// ============================================================================
// Bluetooth Initialization
// ============================================================================

/// Initialize the Bluetooth scanner if available.
///
/// Returns `None` if Bluetooth is not configured or not available.
#[cfg(feature = "bluetooth")]
async fn init_bluetooth(config: &Config) -> Option<tether_core::BluetoothScanner> {
    use tether_core::{BluetoothScanner, BtConfig};

    let bt_config = BtConfig {
        target_address: config.bluetooth.target_address.clone(),
        rssi_threshold: config.bluetooth.rssi_threshold.unwrap_or(-70),
        scan_timeout_secs: config.bluetooth.scan_timeout_secs.unwrap_or(10),
    };

    match BluetoothScanner::new(bt_config).await {
        Ok(scanner) => {
            info!("Bluetooth scanner initialized");
            Some(scanner)
        }
        Err(e) => {
            warn!(error = %e, "Bluetooth scanner not available");
            None
        }
    }
}

#[cfg(not(feature = "bluetooth"))]
async fn init_bluetooth(_config: &Config) -> Option<tether_core::BluetoothScanner> {
    info!("Bluetooth support not compiled in");
    None
}

// ============================================================================
// Router Construction
// ============================================================================

/// Builds the complete application router with all routes and middleware.
///
/// # Middleware Order (bottom to top execution)
///
/// 1. **TraceLayer** (outermost): Logs all requests/responses
/// 2. **CorsLayer** (dev only): Handles CORS preflight and headers
/// 3. Route-specific handlers
fn build_router(state: SharedState, is_production: bool) -> Router {
    // Build the main router with all API routes
    let mut app = api::create_router(state);

    // Apply middleware using ServiceBuilder (executes bottom-to-top)
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // Add CORS layer only in development mode
    if !is_production {
        info!("CORS enabled for development");

        let cors = CorsLayer::new()
            // Allow any origin in development (for Vite dev server)
            .allow_origin(Any)
            // Allow common HTTP methods
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
            // Allow common headers
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
            // Cache preflight requests for 1 hour
            .max_age(Duration::from_secs(3600));

        app = app.layer(cors);
    }

    app.layer(ServiceBuilder::new().layer(trace_layer))
}

// ============================================================================
// Graceful Shutdown
// ============================================================================

/// Creates a future that resolves when a shutdown signal is received.
///
/// Handles both Ctrl+C (SIGINT) and SIGTERM signals on Unix systems.
/// On Windows, only Ctrl+C is handled.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        }
        () = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_data_paths_development() {
        // Test development paths
        let (config_path, passes_path) = resolve_data_paths(false);
        assert!(config_path.to_string_lossy().contains("config.toml"));
        assert!(passes_path.to_string_lossy().contains("passes.json"));
    }

    #[test]
    fn test_resolve_data_paths_production() {
        // Test production paths
        let (config_path, passes_path) = resolve_data_paths(true);
        assert_eq!(config_path, PathBuf::from("/etc/tether/config.toml"));
        assert_eq!(passes_path, PathBuf::from("/var/lib/tether/passes.json"));
    }
}
