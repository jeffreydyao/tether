//! Logging initialization and configuration.
//!
//! This module provides environment-aware logging setup:
//! - **Production**: JSON logs to rolling files + compact logs to stdout
//! - **Development**: Pretty logs to stdout with span events

use std::path::PathBuf;
use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Static guards to keep non-blocking file writers alive.
/// These must persist for the lifetime of the program.
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static STDOUT_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Initialize the logging system with environment-appropriate configuration.
///
/// # Arguments
///
/// * `is_production` - Whether to use production logging configuration
///
/// # Production Mode
///
/// - Logs to rolling daily files in `/var/log/tether/`
/// - Also logs to stdout for systemd journal capture
/// - JSON format for structured logging in files
/// - Compact format for stdout (no ANSI colors)
///
/// # Development Mode
///
/// - Logs to stdout only with pretty formatting
/// - Includes span events for debugging
/// - ANSI colors enabled
///
/// # Errors
///
/// Returns an error if the env filter cannot be parsed.
pub fn init(is_production: bool) -> anyhow::Result<()> {
    let log_level = std::env::var("TETHER_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

    let env_filter =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(&log_level))?;

    if is_production {
        init_production(env_filter)?;
    } else {
        init_development(env_filter);
    }

    Ok(())
}

/// Initialize production logging with file + stdout output.
fn init_production(env_filter: EnvFilter) -> anyhow::Result<()> {
    let log_dir = log_directory();

    // Ensure log directory exists
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).ok();
    }

    // Rolling file appender - creates new file daily
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "tether");

    // Non-blocking writer for file output
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);

    // Non-blocking writer for stdout
    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

    // File layer - JSON format for structured logging
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking_file)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Stdout layer - compact format for journald
    let stdout_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(non_blocking_stdout)
        .with_target(true)
        .with_ansi(false); // No ANSI colors for journald

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    // Store guards to prevent dropping (keeps file writer alive)
    let _ = FILE_GUARD.set(file_guard);
    let _ = STDOUT_GUARD.set(stdout_guard);

    Ok(())
}

/// Initialize development logging with pretty stdout output.
fn init_development(env_filter: EnvFilter) {
    let stdout_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .init();
}

/// Returns the appropriate log directory for the current platform.
fn log_directory() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/var/log/tether")
    }
    #[cfg(not(target_os = "linux"))]
    {
        directories::ProjectDirs::from("", "", "tether")
            .map(|dirs| dirs.data_dir().join("logs"))
            .unwrap_or_else(|| PathBuf::from("./logs"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_directory_is_valid_path() {
        let dir = log_directory();
        // Just verify it returns a non-empty path
        assert!(!dir.as_os_str().is_empty());
    }
}
