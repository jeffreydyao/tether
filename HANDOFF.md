# Tether Implementation - Session Handoff Document

## Overview

This document captures the state of the tether project planning session so another Claude Code instance can continue exactly where we left off.

**Project**: Tether - A Raspberry Pi-based phone accountability system
**Current Phase**: Plan Mode - Creating excruciating implementation details
**Master Plan File**: `/Users/jeffrey/.claude/plans/parsed-hatching-papert.md`

## What Tether Does

- Tracks phone proximity via Bluetooth RSSI on Raspberry Pi
- Provides configurable monthly "passes" for exceptions
- Exposes HTTP API with OpenAPI documentation
- Has a mobile-first web UI (Vite + React + shadcn/ui)
- Uses dumbpipe/iroh for P2P remote access
- Deploys via custom Raspberry Pi OS image using sdm
- Has an MCP server for AI integration via rmcp-openapi

## User Preferences (Already Decided)

1. **Implementation Approach**: Multiple sessions (not single session)
2. **MCP Server**: rmcp-openapi (auto-generate from OpenAPI spec)
3. **UI Design**: Clean & minimal using frontend-design skill

## Pending Agent Results to Collect

The following agents were launched to create detailed implementation plans. Their results need to be collected using `TaskOutput`:

### Session 1 - Rust Backend (Remaining)

| Agent ID | Task | Status |
|----------|------|--------|
| `aba6fe5` | HTTP routes implementation | Pending collection |
| `aedc0d9` | OpenAPI generation with utoipa | Pending collection |

### Session 2 - Web UI (All Pending)

| Agent ID | Task | Status |
|----------|------|--------|
| `aa8d9c4` | Web UI project setup (Vite+React+shadcn) | Pending collection |
| `a5729b9` | Routing and layout components | Pending collection |
| `a658917` | Onboarding wizard (4 steps) | Pending collection |
| `ab1475f` | Dashboard components | Pending collection |
| `ac80f8c` | Settings drawer | Pending collection |

### Session 3 - Pi Deployment (All Pending)

| Agent ID | Task | Status |
|----------|------|--------|
| `ac06992` | sdm plugin and image build | Pending collection |
| `ad2e14b` | Systemd services | Pending collection |
| `a053e62` | Network watchdog script | Pending collection |
| `a929758` | Cross-compilation setup | Pending collection |

### Session 4 - MCP & Tooling (All Pending)

| Agent ID | Task | Status |
|----------|------|--------|
| `aa7d0b3` | MCP server with rmcp-openapi | Pending collection |
| `a16ad9d` | Makefile and helper scripts | Pending collection |

## Already Collected Agent Results

These results have been collected and should be incorporated into the final plan:

### 1. Rust Workspace Setup
```toml
# Workspace Cargo.toml key dependencies
[workspace.dependencies]
tokio = { version = "1.43", features = ["full"] }
axum = { version = "0.8", features = ["macros"] }
bluer = { version = "0.17", features = ["bluetoothd"] }
utoipa = { version = "5.3", features = ["axum_extras", "chrono", "uuid"] }
thiserror = "2.0"
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
```
- Directory structure: `crates/tether-core/`, `crates/tether-server/`
- Cross.toml for ARM64 cross-compilation

### 2. Config Module (`tether-core/src/config.rs`)
- Structs: `Config`, `BluetoothConfig`, `WifiConfig`, `WifiNetwork`, `PassesConfig`, `SystemConfig`
- MAC address validation with lazy regex
- TOML serialization/deserialization
- `pending_per_month` for mid-month pass changes

### 3. Passes Module (`tether-core/src/passes.rs`)
- `PassManager` struct with methods: `load_or_create()`, `remaining()`, `use_pass()`, `history()`, `all_history()`, `set_per_month()`
- `PassData` struct for JSON persistence with `current_month`, `remaining`, `per_month`, `history` HashMap
- `PassEntry` struct with `used_at_utc` (DateTime<Utc>) and `reason` (String)
- Automatic month reset logic in `maybe_reset_month()`

### 4. Error Types (`tether-core/src/error.rs`)
```rust
#[derive(Debug, thiserror::Error)]
pub enum TetherError {
    BluetoothAdapterNotFound,
    BluetoothAdapterPoweredOff,
    BluetoothScanFailed(String),
    DeviceNotFound(String),
    NoPassesRemaining,
    // ... more variants
}
```
- Helper methods: `is_bluetooth_error()`, `is_config_error()`, `http_status_code()`, `error_code()`

### 5. Bluetooth Module (`tether-core/src/bluetooth.rs`)
- `BluetoothScanner` struct with bluer Session and Adapter
- `BluetoothConfig` struct with `device_address`, `rssi_threshold`
- `ProximityResult` struct with `nearby`, `rssi`, `device_name`, `timestamp`
- Methods: `new()`, `check_proximity()`, `discover_devices()`, `get_device_rssi()`
- Mock implementation with feature flag `mock-bluetooth`

### 6. HTTP Server Setup (`tether-server/src/main.rs`)
- `AppState` struct with `config`, `pass_manager`, `bluetooth`
- `SharedState = Arc<RwLock<AppState>>`
- Tracing initialization, router construction, graceful shutdown
- Middleware: TraceLayer, CorsLayer (dev only)
- Static file serving with SPA fallback using `ServeDir`

## Instructions for Next Session

### Step 1: Collect Remaining Agent Results

Run `TaskOutput` for each pending agent ID listed above. Example:
```
TaskOutput(task_id: "aba6fe5", block: true)
```

### Step 2: Compile Everything into Master Plan

Once all agent results are collected, update the master plan file at:
`/Users/jeffrey/.claude/plans/parsed-hatching-papert.md`

The plan should have this structure:
1. **Session 1: Rust Backend** (~2 hours)
   - 1.1 Workspace setup
   - 1.2 Config module
   - 1.3 Passes module
   - 1.4 Bluetooth module
   - 1.5 Error types
   - 1.6 HTTP server setup
   - 1.7 HTTP routes
   - 1.8 OpenAPI generation

2. **Session 2: Web UI** (~2 hours)
   - 2.1 Project setup
   - 2.2 Routing and layout
   - 2.3 Onboarding wizard
   - 2.4 Dashboard components
   - 2.5 Settings drawer

3. **Session 3: Pi Deployment** (~1.5 hours)
   - 3.1 sdm plugin
   - 3.2 Systemd services
   - 3.3 Network watchdog
   - 3.4 Cross-compilation

4. **Session 4: MCP & Tooling** (~1 hour)
   - 4.1 MCP server
   - 4.2 Makefile and scripts

### Step 3: Exit Plan Mode

After the master plan is complete with excruciating detail, call `ExitPlanMode` to get user approval.

## Key Technical Decisions

| Component | Decision | Reason |
|-----------|----------|--------|
| Bluetooth | `bluer` crate | Linux-only (Pi), better BlueZ integration than btleplug |
| P2P | dumbpipe CLI | Simpler than embedding iroh directly |
| WiFi/AP | NetworkManager | Easier than wpa_supplicant, handles AP mode |
| HTTP | axum 0.8 + utoipa | Modern, OpenAPI generation |
| Persistence | TOML (config) + JSON (passes) | Human-readable, simple |
| Web UI | Vite + React + shadcn/ui + bun | Fast, modern, user preference |
| API Client | @hey-api/openapi-ts | Auto-generate from OpenAPI |
| MCP | rmcp-openapi | Auto-generate MCP from OpenAPI (user preference) |

## File Paths

- Spec: `/Users/jeffrey/code/tether/claude-spec.md`
- Master Plan: `/Users/jeffrey/.claude/plans/parsed-hatching-papert.md`
- This Handoff: `/Users/jeffrey/code/tether/HANDOFF.md`

## Current Todo List State

```
✅ Detail Rust workspace setup
✅ Detail config module
✅ Detail passes module
✅ Detail Bluetooth module
✅ Detail error types
✅ Detail HTTP server setup
🔄 Detail HTTP routes implementation
🔄 Detail OpenAPI generation
🔄 Detail Web UI project setup
🔄 Detail routing and layout
🔄 Detail onboarding wizard
🔄 Detail dashboard components
🔄 Detail settings drawer
🔄 Detail sdm plugin
🔄 Detail systemd services
🔄 Detail network watchdog
🔄 Detail cross-compilation
🔄 Detail MCP server
🔄 Detail Makefile/scripts
⏳ Compile all into comprehensive plan document
```

## Quick Start Commands

```bash
# Collect all remaining agent results (run these TaskOutput calls):
# aba6fe5, aedc0d9, aa8d9c4, a5729b9, a658917, ab1475f, ac80f8c, ac06992, ad2e14b, a053e62, a929758, aa7d0b3, a16ad9d

# Read the spec for context
Read("/Users/jeffrey/code/tether/claude-spec.md")

# Read current master plan
Read("/Users/jeffrey/.claude/plans/parsed-hatching-papert.md")
```
