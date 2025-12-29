#!/bin/bash
#
# Tether - Raspberry Pi OS Image Builder using sdm
# Builds a fully configured image for Pi Zero 2 W
#
# Usage: sudo ./build-image.sh [--skip-download] [--clean]
#
# Prerequisites:
#   - Linux system with sdm installed (/usr/local/sdm/sdm)
#   - Cross-compiled binaries in target/armv7-unknown-linux-gnueabihf/release/
#   - Web UI built in web-ui/dist/
#   - At least 10GB free disk space
#
set -euo pipefail

#=============================================================================
# Configuration
#=============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

# Raspberry Pi OS Lite (32-bit armhf) - Required for Pi Zero 2 W
# Using 32-bit because Pi Zero 2 W has only 512MB RAM
# Note: Update this URL when newer images are released
IMAGE_URL="https://downloads.raspberrypi.com/raspios_lite_armhf/images/raspios_lite_armhf-2024-11-19/2024-11-19-raspios-bookworm-armhf-lite.img.xz"
IMAGE_FILENAME="2024-11-19-raspios-bookworm-armhf-lite.img.xz"
IMAGE_UNCOMPRESSED="2024-11-19-raspios-bookworm-armhf-lite.img"

# Directories
DOWNLOAD_DIR="${SCRIPT_DIR}/downloads"
OUTPUT_DIR="${SCRIPT_DIR}/output"
ASSETS_DIR="${SCRIPT_DIR}/assets"
CONFIGS_DIR="${SCRIPT_DIR}/configs"
PLUGINS_DIR="${SCRIPT_DIR}/plugins"

# Source binaries (from cross-compilation)
CROSS_TARGET="armv7-unknown-linux-gnueabihf"
BIN_DIR="${PROJECT_ROOT}/target/${CROSS_TARGET}/release"
WEBUI_DIR="${PROJECT_ROOT}/web-ui/dist"

# Output image name
OUTPUT_IMAGE="tether-pi-$(date +%Y%m%d-%H%M%S).img"

# sdm path
SDM="/usr/local/sdm/sdm"

#=============================================================================
# Color Output
#=============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

#=============================================================================
# Preflight Checks
#=============================================================================

preflight_checks() {
    log_info "Running preflight checks..."

    # Must be root
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root (sudo)"
        exit 1
    fi

    # Check sdm is installed
    if [[ ! -x "${SDM}" ]]; then
        log_error "sdm not found at ${SDM}"
        log_error "Install with: curl -L https://raw.githubusercontent.com/gitbls/sdm/master/install-sdm | sudo bash"
        exit 1
    fi
    log_ok "sdm found at ${SDM}"

    # Check cross-compiled binary exists
    if [[ ! -f "${BIN_DIR}/tether-server" ]]; then
        log_error "tether-server binary not found at ${BIN_DIR}/tether-server"
        log_error "Run: ./scripts/build-pi.sh"
        exit 1
    fi
    log_ok "tether-server binary found"

    # Check web UI build exists
    if [[ ! -d "${WEBUI_DIR}" ]] || [[ ! -f "${WEBUI_DIR}/index.html" ]]; then
        log_error "Web UI build not found at ${WEBUI_DIR}"
        log_error "Run: cd web-ui && bun run build"
        exit 1
    fi
    log_ok "Web UI build found"

    # Check disk space (need at least 8GB free)
    AVAILABLE_KB=$(df "${SCRIPT_DIR}" | awk 'NR==2 {print $4}')
    AVAILABLE_GB=$((AVAILABLE_KB / 1024 / 1024))
    if [[ ${AVAILABLE_GB} -lt 8 ]]; then
        log_error "Insufficient disk space. Need at least 8GB, have ${AVAILABLE_GB}GB"
        exit 1
    fi
    log_ok "Disk space: ${AVAILABLE_GB}GB available"

    # Check plugin exists
    if [[ ! -f "${PLUGINS_DIR}/tether" ]]; then
        log_error "tether plugin not found at ${PLUGINS_DIR}/tether"
        exit 1
    fi
    log_ok "tether plugin found"
}

#=============================================================================
# Download Base Image
#=============================================================================

download_base_image() {
    log_info "Checking base image..."

    mkdir -p "${DOWNLOAD_DIR}"

    if [[ -f "${DOWNLOAD_DIR}/${IMAGE_UNCOMPRESSED}" ]]; then
        log_ok "Base image already exists: ${IMAGE_UNCOMPRESSED}"
        return 0
    fi

    if [[ -f "${DOWNLOAD_DIR}/${IMAGE_FILENAME}" ]]; then
        log_info "Compressed image exists, decompressing..."
    else
        log_info "Downloading Raspberry Pi OS Lite..."
        log_info "URL: ${IMAGE_URL}"

        wget --progress=bar:force:noscroll \
            -O "${DOWNLOAD_DIR}/${IMAGE_FILENAME}" \
            "${IMAGE_URL}"

        log_ok "Download complete"
    fi

    log_info "Decompressing image (this takes a few minutes)..."
    xz -dk "${DOWNLOAD_DIR}/${IMAGE_FILENAME}"
    log_ok "Decompression complete"
}

#=============================================================================
# Prepare Assets
#=============================================================================

prepare_assets() {
    log_info "Preparing assets for image..."

    # Create assets directory structure
    mkdir -p "${ASSETS_DIR}/bin"
    mkdir -p "${ASSETS_DIR}/web-ui"

    # Copy tether-server binary
    log_info "Copying tether-server binary..."
    cp "${BIN_DIR}/tether-server" "${ASSETS_DIR}/bin/"
    chmod 755 "${ASSETS_DIR}/bin/tether-server"

    # Copy dumbpipe binary if it exists in our build, otherwise it will be downloaded
    if [[ -f "${BIN_DIR}/dumbpipe" ]]; then
        log_info "Copying dumbpipe binary..."
        cp "${BIN_DIR}/dumbpipe" "${ASSETS_DIR}/bin/"
        chmod 755 "${ASSETS_DIR}/bin/dumbpipe"
    else
        log_warn "dumbpipe binary not found in build output"
        log_warn "Will need to download or build separately"
    fi

    # Copy web UI
    log_info "Copying web UI..."
    rsync -a --delete "${WEBUI_DIR}/" "${ASSETS_DIR}/web-ui/"

    log_ok "Assets prepared"
}

#=============================================================================
# Create Working Image Copy
#=============================================================================

create_working_copy() {
    log_info "Creating working copy of base image..."

    mkdir -p "${OUTPUT_DIR}"

    WORK_IMAGE="${OUTPUT_DIR}/${OUTPUT_IMAGE}"

    cp "${DOWNLOAD_DIR}/${IMAGE_UNCOMPRESSED}" "${WORK_IMAGE}"

    log_ok "Working image created: ${WORK_IMAGE}"
}

#=============================================================================
# Run sdm Customization
#=============================================================================

run_sdm_customize() {
    log_info "Running sdm customization..."
    log_info "This will take 10-20 minutes depending on network speed..."

    WORK_IMAGE="${OUTPUT_DIR}/${OUTPUT_IMAGE}"

    # Extend image to have room for customization
    # Pi OS Lite is ~2GB, we need ~4GB for all our additions
    log_info "Extending image by 2048MB..."
    ${SDM} --extend --xmb 2048 "${WORK_IMAGE}"

    #=========================================================================
    # CRITICAL: Plugin order matters!
    #
    # Order explanation:
    # 1. L10n - Must be first to set locale before any package installation
    # 2. user - Create tether user before copying files with ownership
    # 3. apps - Install system packages (nginx, bluez, etc.)
    # 4. raspiconfig - Enable hardware interfaces (bluetooth, etc.)
    # 5. copyfile - Copy all our assets into the image
    # 6. tether (custom) - Configure tether-specific setup
    # 7. system - Enable/disable services (must be after tether creates them)
    # 8. network - Configure primary network settings
    # 9. chrony - Time sync (can be anywhere after apps)
    #=========================================================================

    ${SDM} --customize "${WORK_IMAGE}" \
        \
        --plugin L10n:"keymap=us|locale=en_US.UTF-8|timezone=UTC|wificountry=US" \
        \
        --plugin user:"adduser=tether|password=tether|groups=sudo,bluetooth,netdev,gpio,i2c,spi|homedir=/home/tether" \
        \
        --plugin apps:"apps=@${CONFIGS_DIR}/apps.txt|name=tether-apps" \
        \
        --plugin raspiconfig:"serial=1|i2c=1|spi=1" \
        \
        --plugin copyfile:"filelist=${CONFIGS_DIR}/copyfiles.txt" \
        \
        --plugin "${PLUGINS_DIR}/tether" \
        \
        --plugin system:"service-enable=tether-server,tether-dumbpipe,tether-network-watchdog,nginx|service-disable=bluetooth|swap=0" \
        \
        --plugin chrony:"nodistsources" \
        \
        --regen-ssh-host-keys \
        --restart

    log_ok "sdm customization complete"
}

#=============================================================================
# Shrink Final Image
#=============================================================================

shrink_image() {
    log_info "Shrinking final image to minimum size..."

    WORK_IMAGE="${OUTPUT_DIR}/${OUTPUT_IMAGE}"

    ${SDM} --shrink "${WORK_IMAGE}"

    # Get final size
    FINAL_SIZE=$(du -h "${WORK_IMAGE}" | cut -f1)
    log_ok "Final image size: ${FINAL_SIZE}"
}

#=============================================================================
# Generate SHA256 Checksum
#=============================================================================

generate_checksum() {
    log_info "Generating SHA256 checksum..."

    WORK_IMAGE="${OUTPUT_DIR}/${OUTPUT_IMAGE}"

    sha256sum "${WORK_IMAGE}" > "${WORK_IMAGE}.sha256"

    log_ok "Checksum saved to ${WORK_IMAGE}.sha256"
}

#=============================================================================
# Main
#=============================================================================

main() {
    echo "=============================================="
    echo "  Tether - Raspberry Pi Image Builder"
    echo "=============================================="
    echo ""

    # Parse arguments
    SKIP_DOWNLOAD=false
    CLEAN=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-download)
                SKIP_DOWNLOAD=true
                shift
                ;;
            --clean)
                CLEAN=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Clean if requested
    if [[ "${CLEAN}" == "true" ]]; then
        log_info "Cleaning output directory..."
        rm -rf "${OUTPUT_DIR}"
        log_ok "Clean complete"
    fi

    # Run build steps
    preflight_checks

    if [[ "${SKIP_DOWNLOAD}" != "true" ]]; then
        download_base_image
    fi

    prepare_assets
    create_working_copy
    run_sdm_customize
    shrink_image
    generate_checksum

    echo ""
    echo "=============================================="
    log_ok "BUILD COMPLETE"
    echo "=============================================="
    echo ""
    echo "Output image: ${OUTPUT_DIR}/${OUTPUT_IMAGE}"
    echo "Checksum:     ${OUTPUT_DIR}/${OUTPUT_IMAGE}.sha256"
    echo ""
    echo "To flash to SD card:"
    echo "  sudo dd if=${OUTPUT_DIR}/${OUTPUT_IMAGE} of=/dev/sdX bs=4M status=progress conv=fsync"
    echo ""
    echo "Or use Raspberry Pi Imager."
    echo ""
}

main "$@"
