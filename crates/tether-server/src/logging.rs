//! Logging initialization and configuration.

use std::path::PathBuf;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the logging system.
///
/// Logs are written to both stdout and a rolling file.
pub fn init() -> anyhow::Result<()> {
    let log_dir = log_directory();

    // Create rolling file appender
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "tether.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Build subscriber with multiple layers
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .json(),
        )
        .init();

    // Store guard to keep file writer alive
    // Note: In production, store _guard in application state
    std::mem::forget(_guard);

    Ok(())
}

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
