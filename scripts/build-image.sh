#!/usr/bin/env bash
# =============================================================================
# TETHER - Raspberry Pi Image Builder
# =============================================================================
# Creates a complete, flashable SD card image using sdm.
#
# PREREQUISITES:
# - Linux host (or Docker with privileged access)
# - sdm installed: https://github.com/gitbls/sdm
# - Base Raspberry Pi OS image downloaded
#
# USAGE:
#   ./scripts/build-image.sh [--base-image PATH] [--output PATH]
#
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Output image path
OUTPUT_IMAGE="${PROJECT_ROOT}/dist/tether-pi.img"

# Base Raspberry Pi OS image (64-bit Lite)
BASE_IMAGE_URL="https://downloads.raspberrypi.com/raspios_lite_arm64/images/raspios_lite_arm64-2024-03-15/2024-03-15-raspios-bookworm-arm64-lite.img.xz"
BASE_IMAGE_DIR="${PROJECT_ROOT}/.cache/pi-images"
BASE_IMAGE="${BASE_IMAGE_DIR}/raspios-bookworm-arm64-lite.img"

# Built artifacts
PI_BINARY="${PROJECT_ROOT}/dist/pi/tether-server"
WEB_DIST="${PROJECT_ROOT}/web/dist"

# sdm configuration directory
SDM_CONFIG_DIR="${PROJECT_ROOT}/pi/sdm-config"

# Docker image for sdm (if not running on Linux)
SDM_DOCKER_IMAGE="tether-sdm-builder:latest"

# Hostname for the Pi
PI_HOSTNAME="tether"

# Default user credentials
PI_USER="tether"
PI_PASSWORD="tether"  # User should change this

USE_DOCKER=false

# -----------------------------------------------------------------------------
# Logging Functions
# -----------------------------------------------------------------------------

log_info() {
    echo -e "\033[34m[INFO]\033[0m $*"
}

log_success() {
    echo -e "\033[32m[SUCCESS]\033[0m $*"
}

log_warning() {
    echo -e "\033[33m[WARNING]\033[0m $*"
}

log_error() {
    echo -e "\033[31m[ERROR]\033[0m $*" >&2
}

die() {
    log_error "$*"
    exit 1
}

# -----------------------------------------------------------------------------
# Argument Parsing
# -----------------------------------------------------------------------------

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --base-image)
                BASE_IMAGE="$2"
                shift 2
                ;;
            --base-image=*)
                BASE_IMAGE="${1#*=}"
                shift
                ;;
            --output)
                OUTPUT_IMAGE="$2"
                shift 2
                ;;
            --output=*)
                OUTPUT_IMAGE="${1#*=}"
                shift
                ;;
            --in-docker)
                # Internal flag - we're running inside Docker
                shift
                ;;
            --help|-h)
                print_usage
                exit 0
                ;;
            *)
                die "Unknown argument: $1"
                ;;
        esac
    done
}

print_usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Build a complete Raspberry Pi SD card image for Tether.

Options:
    --base-image PATH   Path to base Raspberry Pi OS image
    --output PATH       Output image path (default: dist/tether-pi.img)
    -h, --help          Show this help message

Prerequisites:
    - Linux host or Docker with privileged access
    - sdm installed (https://github.com/gitbls/sdm)
    - Built tether-server binary (run 'make build-pi' first)
    - Built web UI (run 'make build-web' first)
EOF
}

# -----------------------------------------------------------------------------
# Pre-flight Checks
# -----------------------------------------------------------------------------

check_dependencies() {
    log_info "Checking dependencies..."

    # Check if running on Linux or if Docker is available
    if [[ "$(uname)" != "Linux" ]]; then
        log_warning "Not running on Linux. Will use Docker."

        if ! command -v docker &>/dev/null; then
            die "Docker required on non-Linux systems"
        fi

        USE_DOCKER=true
    else
        # Check for sdm
        if ! command -v sdm &>/dev/null; then
            die "sdm not found. Install from: https://github.com/gitbls/sdm"
        fi
    fi

    # Check built artifacts exist
    if [[ ! -f "${PI_BINARY}" ]]; then
        die "Pi binary not found: ${PI_BINARY}
Run 'make build-pi' first."
    fi

    if [[ ! -d "${WEB_DIST}" ]]; then
        die "Web dist not found: ${WEB_DIST}
Run 'make build-web' first."
    fi

    log_success "Dependency check passed"
}

# -----------------------------------------------------------------------------
# Download Base Image
# -----------------------------------------------------------------------------

download_base_image() {
    if [[ -f "${BASE_IMAGE}" ]]; then
        log_info "Base image already exists: ${BASE_IMAGE}"
        return 0
    fi

    log_info "Downloading Raspberry Pi OS base image..."

    mkdir -p "${BASE_IMAGE_DIR}"

    local compressed_image="${BASE_IMAGE}.xz"

    # Download
    curl -L -o "${compressed_image}" "${BASE_IMAGE_URL}"

    # Decompress
    log_info "Decompressing image..."
    xz -d "${compressed_image}"

    log_success "Base image downloaded: ${BASE_IMAGE}"
}

# -----------------------------------------------------------------------------
# Create sdm Configuration
# -----------------------------------------------------------------------------

create_sdm_config() {
    log_info "Creating sdm configuration..."

    mkdir -p "${SDM_CONFIG_DIR}"

    # Create first-boot script
    cat > "${SDM_CONFIG_DIR}/01-tether-install.sh" <<'SCRIPT'
#!/bin/bash
# Tether installation script - runs during sdm customization

# Install required packages
apt-get update
apt-get install -y \
    bluez \
    bluetooth \
    libbluetooth-dev \
    hostapd \
    dnsmasq \
    nginx-light

# Create tether user if not exists
if ! id -u tether &>/dev/null; then
    useradd -m -s /bin/bash tether
    echo "tether:tether" | chpasswd
    usermod -aG sudo,bluetooth tether
fi

# Create directories
mkdir -p /opt/tether/bin
mkdir -p /opt/tether/web
mkdir -p /var/lib/tether
mkdir -p /var/log/tether
mkdir -p /etc/tether

# Set permissions
chown -R tether:tether /opt/tether
chown -R tether:tether /var/lib/tether
chown -R tether:tether /var/log/tether
chown -R tether:tether /etc/tether

echo "Tether installation complete"
SCRIPT

    chmod +x "${SDM_CONFIG_DIR}/01-tether-install.sh"

    # Create systemd service file
    cat > "${SDM_CONFIG_DIR}/tether.service" <<'SERVICE'
[Unit]
Description=Tether Server
After=network.target bluetooth.target
Wants=bluetooth.target

[Service]
Type=simple
User=tether
Group=tether
Environment="RUST_LOG=info"
Environment="TETHER_CONFIG=/etc/tether/config.toml"
ExecStart=/opt/tether/bin/tether-server
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SERVICE

    log_success "sdm configuration created"
}

# -----------------------------------------------------------------------------
# Build Image with sdm
# -----------------------------------------------------------------------------

build_with_sdm() {
    log_info "Building image with sdm..."

    # Copy base image to work on
    mkdir -p "$(dirname "${OUTPUT_IMAGE}")"
    cp "${BASE_IMAGE}" "${OUTPUT_IMAGE}"

    # Run sdm customize
    sudo sdm --customize "${OUTPUT_IMAGE}" \
        --hostname "${PI_HOSTNAME}" \
        --user "${PI_USER}" \
        --password "${PI_PASSWORD}" \
        --locale "en_US.UTF-8" \
        --timezone "UTC" \
        --ssh service \
        --extend 1024 \
        --script "${SDM_CONFIG_DIR}/01-tether-install.sh"

    # Copy binaries into image
    sudo sdm --mount "${OUTPUT_IMAGE}"

    # Copy tether binary
    sudo cp "${PI_BINARY}" /mnt/sdm/opt/tether/bin/
    sudo chmod +x /mnt/sdm/opt/tether/bin/tether-server

    # Copy web dist
    sudo cp -r "${WEB_DIST}"/* /mnt/sdm/opt/tether/web/

    # Copy systemd service
    sudo cp "${SDM_CONFIG_DIR}/tether.service" /mnt/sdm/etc/systemd/system/
    sudo ln -sf /etc/systemd/system/tether.service /mnt/sdm/etc/systemd/system/multi-user.target.wants/

    # Create default config
    sudo tee /mnt/sdm/etc/tether/config.toml > /dev/null <<'CONFIG'
# Tether Configuration
# This file is created on first boot - modify as needed

[server]
bind_address = "0.0.0.0:3000"
web_root = "/opt/tether/web"

[bluetooth]
# Configured via web UI on first boot
target_device = ""
rssi_threshold = -70

[passes]
monthly_allowance = 3

[storage]
data_dir = "/var/lib/tether"
log_file = "/var/log/tether/tether.log"
CONFIG

    sudo sdm --unmount "${OUTPUT_IMAGE}"

    log_success "Image built: ${OUTPUT_IMAGE}"
}

# -----------------------------------------------------------------------------
# Build with Docker (for non-Linux hosts)
# -----------------------------------------------------------------------------

build_with_docker() {
    log_info "Building image using Docker..."

    # Create Dockerfile for sdm if needed
    local dockerfile="${PROJECT_ROOT}/pi/Dockerfile.sdm"
    if [[ ! -f "${dockerfile}" ]]; then
        mkdir -p "${PROJECT_ROOT}/pi"
        cat > "${dockerfile}" <<'DOCKERFILE'
FROM debian:bookworm

RUN apt-get update && apt-get install -y \
    curl \
    xz-utils \
    qemu-user-static \
    binfmt-support \
    parted \
    dosfstools \
    && rm -rf /var/lib/apt/lists/*

# Install sdm
RUN curl -L https://raw.githubusercontent.com/gitbls/sdm/master/EZsdmInstaller | bash

WORKDIR /build
ENTRYPOINT ["/bin/bash"]
DOCKERFILE
    fi

    # Build Docker image for sdm if not exists
    if ! docker image inspect "${SDM_DOCKER_IMAGE}" &>/dev/null; then
        log_info "Building sdm Docker image..."
        docker build -t "${SDM_DOCKER_IMAGE}" -f "${dockerfile}" "${PROJECT_ROOT}/pi"
    fi

    mkdir -p "$(dirname "${OUTPUT_IMAGE}")"

    # Run sdm in Docker
    docker run --rm --privileged \
        -v "${PROJECT_ROOT}:/project" \
        -v "${BASE_IMAGE_DIR}:/cache" \
        -v "$(dirname "${OUTPUT_IMAGE}"):/output" \
        "${SDM_DOCKER_IMAGE}" \
        /project/scripts/build-image.sh --in-docker

    log_success "Image built: ${OUTPUT_IMAGE}"
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    parse_args "$@"

    # Check if running inside Docker
    if [[ "${1:-}" == "--in-docker" ]]; then
        # We're inside Docker, run sdm directly
        build_with_sdm
        exit 0
    fi

    log_info "========================================"
    log_info "TETHER - Raspberry Pi Image Builder"
    log_info "========================================"

    check_dependencies
    download_base_image
    create_sdm_config

    if [[ "${USE_DOCKER}" == true ]]; then
        build_with_docker
    else
        build_with_sdm
    fi

    # Print image info
    local image_size
    image_size=$(du -h "${OUTPUT_IMAGE}" | cut -f1)

    log_success "========================================"
    log_success "Image build complete!"
    log_success "========================================"
    log_info "Output:   ${OUTPUT_IMAGE}"
    log_info "Size:     ${image_size}"
    log_info ""
    log_info "Flash to SD card:"
    log_info "  sudo dd if=${OUTPUT_IMAGE} of=/dev/sdX bs=4M status=progress"
    log_info ""
    log_info "Or use Raspberry Pi Imager"
    log_success "========================================"
}

main "$@"
