#!/bin/bash
# tether-setup-user.sh
# Creates the tether system user with appropriate group memberships
# Run as root during sdm image build or first boot

set -euo pipefail

TETHER_USER="tether"
TETHER_HOME="/var/lib/tether"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    echo "ERROR: This script must be run as root" >&2
    exit 1
fi

# Create tether group if it doesn't exist
if ! getent group "${TETHER_USER}" >/dev/null 2>&1; then
    echo "Creating group: ${TETHER_USER}"
    groupadd --system "${TETHER_USER}"
fi

# Create tether user if it doesn't exist
if ! id "${TETHER_USER}" >/dev/null 2>&1; then
    echo "Creating user: ${TETHER_USER}"
    useradd \
        --system \
        --gid "${TETHER_USER}" \
        --home-dir "${TETHER_HOME}" \
        --no-create-home \
        --shell /usr/sbin/nologin \
        --comment "Tether service account" \
        "${TETHER_USER}"
fi

# Add tether user to required groups
# bluetooth: Required for BlueZ D-Bus API access
# netdev: Required for NetworkManager D-Bus API access (wifi management)
echo "Adding ${TETHER_USER} to supplementary groups..."

for group in bluetooth netdev; do
    if getent group "${group}" >/dev/null 2>&1; then
        if ! groups "${TETHER_USER}" | grep -qw "${group}"; then
            echo "  Adding to group: ${group}"
            usermod -aG "${group}" "${TETHER_USER}"
        else
            echo "  Already in group: ${group}"
        fi
    else
        echo "  WARNING: Group ${group} does not exist, skipping"
    fi
done

echo "User ${TETHER_USER} setup complete"
echo "Groups: $(groups ${TETHER_USER})"
