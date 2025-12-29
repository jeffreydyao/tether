#!/bin/bash
# tether-setup-dirs.sh
# Creates all required directories with correct ownership and permissions
# Run as root during sdm image build or first boot

set -euo pipefail

TETHER_USER="tether"
TETHER_GROUP="tether"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    echo "ERROR: This script must be run as root" >&2
    exit 1
fi

# Verify tether user exists
if ! id "${TETHER_USER}" >/dev/null 2>&1; then
    echo "ERROR: User ${TETHER_USER} does not exist. Run tether-setup-user.sh first." >&2
    exit 1
fi

echo "Creating tether directory structure..."

# /opt/tether - Main installation directory
echo "Creating /opt/tether..."
install -d -m 755 -o root -g root /opt/tether
install -d -m 755 -o root -g root /opt/tether/bin
install -d -m 755 -o root -g root /opt/tether/scripts
install -d -m 755 -o root -g root /opt/tether/web-ui

# /opt/tether/config - Configuration directory
# Owner: root (only root can modify config)
# Group: tether (service can read)
# Mode: 750 (rwxr-x---)
echo "Creating /opt/tether/config..."
install -d -m 750 -o root -g "${TETHER_GROUP}" /opt/tether/config

# Create default config.toml if it doesn't exist
if [[ ! -f /opt/tether/config/tether.toml ]]; then
    echo "Creating default /opt/tether/config/tether.toml..."
    cat > /opt/tether/config/tether.toml << 'EOF'
# Tether Configuration
# This file is created during first boot and modified via the web UI

[bluetooth]
# target_device_mac = "AA:BB:CC:DD:EE:FF"
# target_device_name = "iPhone"
# rssi_threshold = -70

[wifi]
# primary_network = "MyNetwork"
# [[wifi.networks]]
# ssid = "MyNetwork"
# psk = "password"

[passes]
monthly_limit = 3

[server]
listen_address = "0.0.0.0"
port = 3000

[timezone]
# tz = "America/Los_Angeles"
EOF
    chown root:"${TETHER_GROUP}" /opt/tether/config/tether.toml
    chmod 640 /opt/tether/config/tether.toml
fi

# /opt/tether/data - Persistent data directory
# Owner: tether (service needs write access)
# Group: tether
# Mode: 750 (rwxr-x---)
echo "Creating /opt/tether/data..."
install -d -m 750 -o "${TETHER_USER}" -g "${TETHER_GROUP}" /opt/tether/data

# /opt/tether/logs - Log directory (supplementary to journald)
# Owner: tether (service needs write access)
# Group: tether
# Mode: 750 (rwxr-x---)
echo "Creating /opt/tether/logs..."
install -d -m 750 -o "${TETHER_USER}" -g "${TETHER_GROUP}" /opt/tether/logs

echo "Directory structure created successfully"

# Print summary
echo ""
echo "Directory summary:"
ls -la /opt/tether/
echo ""
ls -la /opt/tether/config/
echo ""
ls -la /opt/tether/data/
echo ""
ls -la /opt/tether/logs/
