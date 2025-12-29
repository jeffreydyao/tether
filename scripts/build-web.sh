#!/usr/bin/env bash
# =============================================================================
# TETHER - Web UI Build Script
# =============================================================================
# Builds the React web UI using Bun.
#
# PREREQUISITES:
# - Bun installed (https://bun.sh)
# - OpenAPI spec generated (or will be generated automatically)
#
# USAGE:
#   ./scripts/build-web.sh [OPTIONS]
#
# OPTIONS:
#   --skip-api    Skip TypeScript API client generation
#   --dev         Build in development mode (no minification)
#   -h, --help    Show this help message
#
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WEB_DIR="${PROJECT_ROOT}/web-ui"
WEB_DIST_DIR="${WEB_DIR}/dist"
CLIENT_OUTPUT_DIR="${WEB_DIR}/src/api"
OPENAPI_FILE="${PROJECT_ROOT}/openapi.json"

# Build options
SKIP_API=false
DEV_MODE=false

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
            --skip-api)
                SKIP_API=true
                shift
                ;;
            --dev)
                DEV_MODE=true
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

Build the Tether web UI.

Options:
    --skip-api      Skip TypeScript API client generation
    --dev           Build in development mode (no minification)
    -h, --help      Show this help message

The script will:
1. Check for bun and required dependencies
2. Generate TypeScript API client from OpenAPI spec (unless --skip-api)
3. Build the React application
4. Output to web/dist/
EOF
}

# -----------------------------------------------------------------------------
# Pre-flight Checks
# -----------------------------------------------------------------------------

check_dependencies() {
    log_info "Checking dependencies..."

    # Check for bun
    if ! command -v bun &>/dev/null; then
        die "bun not found. Install from: https://bun.sh"
    fi

    # Check web directory exists
    if [[ ! -d "${WEB_DIR}" ]]; then
        die "Web directory not found: ${WEB_DIR}"
    fi

    # Check package.json exists
    if [[ ! -f "${WEB_DIR}/package.json" ]]; then
        die "package.json not found in ${WEB_DIR}"
    fi

    log_success "Dependencies check passed"
}

# -----------------------------------------------------------------------------
# Install Dependencies
# -----------------------------------------------------------------------------

install_dependencies() {
    log_info "Installing npm dependencies..."

    (
        cd "${WEB_DIR}"
        bun install
    )

    log_success "Dependencies installed"
}

# -----------------------------------------------------------------------------
# Generate API Client
# -----------------------------------------------------------------------------

generate_api_client() {
    if [[ "${SKIP_API}" == true ]]; then
        log_info "Skipping API client generation (--skip-api)"
        return 0
    fi

    # Check if OpenAPI spec exists
    if [[ ! -f "${OPENAPI_FILE}" ]]; then
        log_warning "OpenAPI spec not found at ${OPENAPI_FILE}"
        log_info "Attempting to generate OpenAPI spec..."

        # Try to generate it
        if [[ -x "${SCRIPT_DIR}/generate-openapi.sh" ]]; then
            "${SCRIPT_DIR}/generate-openapi.sh" --skip-client
        else
            log_warning "generate-openapi.sh not found or not executable"
            log_warning "Proceeding without API client generation"
            return 0
        fi
    fi

    log_info "Generating TypeScript API client..."

    # Create client output directory
    mkdir -p "${CLIENT_OUTPUT_DIR}"

    (
        cd "${WEB_DIR}"

        # Generate TypeScript client using @hey-api/openapi-ts
        bunx @hey-api/openapi-ts \
            --input "${OPENAPI_FILE}" \
            --output "${CLIENT_OUTPUT_DIR}" \
            --client @hey-api/client-fetch
    )

    # Verify generation succeeded
    if [[ -f "${CLIENT_OUTPUT_DIR}/index.ts" ]]; then
        log_success "TypeScript client generated at: ${CLIENT_OUTPUT_DIR}"
    else
        log_warning "TypeScript client generation may have failed"
    fi
}

# -----------------------------------------------------------------------------
# Build Web UI
# -----------------------------------------------------------------------------

build_web() {
    log_info "Building web UI..."

    (
        cd "${WEB_DIR}"

        if [[ "${DEV_MODE}" == true ]]; then
            log_info "Building in development mode..."
            bun run build --mode development
        else
            bun run build
        fi
    )

    # Verify build succeeded
    if [[ -d "${WEB_DIST_DIR}" ]] && [[ -f "${WEB_DIST_DIR}/index.html" ]]; then
        log_success "Web UI built successfully"
    else
        die "Build failed - dist directory or index.html not found"
    fi
}

# -----------------------------------------------------------------------------
# Print Build Info
# -----------------------------------------------------------------------------

print_build_info() {
    log_info "========================================"
    log_info "Web UI Build Complete"
    log_info "========================================"

    local file_count
    local total_size

    file_count=$(find "${WEB_DIST_DIR}" -type f | wc -l | tr -d ' ')
    total_size=$(du -sh "${WEB_DIST_DIR}" | cut -f1)

    log_info "Output:     ${WEB_DIST_DIR}"
    log_info "Files:      ${file_count}"
    log_info "Total size: ${total_size}"
    log_info "========================================"
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    parse_args "$@"

    log_info "========================================"
    log_info "TETHER - Web UI Builder"
    log_info "========================================"

    check_dependencies
    install_dependencies
    generate_api_client
    build_web
    print_build_info

    log_success "========================================"
    log_success "Build complete!"
    log_success "========================================"
}

main "$@"
