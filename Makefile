# Makefile for tether project
#
# Cross-compilation for Raspberry Pi Zero 2 W (32-bit armhf)
# Uses cross-rs for Docker-based cross-compilation

TARGET := armv7-unknown-linux-gnueabihf
BINARY := tether-server
OUTPUT_DIR := target/pi

.PHONY: all build build-pi strip clean test check fmt lint deploy-pi help

# Default target
all: build

# Build for current platform (development)
build:
	cargo build --release

# Build for Raspberry Pi
build-pi:
	./scripts/build-pi.sh

# Build and strip for Raspberry Pi
strip: build-pi
	@if command -v arm-linux-gnueabihf-strip >/dev/null 2>&1; then \
		arm-linux-gnueabihf-strip $(OUTPUT_DIR)/$(BINARY); \
	else \
		docker run --rm -v $(PWD)/$(OUTPUT_DIR):/work \
			ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:main \
			arm-linux-gnueabihf-strip /work/$(BINARY) 2>/dev/null || true; \
	fi
	@echo "Binary size: $$(ls -lh $(OUTPUT_DIR)/$(BINARY) | awk '{print $$5}')"

# Clean build artifacts
clean:
	cargo clean
	rm -rf $(OUTPUT_DIR)
	rm -f build-pi.log

# Run tests
test:
	cargo test --workspace

# Check compilation without building
check:
	cargo check --workspace

# Format code
fmt:
	cargo fmt --all

# Run lints
lint:
	cargo clippy --workspace -- -D warnings

# Deploy to Raspberry Pi (requires PI_HOST environment variable)
deploy-pi: build-pi
	@if [ -z "$(PI_HOST)" ]; then \
		echo "Error: Set PI_HOST environment variable (e.g., PI_HOST=pi@192.168.1.100)"; \
		exit 1; \
	fi
	scp $(OUTPUT_DIR)/$(BINARY) $(PI_HOST):/home/pi/
	@echo "Deployed to $(PI_HOST)"

# Generate OpenAPI spec
openapi:
	cargo run -p tether-server -- --openapi > openapi.json

# Development server
dev:
	cargo run -p tether-server

# Help
help:
	@echo "Tether Build Targets:"
	@echo ""
	@echo "  build       - Build for current platform (development)"
	@echo "  build-pi    - Cross-compile for Raspberry Pi Zero 2 W"
	@echo "  strip       - Build and strip binary for Pi"
	@echo "  clean       - Remove build artifacts"
	@echo "  test        - Run tests"
	@echo "  check       - Check compilation"
	@echo "  fmt         - Format code"
	@echo "  lint        - Run clippy lints"
	@echo "  deploy-pi   - Deploy to Pi (requires PI_HOST=user@host)"
	@echo "  openapi     - Generate OpenAPI spec"
	@echo "  dev         - Run development server"
	@echo ""
	@echo "Environment Variables:"
	@echo "  PI_HOST     - SSH host for deployment (e.g., pi@192.168.1.100)"
