# =============================================================================
# TETHER - Central Build Orchestration
# =============================================================================
#
# This Makefile provides build targets for all Tether components:
# - Rust server (tether-server) for Raspberry Pi
# - MCP server (tether-mcp) for cloud deployment
# - React web UI
# - Raspberry Pi SD card image
# - Cloud Run deployment
#
# Quick Start:
#   make               - Build everything for Pi
#   make dev           - Run development servers
#   make build-image   - Create complete Pi SD card image
#   make deploy-cloud  - Deploy MCP server to Cloud Run
#
# =============================================================================

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

# Pi cross-compilation target
PI_TARGET := armv7-unknown-linux-gnueabihf
PI_TARGET_64 := aarch64-unknown-linux-gnu

# Output directories
DIST_DIR := dist
PI_DIST_DIR := $(DIST_DIR)/pi
WEB_DIST_DIR := web/dist
MCP_DIST_DIR := $(DIST_DIR)/mcp

# Binary names
PI_BINARY := tether-server
MCP_BINARY := tether-mcp

# Scripts
SCRIPTS_DIR := scripts

# Web UI directory
WEB_DIR := web

# Quietable commands (use V=1 for verbose output)
Q := $(if $(V),,@)

# -----------------------------------------------------------------------------
# Default Target
# -----------------------------------------------------------------------------

.PHONY: all
all: build-pi build-web

# -----------------------------------------------------------------------------
# Rust Development
# -----------------------------------------------------------------------------

.PHONY: build check test fmt lint

## Build all Rust packages for current platform
build:
	$(Q)cargo build --release

## Check all packages compile
check:
	$(Q)cargo check --workspace

## Run all tests
test:
	$(Q)cargo test --workspace

## Format all code
fmt:
	$(Q)cargo fmt --all

## Run clippy lints
lint:
	$(Q)cargo clippy --workspace -- -D warnings

# -----------------------------------------------------------------------------
# Cross-Compilation for Raspberry Pi
# -----------------------------------------------------------------------------

.PHONY: build-pi build-pi-64 strip

## Cross-compile tether-server for Raspberry Pi (32-bit)
build-pi:
	$(Q)$(SCRIPTS_DIR)/build-pi.sh

## Cross-compile tether-server for Raspberry Pi (64-bit)
build-pi-64:
	$(Q)$(SCRIPTS_DIR)/build-pi.sh --target $(PI_TARGET_64)

## Strip symbols from Pi binary for smaller size
strip: build-pi
	$(Q)@if command -v arm-linux-gnueabihf-strip >/dev/null 2>&1; then \
		arm-linux-gnueabihf-strip $(PI_DIST_DIR)/$(PI_BINARY); \
	else \
		docker run --rm -v $(PWD)/$(PI_DIST_DIR):/work \
			ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:main \
			arm-linux-gnueabihf-strip /work/$(PI_BINARY) 2>/dev/null || true; \
	fi
	@echo "Binary size: $$(ls -lh $(PI_DIST_DIR)/$(PI_BINARY) | awk '{print $$5}')"

# -----------------------------------------------------------------------------
# MCP Server
# -----------------------------------------------------------------------------

.PHONY: build-mcp

## Build MCP server for current platform
build-mcp:
	$(Q)cargo build --release --package tether-mcp
	$(Q)mkdir -p $(MCP_DIST_DIR)
	$(Q)cp target/release/$(MCP_BINARY) $(MCP_DIST_DIR)/

# -----------------------------------------------------------------------------
# Web UI
# -----------------------------------------------------------------------------

.PHONY: build-web install-web lint-web

## Build React web UI
build-web:
	$(Q)$(SCRIPTS_DIR)/build-web.sh

## Install web dependencies
install-web:
	$(Q)cd $(WEB_DIR) && bun install

## Lint web code
lint-web:
	$(Q)cd $(WEB_DIR) && bun run lint

# -----------------------------------------------------------------------------
# OpenAPI Generation
# -----------------------------------------------------------------------------

.PHONY: generate-openapi openapi

## Generate OpenAPI spec and TypeScript client
generate-openapi:
	$(Q)$(SCRIPTS_DIR)/generate-openapi.sh

## Alias for generate-openapi
openapi: generate-openapi

# -----------------------------------------------------------------------------
# Pi Image Building
# -----------------------------------------------------------------------------

.PHONY: build-image

## Build complete Pi SD card image (requires Linux or Docker)
build-image: build-pi build-web
	$(Q)$(SCRIPTS_DIR)/build-image.sh

# -----------------------------------------------------------------------------
# Cloud Deployment
# -----------------------------------------------------------------------------

.PHONY: deploy-cloud

## Deploy MCP server to Google Cloud Run
deploy-cloud:
	$(Q)deploy/cloud/deploy.sh

# -----------------------------------------------------------------------------
# Development
# -----------------------------------------------------------------------------

.PHONY: dev dev-server dev-web dev-all

## Run Rust server in development mode
dev-server:
	$(Q)RUST_LOG=debug cargo run -p tether-server

## Run web UI development server
dev-web:
	$(Q)cd $(WEB_DIR) && bun run dev

## Run both servers (requires two terminals or use tmux)
dev: dev-server

## Run all development servers in parallel (using background jobs)
dev-all:
	@echo "Starting development servers..."
	@echo "  Rust server: http://localhost:3000"
	@echo "  Web UI:      http://localhost:5173"
	@echo ""
	$(Q)(trap 'kill 0' SIGINT; \
		(RUST_LOG=debug cargo run -p tether-server) & \
		(cd $(WEB_DIR) && bun run dev) & \
		wait)

# -----------------------------------------------------------------------------
# Pi Deployment
# -----------------------------------------------------------------------------

.PHONY: deploy-pi ssh-pi

## Deploy built binary to Raspberry Pi (requires PI_HOST)
deploy-pi: build-pi
	@if [ -z "$(PI_HOST)" ]; then \
		echo "Error: Set PI_HOST environment variable (e.g., PI_HOST=pi@192.168.1.100)"; \
		exit 1; \
	fi
	$(Q)scp $(PI_DIST_DIR)/$(PI_BINARY) $(PI_HOST):/opt/tether/bin/
	$(Q)ssh $(PI_HOST) 'sudo systemctl restart tether'
	@echo "Deployed to $(PI_HOST) and restarted tether service"

## SSH to Raspberry Pi
ssh-pi:
	@if [ -z "$(PI_HOST)" ]; then \
		echo "Error: Set PI_HOST environment variable (e.g., PI_HOST=pi@192.168.1.100)"; \
		exit 1; \
	fi
	$(Q)ssh $(PI_HOST)

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------

.PHONY: clean clean-rust clean-web clean-dist clean-all

## Clean Rust build artifacts
clean-rust:
	$(Q)cargo clean

## Clean web build artifacts
clean-web:
	$(Q)rm -rf $(WEB_DIR)/dist $(WEB_DIR)/node_modules/.cache

## Clean distribution directory
clean-dist:
	$(Q)rm -rf $(DIST_DIR)

## Clean all build artifacts
clean-all: clean-rust clean-web clean-dist
	$(Q)rm -f openapi.json build-pi.log

## Default clean target
clean: clean-dist
	$(Q)cargo clean

# -----------------------------------------------------------------------------
# Installation
# -----------------------------------------------------------------------------

.PHONY: install-deps

## Install development dependencies
install-deps:
	@echo "Installing Rust dependencies..."
	$(Q)rustup target add $(PI_TARGET) $(PI_TARGET_64) 2>/dev/null || true
	$(Q)cargo install cross --locked 2>/dev/null || true
	@echo "Installing web dependencies..."
	$(Q)cd $(WEB_DIR) && bun install
	@echo "Dependencies installed."

# -----------------------------------------------------------------------------
# Help
# -----------------------------------------------------------------------------

.PHONY: help

## Show this help
help:
	@echo "Tether Build System"
	@echo "==================="
	@echo ""
	@echo "Development:"
	@echo "  make dev           Run Rust server in development mode"
	@echo "  make dev-web       Run web UI development server"
	@echo "  make dev-all       Run both servers in parallel"
	@echo "  make build         Build all Rust packages"
	@echo "  make check         Check compilation without building"
	@echo "  make test          Run all tests"
	@echo "  make fmt           Format all code"
	@echo "  make lint          Run clippy lints"
	@echo ""
	@echo "Raspberry Pi:"
	@echo "  make build-pi      Cross-compile for Pi (32-bit)"
	@echo "  make build-pi-64   Cross-compile for Pi (64-bit)"
	@echo "  make strip         Strip binary for smaller size"
	@echo "  make build-image   Build complete Pi SD card image"
	@echo "  make deploy-pi     Deploy to Pi (requires PI_HOST)"
	@echo ""
	@echo "Web UI:"
	@echo "  make build-web     Build React web UI"
	@echo "  make install-web   Install web dependencies"
	@echo "  make lint-web      Lint web code"
	@echo ""
	@echo "MCP Server:"
	@echo "  make build-mcp     Build MCP server"
	@echo "  make deploy-cloud  Deploy to Cloud Run"
	@echo ""
	@echo "OpenAPI:"
	@echo "  make generate-openapi  Generate OpenAPI spec and TypeScript client"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean         Clean distribution directory"
	@echo "  make clean-all     Clean all build artifacts"
	@echo ""
	@echo "Environment Variables:"
	@echo "  PI_HOST    SSH host for Pi deployment (e.g., pi@192.168.1.100)"
	@echo "  V=1        Verbose output"
	@echo ""
