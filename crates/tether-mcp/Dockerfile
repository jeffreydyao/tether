# =============================================================================
# TETHER MCP Server - Multi-stage Docker Build
# =============================================================================
# This Dockerfile builds the tether-mcp server for deployment to Cloud Run
# or other container platforms.
#
# Features:
# - Multi-stage build for minimal image size
# - Rust nightly for edition 2024 support
# - dumbpipe binary included for P2P connectivity
# - Runs as non-root user for security
#
# Build:
#   docker build -t tether-mcp -f crates/tether-mcp/Dockerfile .
#
# Run:
#   docker run -e TETHER_DUMBPIPE_TICKET=<ticket> tether-mcp
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build the Rust binary
# -----------------------------------------------------------------------------
FROM rust:latest AS builder

# Install nightly toolchain
RUN rustup default nightly && rustup update nightly

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/tether-mcp ./crates/tether-mcp

# Create dummy workspace members to satisfy Cargo
RUN mkdir -p crates/tether-core/src crates/tether-server/src && \
    echo 'fn main() {}' > crates/tether-core/src/lib.rs && \
    echo 'fn main() {}' > crates/tether-server/src/main.rs

# Copy the Cargo.toml files for workspace members
COPY crates/tether-core/Cargo.toml crates/tether-core/
COPY crates/tether-server/Cargo.toml crates/tether-server/

# Build dependencies first (for better caching)
RUN cargo build --release --package tether-mcp 2>/dev/null || true

# Now copy actual source and rebuild
COPY crates/tether-mcp ./crates/tether-mcp

# Build the release binary
RUN cargo build --release --package tether-mcp

# Verify the binary exists
RUN ls -la /build/target/release/tether-mcp

# -----------------------------------------------------------------------------
# Stage 2: Install dumbpipe
# -----------------------------------------------------------------------------
FROM rust:latest AS dumbpipe-builder

RUN cargo install dumbpipe

# -----------------------------------------------------------------------------
# Stage 3: Final minimal image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --user-group tether

# Copy binaries
COPY --from=builder /build/target/release/tether-mcp /usr/local/bin/
COPY --from=dumbpipe-builder /usr/local/cargo/bin/dumbpipe /usr/local/bin/

# Ensure binaries are executable
RUN chmod +x /usr/local/bin/tether-mcp /usr/local/bin/dumbpipe

# Switch to non-root user
USER tether
WORKDIR /home/tether

# Default environment
ENV RUST_LOG=info
ENV MCP_TRANSPORT=stdio

# Health check - just verify the binary runs
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD tether-mcp --help || exit 1

# The MCP server uses stdio, so no port exposure needed for MCP protocol
# However, dumbpipe may need network access which is handled internally

# Entry point
ENTRYPOINT ["tether-mcp"]
