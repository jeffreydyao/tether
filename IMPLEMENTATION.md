# Tether Implementation Plan

A Raspberry Pi-based phone accountability system that tracks Bluetooth proximity and provides configurable monthly passes.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Raspberry Pi Zero 2 W                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  tether-server  │  │    dumbpipe     │  │ network-watchdog│  │
│  │  (axum HTTP)    │◄─│  (P2P tunnel)   │  │ (WiFi/AP mgmt)  │  │
│  │  port 3000      │  │                 │  │                 │  │
│  └────────┬────────┘  └────────┬────────┘  └─────────────────┘  │
│           │                    │                                 │
│  ┌────────▼────────┐          │     ┌─────────────────────────┐ │
│  │   Web UI        │          │     │  BlueZ (Bluetooth)      │ │
│  │   (React SPA)   │          │     │  via bluer crate        │ │
│  └─────────────────┘          │     └─────────────────────────┘ │
└───────────────────────────────┼─────────────────────────────────┘
                                │ iroh P2P
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Cloud (Google Cloud Run)                      │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  tether-mcp (MCP Server)                                    ││
│  │  - Connects via dumbpipe ticket (env var)                   ││
│  │  - Exposes filtered endpoints as MCP tools                  ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

**Target Platform:** Raspberry Pi Zero 2 W (512MB RAM)
**Target Architecture:** 32-bit armhf (`armv7-unknown-linux-gnueabihf`)
**OS:** Raspberry Pi OS Lite (Trixie, armhf)

---

## Phase 1: Rust Backend

Core Rust workspace with `tether-core` (business logic) and `tether-server` (HTTP API).

| Sub-Phase | Description | Detailed Plan |
|-----------|-------------|---------------|
| 1.1 | Cargo workspace setup, dependencies, Cross.toml | [plans/1.1-workspace-setup.md](plans/1.1-workspace-setup.md) |
| 1.2 | Configuration module (TOML loading, types) | [plans/1.2-config-module.md](plans/1.2-config-module.md) |
| 1.3 | Pass management and JSON persistence | [plans/1.3-passes-module.md](plans/1.3-passes-module.md) |
| 1.4 | Bluetooth scanning with bluer crate | [plans/1.4-bluetooth-module.md](plans/1.4-bluetooth-module.md) |
| 1.5 | Error types with thiserror | [plans/1.5-error-types.md](plans/1.5-error-types.md) |
| 1.6 | HTTP server setup (axum, state, logging) | [plans/1.6-http-server-setup.md](plans/1.6-http-server-setup.md) |
| 1.7 | HTTP route handlers | [plans/1.7-http-routes.md](plans/1.7-http-routes.md) |
| 1.8 | OpenAPI generation with utoipa | [plans/1.8-openapi-generation.md](plans/1.8-openapi-generation.md) |

### Key Dependencies
- `bluer` - Bluetooth RSSI via BlueZ
- `axum` - HTTP server
- `utoipa` + `utoipa-axum` - OpenAPI generation
- `tokio` - Async runtime
- `serde` + `toml` - Config serialization
- `chrono` - Date/time handling
- `thiserror` - Error types
- `tracing` + `tracing-appender` - Logging

### HTTP API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/proximity` | Check if phone is nearby |
| GET | `/api/passes` | Remaining passes for current month |
| GET | `/api/passes/history?month=YYYY-MM` | Pass history |
| POST | `/api/passes/use` | Use a pass with reason |
| GET | `/api/config` | Current configuration |
| PUT | `/api/config/bluetooth` | Update BT target |
| PUT | `/api/config/wifi` | Update WiFi networks |
| PUT | `/api/config/timezone` | Update timezone |
| PUT | `/api/config/passes` | Update passes/month |
| GET | `/api/devices` | Scan for BT devices |
| GET | `/api/system/status` | System status |
| GET | `/api/system/ticket` | Dumbpipe ticket |
| POST | `/api/system/restart` | Restart system |
| GET | `/api/openapi.json` | OpenAPI spec |
| GET | `/health` | Health check |

---

## Phase 2: Web UI

React SPA with Vite, shadcn/ui, and TanStack Query.

| Sub-Phase | Description | Detailed Plan |
|-----------|-------------|---------------|
| 2.1 | Project setup (Vite, shadcn, openapi-ts) | [plans/2.1-webui-project-setup.md](plans/2.1-webui-project-setup.md) |
| 2.2 | Routing and layout (AppShell, MobileNav) | [plans/2.2-routing-and-layout.md](plans/2.2-routing-and-layout.md) |
| 2.3 | Onboarding wizard (6 steps) | [plans/2.3-onboarding-wizard.md](plans/2.3-onboarding-wizard.md) |
| 2.4 | Dashboard components (ProximityCard, PassesCard) | [plans/2.4-dashboard-components.md](plans/2.4-dashboard-components.md) |
| 2.5 | Settings drawer | [plans/2.5-settings-drawer.md](plans/2.5-settings-drawer.md) |

### Onboarding Flow
1. **BluetoothScanStep** - Scan and select device to track
2. **SignalThresholdStep** - Real-time RSSI calibration
3. **PassesConfigStep** - Set monthly pass count
4. **WifiConfigStep** - Configure primary WiFi
5. **TimezoneStep** - Auto-inferred timezone
6. **CompletionStep** - Network switch instructions

### Dashboard
- **Status Tab**: ProximityCard (real-time), DumbpipeCard (ticket)
- **Passes Tab**: PassesCard (remaining + history)
- **Settings**: Drawer with WiFi, Bluetooth, Passes, Timezone

---

## Phase 3: Pi Deployment

Custom Raspberry Pi OS image with sdm, systemd services, and network management.

| Sub-Phase | Description | Detailed Plan |
|-----------|-------------|---------------|
| 3.1 | sdm plugin and image build script | [plans/3.1-sdm-plugin.md](plans/3.1-sdm-plugin.md) |
| 3.2 | Systemd service files | [plans/3.2-systemd-services.md](plans/3.2-systemd-services.md) |
| 3.3 | Network watchdog script | [plans/3.3-network-watchdog.md](plans/3.3-network-watchdog.md) |
| 3.4 | Cross-compilation setup | [plans/3.4-cross-compilation.md](plans/3.4-cross-compilation.md) |

### Base Image
- **OS:** Raspberry Pi OS Lite (Trixie, armhf)
- **URL:** `https://downloads.raspberrypi.com/raspios_lite_armhf/images/raspios_lite_armhf-2025-12-04/2025-12-04-raspios-trixie-armhf-lite.img.xz`

### Cross-Compilation
- **Target:** `armv7-unknown-linux-gnueabihf`
- **Tool:** cross-rs with Docker

### Systemd Services
1. **tether.service** - HTTP server (auto-restart)
2. **tether-dumbpipe.service** - P2P tunnel
3. **tether-hotspot.service** - AP mode management
4. **tether-network-monitor.service** - WiFi/AP watchdog

### Network Management
- First boot: Create "TetherSetup" AP (open)
- After onboarding: Connect to configured WiFi
- Fallback: Re-enable AP if no internet

---

## Phase 4: MCP & Tooling

MCP server for Claude integration and build automation.

| Sub-Phase | Description | Detailed Plan |
|-----------|-------------|---------------|
| 4.1 | MCP server implementation | [plans/4.1-mcp-server.md](plans/4.1-mcp-server.md) |
| 4.2 | Makefile and build scripts | [plans/4.2-makefile-scripts.md](plans/4.2-makefile-scripts.md) |

### MCP Server
- Reads `TETHER_DUMBPIPE_TICKET` from environment
- Connects via dumbpipe to Pi
- Exposes filtered endpoints: proximity, passes, history, use-pass
- Deployed to Google Cloud Run

---

## Repository Structure

```
tether/
├── Cargo.toml                      # Workspace root
├── Cross.toml                      # cross-rs config for armv7
├── Makefile                        # Build commands
├── IMPLEMENTATION.md               # This file
│
├── crates/
│   ├── tether-core/                # Shared business logic
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs
│   │       ├── passes.rs
│   │       ├── bluetooth.rs
│   │       └── error.rs
│   │
│   ├── tether-server/              # HTTP server
│   │   └── src/
│   │       ├── main.rs
│   │       ├── state.rs
│   │       ├── error.rs
│   │       └── routes/
│   │
│   └── tether-mcp/                 # MCP server
│       └── src/main.rs
│
├── web/                            # Vite + React + shadcn/ui
│   └── src/
│       ├── client/                 # Generated API client
│       ├── components/
│       ├── hooks/
│       └── pages/
│
├── deploy/
│   ├── pi/sdm/                     # Pi image build
│   └── cloud/                      # Cloud Run deployment
│
└── plans/                          # Detailed implementation plans
    ├── 1.1-workspace-setup.md
    ├── 1.2-config-module.md
    ├── ...
    └── 4.2-makefile-scripts.md
```

---

## Build Commands

```bash
# Build everything and create Pi image
make build-image

# Individual targets
make build-pi        # Cross-compile Rust for armv7
make build-web       # Build React app
make build-mcp       # Build MCP server
make deploy-cloud    # Deploy MCP to Cloud Run

# Development
make dev-server      # Run Rust server locally
make dev-web         # Run React dev server

# Code quality
make check           # cargo check
make fmt             # Format all code
make lint            # Run linters
make test            # Run all tests
```

---

## Implementation Order

1. **Phase 1.1-1.5**: Core Rust types and business logic
2. **Phase 1.6-1.8**: HTTP server and OpenAPI
3. **Phase 2.1**: Web UI project setup + API client generation
4. **Phase 2.2-2.5**: Web UI components
5. **Phase 3.4**: Cross-compilation setup
6. **Phase 3.1-3.3**: Pi deployment (sdm, systemd, network)
7. **Phase 4.1-4.2**: MCP server and build automation
