//! Tether MCP Server
//!
//! This MCP server connects to a Raspberry Pi running the Tether HTTP server
//! via dumbpipe (iroh-based secure tunnel) and exposes filtered API endpoints
//! as MCP tools for AI agents.
//!
//! # Architecture
//!
//! 1. Read TETHER_DUMBPIPE_TICKET from environment
//! 2. Spawn dumbpipe as a subprocess in connect-tcp mode
//! 3. Wait for dumbpipe to establish connection
//! 4. Proxy API requests through the tunnel
//! 5. Expose MCP tools for proximity and pass management
//!
//! # Environment Variables
//!
//! - `TETHER_DUMBPIPE_TICKET`: Required. The dumbpipe ticket for connecting to the Pi
//! - `TETHER_LOCAL_PORT`: Optional. Local port for dumbpipe tunnel (default: 38080)
//! - `RUST_LOG`: Optional. Logging level (default: info)
//! - `MCP_TRANSPORT`: Optional. "stdio" or "streamable-http" (default: stdio)
//! - `MCP_HTTP_PORT`: Optional. Port for HTTP transport (default: 8080)

use std::net::SocketAddr;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use url::Url;

/// Environment variable names
mod env_vars {
    pub const DUMBPIPE_TICKET: &str = "TETHER_DUMBPIPE_TICKET";
    pub const LOCAL_PORT: &str = "TETHER_LOCAL_PORT";
    pub const MCP_TRANSPORT: &str = "MCP_TRANSPORT";
    pub const MCP_HTTP_PORT: &str = "MCP_HTTP_PORT";
}

/// Default configuration values
mod defaults {
    pub const LOCAL_PORT: u16 = 38080;
    pub const DUMBPIPE_CONNECT_TIMEOUT_SECS: u64 = 30;
    pub const HTTP_PORT: u16 = 8080;
}

/// Custom error types for the MCP server
#[derive(Debug, thiserror::Error)]
pub enum TetherMcpError {
    #[error("TETHER_DUMBPIPE_TICKET environment variable is not set")]
    TicketNotSet,

    #[error("TETHER_DUMBPIPE_TICKET is empty or invalid: {0}")]
    InvalidTicket(String),

    #[error("Failed to spawn dumbpipe subprocess: {0}")]
    DumbpipeSpawnFailed(#[source] std::io::Error),

    #[error("Dumbpipe failed to connect within {0} seconds")]
    DumbpipeConnectionTimeout(u64),

    #[error("Dumbpipe process exited unexpectedly: {0}")]
    DumbpipeExitedUnexpectedly(String),
}

/// Configuration for the MCP server
#[derive(Debug, Clone)]
pub struct Config {
    /// The dumbpipe ticket for connecting to the Raspberry Pi
    pub dumbpipe_ticket: String,

    /// Local port for the dumbpipe tunnel
    pub local_port: u16,

    /// Transport mode: "stdio" or "streamable-http"
    pub transport_mode: TransportMode,

    /// Port for HTTP transport
    pub http_port: u16,
}

/// Transport mode for the MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// Standard input/output - for local use with Claude Desktop
    Stdio,
    /// HTTP/SSE - for Cloud Run deployment
    StreamableHttp,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, TetherMcpError> {
        let dumbpipe_ticket = std::env::var(env_vars::DUMBPIPE_TICKET)
            .map_err(|_| TetherMcpError::TicketNotSet)?;

        if dumbpipe_ticket.trim().is_empty() {
            return Err(TetherMcpError::InvalidTicket("Ticket is empty".to_string()));
        }

        let local_port = std::env::var(env_vars::LOCAL_PORT)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults::LOCAL_PORT);

        let transport_mode = match std::env::var(env_vars::MCP_TRANSPORT)
            .unwrap_or_else(|_| "stdio".to_string())
            .to_lowercase()
            .as_str()
        {
            "http" | "streamable-http" | "streamable" => TransportMode::StreamableHttp,
            _ => TransportMode::Stdio,
        };

        let http_port = std::env::var(env_vars::MCP_HTTP_PORT)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults::HTTP_PORT);

        Ok(Self {
            dumbpipe_ticket,
            local_port,
            transport_mode,
            http_port,
        })
    }
}

/// Manages the dumbpipe subprocess lifecycle
pub struct DumbpipeManager {
    child: Child,
    local_addr: SocketAddr,
}

impl DumbpipeManager {
    /// Spawn dumbpipe in connect-tcp mode and wait for it to be ready
    pub async fn spawn_and_wait(ticket: &str, local_port: u16) -> Result<Self, TetherMcpError> {
        info!("Spawning dumbpipe connect-tcp on port {}", local_port);

        let local_addr: SocketAddr = format!("127.0.0.1:{local_port}")
            .parse()
            .expect("Valid socket address");

        let mut child = Command::new("dumbpipe")
            .arg("connect-tcp")
            .arg("--addr")
            .arg(local_addr.to_string())
            .arg(ticket)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(TetherMcpError::DumbpipeSpawnFailed)?;

        let stderr = child
            .stderr
            .take()
            .expect("stderr was configured as piped");

        let ready = Self::wait_for_ready(stderr, defaults::DUMBPIPE_CONNECT_TIMEOUT_SECS).await?;

        if !ready {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(TetherMcpError::DumbpipeExitedUnexpectedly(format!(
                    "Exit status: {status}"
                )));
            }
            return Err(TetherMcpError::DumbpipeConnectionTimeout(
                defaults::DUMBPIPE_CONNECT_TIMEOUT_SECS,
            ));
        }

        info!(
            "Dumbpipe connected successfully, tunnel available at {}",
            local_addr
        );

        Ok(Self { child, local_addr })
    }

    async fn wait_for_ready(
        stderr: tokio::process::ChildStderr,
        timeout_secs: u64,
    ) -> Result<bool, TetherMcpError> {
        let mut reader = BufReader::new(stderr).lines();

        let wait_future = async {
            while let Ok(Some(line)) = reader.next_line().await {
                debug!("dumbpipe: {}", line);

                if line.contains("listening")
                    || line.contains("connected")
                    || line.contains("forwarding")
                    || line.contains("Listening")
                    || line.contains("Connected")
                {
                    return true;
                }

                if line.contains("error") || line.contains("Error") || line.contains("failed") {
                    error!("Dumbpipe error: {}", line);
                    return false;
                }
            }
            false
        };

        match timeout(Duration::from_secs(timeout_secs), wait_future).await {
            Ok(result) => Ok(result),
            Err(_) => {
                warn!("Timeout waiting for dumbpipe ready signal, proceeding anyway...");
                Ok(true)
            }
        }
    }

    pub fn base_url(&self) -> Url {
        Url::parse(&format!("http://{}", self.local_addr)).expect("Valid URL")
    }

    pub async fn shutdown(mut self) -> Result<()> {
        info!("Shutting down dumbpipe...");
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        info!("Dumbpipe shutdown complete");
        Ok(())
    }
}

// API response types
#[derive(Debug, Serialize, Deserialize)]
pub struct ProximityResponse {
    pub device_name: Option<String>,
    pub is_nearby: bool,
    pub rssi: Option<i16>,
    pub threshold: i16,
    pub last_seen: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassesResponse {
    pub remaining: u32,
    pub total: u32,
    pub month: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassHistoryEntry {
    pub used_at: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassHistoryResponse {
    pub month: String,
    pub entries: Vec<PassHistoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsePassRequest {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsePassResponse {
    pub success: bool,
    pub remaining: u32,
    pub message: String,
}

/// HTTP client for talking to the Tether server through the dumbpipe tunnel
#[derive(Clone)]
pub struct TetherClient {
    client: reqwest::Client,
    base_url: Url,
}

impl TetherClient {
    pub fn new(base_url: Url) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    pub async fn get_proximity(&self) -> Result<ProximityResponse> {
        let url = self.base_url.join("/api/proximity")?;
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch proximity")?;
        resp.json().await.context("Failed to parse proximity response")
    }

    pub async fn get_passes(&self) -> Result<PassesResponse> {
        let url = self.base_url.join("/api/passes")?;
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch passes")?;
        resp.json().await.context("Failed to parse passes response")
    }

    pub async fn get_pass_history(&self, month: Option<&str>) -> Result<PassHistoryResponse> {
        let mut url = self.base_url.join("/api/passes/history")?;
        if let Some(m) = month {
            url.set_query(Some(&format!("month={m}")));
        }
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch pass history")?;
        resp.json()
            .await
            .context("Failed to parse pass history response")
    }

    pub async fn use_pass(&self, reason: &str) -> Result<UsePassResponse> {
        let url = self.base_url.join("/api/passes/use")?;
        let resp = self
            .client
            .post(url)
            .json(&UsePassRequest {
                reason: reason.to_string(),
            })
            .send()
            .await
            .context("Failed to use pass")?;
        resp.json().await.context("Failed to parse use pass response")
    }
}

// Tool parameter types
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetPassHistoryArgs {
    /// Month to query in YYYY-MM format (e.g., '2025-01'). Defaults to current month if not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UsePassArgs {
    /// The reason for using the pass (e.g., 'Early flight tomorrow', 'On-call for work')
    pub reason: String,
}

/// The MCP server handler for Tether
#[derive(Clone)]
pub struct TetherMcpServer {
    client: TetherClient,
    tool_router: ToolRouter<TetherMcpServer>,
}

#[tool_router]
impl TetherMcpServer {
    pub fn new(client: TetherClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    /// Check if the tracked phone is near the Raspberry Pi
    #[tool(description = "Check if the tracked phone is currently near the Raspberry Pi based on Bluetooth signal strength. Returns whether the phone is nearby along with signal strength information.")]
    async fn get_proximity(&self) -> Result<CallToolResult, McpError> {
        match self.client.get_proximity().await {
            Ok(resp) => {
                let status = if resp.is_nearby { "nearby" } else { "not nearby" };
                let rssi_info = resp
                    .rssi
                    .map(|r| format!(" (signal: {r} dBm)"))
                    .unwrap_or_default();
                let device = resp
                    .device_name
                    .as_deref()
                    .unwrap_or("configured device");

                let text = format!(
                    "Phone ({device}) is {status}{rssi_info}. Threshold: {} dBm.",
                    resp.threshold
                );

                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to check proximity: {e}"
            ))])),
        }
    }

    /// Get the number of remaining passes for the current month
    #[tool(description = "Get the number of remaining emergency passes for the current month. These passes allow keeping the phone nearby on exceptional nights.")]
    async fn get_passes_remaining(&self) -> Result<CallToolResult, McpError> {
        match self.client.get_passes().await {
            Ok(resp) => {
                let text = format!(
                    "Passes remaining for {}: {}/{} passes available.",
                    resp.month, resp.remaining, resp.total
                );

                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get passes: {e}"
            ))])),
        }
    }

    /// Get the history of pass usage
    #[tool(description = "Get the history of emergency pass usage for a specific month or the current month. Shows when passes were used and the reasons provided.")]
    async fn get_pass_history(
        &self,
        Parameters(args): Parameters<GetPassHistoryArgs>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.get_pass_history(args.month.as_deref()).await {
            Ok(resp) => {
                if resp.entries.is_empty() {
                    let text = format!("No passes used in {}.", resp.month);
                    return Ok(CallToolResult::success(vec![Content::text(text)]));
                }

                let mut text = format!("Pass history for {}:\n", resp.month);
                for entry in &resp.entries {
                    text.push_str(&format!("- {}: {}\n", entry.used_at, entry.reason));
                }

                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to get pass history: {e}"
            ))])),
        }
    }

    /// Use an emergency pass
    #[tool(description = "Use an emergency pass for tonight. This allows keeping the phone nearby for one night. Requires a reason explaining why the pass is needed. Use sparingly as passes are limited each month.")]
    async fn use_pass(&self, Parameters(args): Parameters<UsePassArgs>) -> Result<CallToolResult, McpError> {
        if args.reason.trim().is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                "A reason is required to use a pass.",
            )]));
        }

        match self.client.use_pass(&args.reason).await {
            Ok(resp) => {
                let text = if resp.success {
                    format!(
                        "Pass used successfully. {}. You have {} passes remaining.",
                        resp.message, resp.remaining
                    )
                } else {
                    format!("Could not use pass: {}", resp.message)
                };

                Ok(CallToolResult::success(vec![Content::text(text)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to use pass: {e}"
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for TetherMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "tether-mcp".to_string(),
                title: Some("Tether MCP Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/jeffrey/tether".to_string()),
            },
            instructions: Some(
                "Tether MCP Server - Monitor phone proximity and manage emergency passes. \
                \n\nTools available:\
                \n- get_proximity: Check if the phone is near the Raspberry Pi\
                \n- get_passes_remaining: See how many emergency passes are left this month\
                \n- get_pass_history: Review past pass usage\
                \n- use_pass: Use an emergency pass when needed (requires a reason)"
                    .to_string(),
            ),
        }
    }
}

/// Setup signal handlers for graceful shutdown
fn setup_signal_handlers() -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");
            let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");

            tokio::select! {
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating shutdown...");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating shutdown...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("Ctrl+C handler");
            info!("Received Ctrl+C, initiating shutdown...");
        }

        let _ = tx.send(());
    });

    rx
}

/// Initialize logging
fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info")
            .add_directive("tether_mcp=debug".parse().unwrap())
            .add_directive("rmcp=info".parse().unwrap())
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .with_writer(std::io::stderr),
        )
        .init();
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    info!("Tether MCP Server starting...");

    let config = Config::from_env().context("Failed to load configuration")?;

    info!(
        "Configuration: port={}, transport={:?}",
        config.local_port, config.transport_mode
    );

    let shutdown_rx = setup_signal_handlers();

    // Spawn dumbpipe and wait for connection
    let dumbpipe = DumbpipeManager::spawn_and_wait(&config.dumbpipe_ticket, config.local_port)
        .await
        .context("Failed to establish dumbpipe connection")?;

    let base_url = dumbpipe.base_url();
    info!("Dumbpipe tunnel established at {}", base_url);

    let client = TetherClient::new(base_url);
    let mcp_server = TetherMcpServer::new(client);

    match config.transport_mode {
        TransportMode::Stdio => {
            info!("Starting MCP server with stdio transport");

            let transport = rmcp::transport::stdio();

            tokio::select! {
                result = mcp_server.serve(transport) => {
                    match result {
                        Ok(ct) => {
                            info!("MCP server running, waiting for completion...");
                            if let Err(e) = ct.waiting().await {
                                error!("MCP server error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to start MCP server: {}", e);
                        }
                    }
                }
                _ = shutdown_rx => {
                    info!("Shutdown signal received");
                }
            }
        }
        TransportMode::StreamableHttp => {
            // For HTTP transport, we run the same stdio server
            // The expectation is that HTTP transport will be handled by a reverse proxy
            // or by running this behind an HTTP-to-stdio adapter like mcp-proxy
            warn!(
                "HTTP transport requested on port {} - falling back to stdio. \
                Use an HTTP-to-stdio proxy for HTTP support.",
                config.http_port
            );

            let transport = rmcp::transport::stdio();

            tokio::select! {
                result = mcp_server.serve(transport) => {
                    match result {
                        Ok(ct) => {
                            info!("MCP server running, waiting for completion...");
                            if let Err(e) = ct.waiting().await {
                                error!("MCP server error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to start MCP server: {}", e);
                        }
                    }
                }
                _ = shutdown_rx => {
                    info!("Shutdown signal received");
                }
            }
        }
    }

    info!("Cleaning up...");
    dumbpipe.shutdown().await.context("Failed to shutdown dumbpipe")?;

    info!("Tether MCP Server shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_ticket_not_set() {
        std::env::remove_var(env_vars::DUMBPIPE_TICKET);
        let result = Config::from_env();
        assert!(matches!(result, Err(TetherMcpError::TicketNotSet)));
    }

    #[test]
    fn test_config_empty_ticket() {
        std::env::set_var(env_vars::DUMBPIPE_TICKET, "");
        let result = Config::from_env();
        assert!(matches!(result, Err(TetherMcpError::InvalidTicket(_))));
        std::env::remove_var(env_vars::DUMBPIPE_TICKET);
    }

    #[test]
    fn test_config_valid() {
        std::env::set_var(env_vars::DUMBPIPE_TICKET, "endpoint12345");
        std::env::set_var(env_vars::LOCAL_PORT, "9999");
        let config = Config::from_env().unwrap();
        assert_eq!(config.local_port, 9999);
        assert_eq!(config.dumbpipe_ticket, "endpoint12345");
        std::env::remove_var(env_vars::DUMBPIPE_TICKET);
        std::env::remove_var(env_vars::LOCAL_PORT);
    }
}
