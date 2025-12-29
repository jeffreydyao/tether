# CLAUDE.md - AI Assistant Context for tether

This file provides context for AI assistants (Claude, etc.) working on this codebase.

## Project Overview

**tether** is a Raspberry Pi-based phone proximity tracker that helps users keep their phones away from the bedroom at night. It consists of:

1. **Raspberry Pi Application**: Rust server + React web UI running on a Pi Zero 2 W
2. **MCP Server**: Cloud-deployed service that exposes the Pi's API to AI assistants

## Architecture Summary

```
Raspberry Pi Zero 2 W
├── tether-core       # Business logic (Bluetooth, passes, config)
├── tether-server     # HTTP API (axum) with OpenAPI via utoipa
├── web/              # React + Vite + shadcn/ui
└── dumbpipe          # Secure P2P tunnel via iroh

Cloud Run
└── tether-mcp        # MCP server (uses rmcp SDK)
```

## Key Files

| Path | Purpose |
|------|---------|
| `Cargo.toml` | Rust workspace root |
| `crates/tether-core/src/lib.rs` | Core business logic entry |
| `crates/tether-server/src/main.rs` | HTTP server entry point |
| `crates/tether-server/src/api/` | API route handlers |
| `crates/tether-mcp/src/main.rs` | MCP server entry |
| `web/src/App.tsx` | React app entry |
| `web/src/client/` | Generated TypeScript API client |
| `Makefile` | Build orchestration |
| `scripts/` | Build and deployment scripts |
| `pi/sdm-config/` | Raspberry Pi image customization |

## Common Tasks

### Generate OpenAPI spec and TypeScript client
```bash
make generate-openapi
```
This runs `scripts/generate-openapi.sh`, which:
1. Builds and runs the Rust server with `--openapi`
2. Writes `openapi.json` to project root
3. Generates TypeScript client in `web/src/client/`

### Build for Raspberry Pi
```bash
make build-pi
```
Cross-compiles using `cross` (Docker-based) to `armv7-unknown-linux-gnueabihf`.

### Build complete Pi image
```bash
make build-image
```
Creates `dist/tether-pi.img` using sdm.

### Run development servers
```bash
make dev-server  # Rust on :3000
make dev-web     # Vite on :5173
make dev-all     # Both in parallel
```

## Code Conventions

### Rust
- Use `thiserror` for error types
- Use `tracing` for logging (not `log`)
- OpenAPI: Heavy use of utoipa macros (`#[utoipa::path]`, `#[derive(ToSchema)]`)
- Axum: Use extractors, state, and layers idiomatically
- Config: TOML format, validated with `serde` + custom validation

### TypeScript/React
- Use `bun` as package manager
- Use `biome` for formatting/linting
- Use `@hey-api/openapi-ts` for API client generation
- Use shadcn/ui components exclusively
- Mobile-first: Use `<Drawer>` on mobile, dialogs on desktop

### Make
- All targets are `.PHONY` unless they produce files
- Dependencies are explicitly declared
- Scripts in `scripts/` handle complex logic
- Use `$(Q)` prefix for quietable commands

## Testing

```bash
make test          # All tests (Rust)
make lint          # All linters (clippy)
make check         # Rust type checking only
```

## Environment

Development uses `.env` (copied from `.env.example`). Key variables:

- `RUST_LOG=debug` - Logging level
- `TETHER_MOCK_BLUETOOTH=true` - Mock Bluetooth for dev
- `TETHER_CONFIG=./config/dev.toml` - Config file path

## Common Issues

### OpenAPI generation fails
1. Ensure `--openapi` flag is handled in `main.rs`
2. Check that `utoipa` features are enabled in `Cargo.toml`
3. Try `make generate-openapi` explicitly

### Cross-compilation fails
1. Ensure Docker is running
2. Try `./scripts/build-pi.sh` explicitly
3. Check `~/.cargo/config.toml` for conflicting linker settings

### Web build fails
1. Run `cd web && bun install` to ensure dependencies
2. Ensure OpenAPI client is generated: `make generate-openapi`
3. Check `web/src/client/` exists and has `index.ts`

### MCP server can't connect
1. Verify `TETHER_DUMBPIPE_TICKET` environment variable is set
2. Check the Pi is online and dumbpipe is running
3. Test with `dumbpipe connect <ticket>` locally

## Adding New API Endpoints

1. Add handler in `crates/tether-server/src/api/`
2. Add utoipa annotations (`#[utoipa::path]`, schemas)
3. Register in router (`src/api/mod.rs`)
4. Run `make generate-openapi` to update TypeScript client
5. Use generated client in `web/src/client/` in React components

## Build Dependencies Graph

```
generate-openapi
       ↓
   build-web ←──────┐
       ↓            │
   build-pi ←───────┤
       ↓            │
  build-image ←─────┘

   build-mcp ←── (standalone)
       ↓
  deploy-cloud
```

## Useful Commands

```bash
# Check what make will do without running
make -n build-image

# Verbose output
make V=1 build-pi

# Clean and rebuild everything
make clean-all && make build-image

# Just regenerate TypeScript client (skip Rust rebuild)
cd web && bunx @hey-api/openapi-ts -i ../openapi.json -o src/client
```
