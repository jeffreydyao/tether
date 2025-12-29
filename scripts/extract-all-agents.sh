#!/bin/bash
# Extract all agent outputs from the previous planning session
# Run this to populate the plans/ directory with agent outputs

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PLANS_DIR="$(dirname "$SCRIPT_DIR")/plans"
EXTRACT_SCRIPT="$SCRIPT_DIR/extract-agent-output.sh"

mkdir -p "$PLANS_DIR"

echo "Extracting agent outputs to $PLANS_DIR..."
echo ""

extract() {
    local agent_id="$1"
    local output_file="$2"
    echo -n "  $agent_id -> $output_file ... "

    if "$EXTRACT_SCRIPT" "$agent_id" > "$PLANS_DIR/$output_file" 2>/dev/null; then
        size=$(wc -c < "$PLANS_DIR/$output_file" | tr -d ' ')
        if [ "$size" -gt 100 ]; then
            echo "OK (${size} bytes)"
        else
            echo "EMPTY/FAILED (${size} bytes)"
        fi
    else
        echo "FAILED"
    fi
}

echo "=== Session 1: Rust Backend (Completed) ==="
extract "ab48093" "1.1-workspace-setup.md"
extract "af72b3a" "1.2-config-module.md"
extract "a71dedf" "1.3-passes-module.md"
extract "adf8828" "1.4-bluetooth-module.md"
extract "aa00c59" "1.5-error-types.md"
extract "a3dec8c" "1.6-http-server-setup.md"

echo ""
echo "=== Session 1: Rust Backend (Pending) ==="
extract "aba6fe5" "1.7-http-routes.md"
extract "aedc0d9" "1.8-openapi-generation.md"

echo ""
echo "=== Session 2: Web UI ==="
extract "aa8d9c4" "2.1-webui-project-setup.md"
extract "a5729b9" "2.2-routing-and-layout.md"
extract "a658917" "2.3-onboarding-wizard.md"
extract "ab1475f" "2.4-dashboard-components.md"
extract "ac80f8c" "2.5-settings-drawer.md"

echo ""
echo "=== Session 3: Pi Deployment ==="
extract "ac06992" "3.1-sdm-plugin.md"
extract "ad2e14b" "3.2-systemd-services.md"
extract "a053e62" "3.3-network-watchdog.md"
extract "a929758" "3.4-cross-compilation.md"

echo ""
echo "=== Session 4: MCP & Tooling ==="
extract "aa7d0b3" "4.1-mcp-server.md"
extract "a16ad9d" "4.2-makefile-scripts.md"

echo ""
echo "Done! All files written to $PLANS_DIR"
echo ""
ls -la "$PLANS_DIR"
