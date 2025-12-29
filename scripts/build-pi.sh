#!/usr/bin/env bash
#
# build-pi.sh - Cross-compile tether-server for Raspberry Pi Zero 2 W
#
# Target: armv7-unknown-linux-gnueabihf (32-bit ARM)
# Uses: cross-rs (https://github.com/cross-rs/cross)
#
# Prerequisites:
#   - Docker or Podman running
#   - cargo install cross --git https://github.com/cross-rs/cross
#
# Usage:
#   ./scripts/build-pi.sh           # Build and package
#   ./scripts/build-pi.sh --clean   # Clean and rebuild
#   ./scripts/build-pi.sh --help    # Show help

set -euo pipefail

# Configuration
TARGET="armv7-unknown-linux-gnueabihf"
BINARY_NAME="tether-server"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${PROJECT_ROOT}/target/pi"
RELEASE_DIR="${PROJECT_ROOT}/target/${TARGET}/release"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_ok() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Cross-compile tether-server for Raspberry Pi Zero 2 W (32-bit ARM)

Options:
    --clean     Clean build directory before building
    --help      Show this help message

Environment Variables:
    PI_HOST     Raspberry Pi SSH host for deployment (e.g., pi@192.168.1.100)

Examples:
    $(basename "$0")                    # Build tether-server
    $(basename "$0") --clean            # Clean and build
    PI_HOST=pi@tether.local $(basename "$0") && make deploy-pi
EOF
}

# Check if cross is installed
check_cross() {
    if ! command -v cross &> /dev/null; then
        log_warn "cross is not installed. Installing..."

        # Install cross from the git repository (recommended for latest features)
        cargo install cross --git https://github.com/cross-rs/cross

        if [ $? -ne 0 ]; then
            log_error "Failed to install cross"
            log_info "Alternative: cargo install cross"
            exit 1
        fi

        log_ok "cross installed successfully"
    else
        log_info "cross is already installed: $(cross --version 2>&1 | head -1)"
    fi
}

# Check if Docker is running
check_docker() {
    if ! docker info &> /dev/null; then
        log_error "Docker is not running. Please start Docker first."
        log_info "On macOS: open -a Docker"
        log_info "On Linux: sudo systemctl start docker"
        exit 1
    fi
    log_ok "Docker is running"
}

# Build the project
build() {
    log_info "Building ${BINARY_NAME} for ${TARGET}..."

    cd "${PROJECT_ROOT}"

    # Set environment variables for cross-compilation
    export PKG_CONFIG_ALLOW_CROSS=1
    export CARGO_TERM_COLOR=always

    # Build with cross
    # Using --release for optimized binary
    # Using -p to specify the package in workspace
    cross build \
        --release \
        --target "${TARGET}" \
        -p tether-server \
        2>&1 | tee "${PROJECT_ROOT}/build-pi.log"

    if [ ${PIPESTATUS[0]} -ne 0 ]; then
        log_error "Build failed! Check build-pi.log for details"
        exit 1
    fi

    log_ok "Build completed successfully"
}

# Copy and strip the binary
package() {
    log_info "Packaging binary..."

    # Create output directory
    mkdir -p "${OUTPUT_DIR}"

    # Source binary path
    BINARY_PATH="${RELEASE_DIR}/${BINARY_NAME}"

    if [ ! -f "${BINARY_PATH}" ]; then
        log_error "Binary not found at ${BINARY_PATH}"
        exit 1
    fi

    # Copy binary to output directory
    cp "${BINARY_PATH}" "${OUTPUT_DIR}/${BINARY_NAME}"

    # Get original size
    ORIGINAL_SIZE=$(ls -lh "${OUTPUT_DIR}/${BINARY_NAME}" | awk '{print $5}')
    log_info "Original binary size: ${ORIGINAL_SIZE}"

    # Strip the binary using the cross-compilation strip tool
    # Note: If you have arm-linux-gnueabihf-strip installed locally, use it
    # Otherwise, we strip inside Docker
    if command -v arm-linux-gnueabihf-strip &> /dev/null; then
        log_info "Stripping binary with arm-linux-gnueabihf-strip..."
        arm-linux-gnueabihf-strip "${OUTPUT_DIR}/${BINARY_NAME}"
    else
        log_info "Stripping binary inside Docker container..."
        docker run --rm \
            -v "${OUTPUT_DIR}:/work" \
            ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:main \
            arm-linux-gnueabihf-strip /work/${BINARY_NAME} 2>/dev/null || {
                log_warn "Could not strip binary (may already be stripped via profile.release)"
            }
    fi

    # Get stripped size
    STRIPPED_SIZE=$(ls -lh "${OUTPUT_DIR}/${BINARY_NAME}" | awk '{print $5}')
    log_ok "Final binary size: ${STRIPPED_SIZE}"

    log_ok "Binary ready at: ${OUTPUT_DIR}/${BINARY_NAME}"
}

# Verify the binary
verify() {
    log_info "Verifying binary..."

    BINARY_PATH="${OUTPUT_DIR}/${BINARY_NAME}"

    # Check file type
    FILE_TYPE=$(file "${BINARY_PATH}")
    log_info "File type: ${FILE_TYPE}"

    # Verify it's an ARM32 ELF binary
    if echo "${FILE_TYPE}" | grep -q "ELF 32-bit LSB.*ARM"; then
        log_ok "Binary is correct architecture (ARM32/armhf)"
    else
        log_error "Binary may not be the correct architecture!"
        log_error "Expected: ELF 32-bit LSB executable, ARM, EABI5"
        exit 1
    fi
}

# Print summary
summary() {
    echo ""
    echo "=========================================="
    echo "  Build Summary"
    echo "=========================================="
    echo "  Target:     ${TARGET}"
    echo "  Binary:     ${OUTPUT_DIR}/${BINARY_NAME}"
    echo "  Size:       $(ls -lh "${OUTPUT_DIR}/${BINARY_NAME}" | awk '{print $5}')"
    echo ""
    echo "  To deploy to Raspberry Pi:"
    echo "    scp ${OUTPUT_DIR}/${BINARY_NAME} pi@<pi-ip>:/home/pi/"
    echo ""
    echo "  Or set PI_HOST and run:"
    echo "    PI_HOST=pi@tether.local make deploy-pi"
    echo ""
    echo "=========================================="
}

# Main execution
main() {
    local clean=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --clean)
                clean=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    echo "=========================================="
    echo "  Tether - Raspberry Pi Build"
    echo "  Target: Pi Zero 2 W (32-bit armhf)"
    echo "=========================================="
    echo ""

    # Clean if requested
    if [ "$clean" = true ]; then
        log_info "Cleaning build directory..."
        rm -rf "${PROJECT_ROOT}/target/${TARGET}"
        rm -rf "${OUTPUT_DIR}"
        log_ok "Clean complete"
    fi

    check_docker
    check_cross
    build
    package
    verify
    summary

    log_ok "Done!"
}

main "$@"
