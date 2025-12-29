`tether` is a hardware and software project which helps users hold themselves accountable to place their phone away from their bedroom at night.

You are tasked with creating a comprehensive plan to build `tether`, which should be broken down into discrete phases and tasks - as small as possible - to ensure individual instances of Claude Code can execute them without any ambiguity. This includes designing and iterating on infrastructure, conducting deep search, and reasoning about what decisions to make by evaluating trade-offs using ultrathink.

Keep in mind that I want to ship this project quickly - it's an afternoon/evening project - so you should keep the implementation as simple as possible + YAGNI in mind.

# Features

## Tracks whether phone is close to Raspberry Pi

When requested, the Raspberry Pi checks whether the phone is close to the Raspberry Pi and returns a simple boolean.
This is lazy; we only check on each request.

## Configurable number of passes per month

In situations where you need to keep your phone with you at night, `tether` provides a configurable number of emergency passes that can be granted per month. Users can 'use' a pass programmatically by providing a reason. When the passes for the month are exhausted, an error is returned. Passes refresh automatically at midnight on the first day of each month.

## Configurable via web UI and ssh

On first boot, the Raspberry Pi creates a temporary Wi-Fi network for configuration. When joined, the user can visit a dashboard via the device's local IP/URL to configure the Bluetooth device to track proximity of, the number of passes to grant per month, the local timezone (automatically inferred by the network's location) and the Wi-Fi networks to connect to.

After configuration is complete, the Raspberry Pi connects to the primary Wi-Fi network and the web UI can be accessed via the local IP/URL.

Users can also `ssh` into the Raspberry Pi locally to view logs. Instructions are available in the web UI.

## Works even when you're travelling

Travelling across continents? No problem. `tether` creates a temporary Wi-Fi network whenever the configured networks can't be found, so the user just needs to join again and configure a new Wi-Fi network. The timezone automatically changes depending on the network location, but the user can configure the timezone at any time.

# High-level architecture

`tether` is comprised of two parts:

1. Raspberry Pi with Wi-Fi/Bluetooth that:

- tracks closeness of configured Bluetooth device (my phone!)
- runs a lightweight web UI for configuration + HTTP server as the API
- exposes the HTTP endpoint remotely and securely using [dumbpipe](https://github.com/n0-computer/dumbpipe?tab=readme-ov-file), which leverages [iroh](https://github.com/n0-computer/iroh), to allow external services to connect to the Raspberry Pi

2. MCP server, allowing MCP clients to query status, remaining passes for the month, and create automations.

We want to enforce a clean separation of concerns:

- Processing and tracking occurs on the Raspberry Pi. The Raspberry Pi exposes resources locally via HTTP and remotely via iroh, enabling consumers to build their own automations / clients / use cases on top
- MCP server only queries and requests resources via a secure P2P connection

Furthermore, we do as little computation as possible, preferring to be lazy and only requesting resources / tasks when needed.

# Raspberry Pi

A Raspberry Pi with Wi-Fi and Bluetooth is required. This project targets the Raspberry Pi Zero 2 W specifically.

## Operating system

We run the latest version of Raspberry Pi OS Lite, with the relevant packages, scripts, helpers and other things below baked in using [sdm](https://github.com/gitbls/sdm), which lets us build the OS image programmatically.

## Configuration, persistence and logging

You can pick any structured format (TOML, YAML, JSON etc) to store configuration. The key points are that the Wi-Fi networks and Bluetooth device to target is easily configurable.

The only data that needs to be persisted is _passes_; we should persist the following data:

- Passes remaining for the current month
- Pass history
  - Can easily look up for current month or any previous monthly, quickly (time complexity)
  - If pass used, following information is stored:
    - Date/time in UTC (it should be apparent that it's in UTC by the field name)
    - Reason

You may pick an efficient and simple data structure and format to do so.

To make debugging easy, all logs from all packages should be written to an idiomatic and consistent location on disk.

## Language

We use Rust so we can share logic between all the packages below.

## HTTP server + logic

We run an [axum](https://github.com/tokio-rs/axum) server that exposes the following endpoints, in addition to any you consider necessary. You can design the API parameters, result types, body shape, etc.

Putting logic behind a HTTP server lets us configure the Raspberry Pi programmatically.

Business logic should be extracted out of the HTTP server into another crate / service, or whatever pattern is idiomatic.

- [GET] Whether Bluetooth device is close to the Raspberry Pi
  - Should return shape like { "device_name": boolean }
- [GET] Passes remaining for month
- [GET] Pass history
  - Defaults to current month, but takes any valid month conforming to a standard format as a parameter
- Endpoint for using a pass with reason.
- Endpoint for restarting the system entirely
- Requisite endpoints for adding, editing, removing and configuring Wi-Fi networks + setting primary network, Bluetooth device to track, timezone, and number of passes available
  - NOTE: If Wi-Fi, Bluetooth or timezone are configured again after first boot:
    - Bluetooth can be updated immediately (we ping Bluetooth lazily anyway)
    - If adding more networks, Wi-Fi doesn't need to update. However in the case that a new primary network is set (i.e. no Wi-Fi network has been configured OR new one set), we should connect to the new primary network immediately and advise the user to switch over.
    - If the number of passes is changed in the middle of the month, IT ONLY TAKES EFFECT THE FOLLOWING MONTH.
- Any other endpoints you deem are required
- [GET or whatever's idiomatic] Expose the full OpenAPI specification
- [GET] The dumbpipe ticket

We should use [utoipa](https://github.com/juhaku/utoipa) to generate an OpenAPI specification from the HTTP server, which can be used to generate a type-safe API client for the Web UI and the MCP server directly (more on those later). The OpenAPI specification should be generated in a location where it's accessible by both the Web UI and the MCP server at build time.

We should leverage OpenAPI comments heavily, keeping in mind that the OpenAPI spec will be transformed into a MCP server consumed by AI agents - so it should be as descriptive as possible.

# [dumbpipe](https://github.com/n0-computer/dumbpipe)

Use dumbpipe to expose the HTTP server, then make the ticket accessible via a local HTTP endpoint in the HTTP server. The ticket should be displayed in the web UI.

## Web UI

Create a Vite + React + TypeScript @shadcn/ui app using the shadcn CLI: https://ui.shadcn.com/docs/cli. This app should run locally on the Raspberry Pi and be accessible on the local network ONLY. Use https://biomejs.dev/guides/getting-started/ for linting the web UI.

You can design the architecture and visual layout of the app; you should use the frontend-design plugin/skill to do this. We should use minimal libraries, keep the code as simple as possible (don't be afraid to use libraries where required), and only use @shadcn/ui for components.

The web UI should have the following functionality:

- Onboarding on first boot with the following sequence AND error handling:
  - Choose Bluetooth device to track proximity of (this should be what's visible from the Raspberry Pi for obvious reasons)
  - Configure signal strength as threshold (allows user to see signal strength of device in real time and pick a threshold to consider 'nearby')
  - Configure the number of passes to grant per month
  - Configure the first (primary) Wi-Fi network to use
    - When the Wi-Fi network is joined successfully, allows users to add more Wi-Fi networks
      - Also supports adding manually
  - Configure system timezone (inferred from connected wi-fi network, but can also be done by itself)
  - When onboarding is finished, the dashboard is available. The temporary Wi-Fi network is turned off, and the user should visit the local URL on the primary Wi-Fi network to continue.
- Other than onboarding:
  - See real-time signal strength of Bluetooth device + whether it's nearby
  - See remaining passes for month + pass history
  - See `dumbpipe` ticket
  - Change configuration (Wi-Fi networks, target bluetooth device, any other settings)
    - NOTE: If the number of passes is changed in the middle of the month, IT ONLY TAKES EFFECT THE FOLLOWING MONTH.

Most importantly, IT SHOULD BE MOBILE FRIENDLY - so we should use `<Drawer />` on mobile devices and a iOS-style tab navigation, etc.

The web UI is hosted on the Raspberry Pi. We should generate a type-safe API client to query the backend using [openapi-ts](https://github.com/hey-api/openapi-ts).

# MCP server

Create a MCP server using the MCP Rust SDK (https://github.com/modelcontextprotocol/rust-sdk?tab=readme-ov-file), and use https://gitlab.com/lx-industries/rmcp-openapi to convert the HTTP server using the OpenAPI spec to a MCP server directly.

This server does NOT run on the Raspberry Pi; it's deployed to a provider like Google Cloud Run, etc. It should work as follows:

- Server starts
- Use dumbpipe to connect to the Raspberry Pi given the ticket; the ticket should be set as an env variable // THIS IS IMPORTANT, OTHERWISE IT WON'T WORK!
- Construct the base URL
- Use this to fetch the OpenAPI JSON
- Create the server and so on.

IMPORTANT: We should filter to only expose the endpoints for whether the phone is close to the endpoint, the pass history, the number of passes available for the month, and requesting a pass with a reason - NO OTHER ENDPOINTS AVAILABLE AS TOOLS!

We don't need to implement authentication.

# Cases to consider

## First boot behavior

On first boot, the Raspberry Pi should automatically create an unauthenticated Wi-Fi network for setup with a descriptive name. The user can connect to the Wi-Fi network and visit the local URL to setup using the web UI onboarding. When the primary Wi-Fi network is configured, the Raspberry Pi connects to it, and if successful, runs dumbpipe to generate the ticket and expose the Raspberry Pi to the internet via iroh. After onboarding, the temporary Wi-Fi network should be turned off.

The onboarding should never appear again unless we flash the OS image again.

## Internet isn't available on the configured Wi-Fi network / Wi-Fi drops out / Wi-Fi can't be found

IMPORTANT: For a Wi-Fi network to be considered stable / available, internet must be available. If no internet is available or the Wi-Fi network isn't available, the Raspberry Pi attempts to connect to the configured Wi-Fi networks in order until it finds a Wi-Fi network with available internet. If none can be connected successfully, the temporary Wi-Fi network is turned on for configuration.

## Any program crashes

The Raspberry Pi should be resilient to crashes; if any program crashes or panics, it should restart automatically.

## Logging

We should log all messages, warnings and errors to a log file and expose this via ssh or HTTP, whichever you think is OK.

# Structuring the repo and build commands

Pick whatever structure you think is best. At the root level of the repo, I should be able to run a command to build the OS image easily (it should build the relevant packages first as dependencies), and any other convenience commands.

# README.md

Document the architecture, features and commands thoroughly in README.md so AI agents and humans can understand the repo.

# CLAUDE.md

Create a lightweight CLAUDE.md to make future iteration in this repo more high quality and easier.
