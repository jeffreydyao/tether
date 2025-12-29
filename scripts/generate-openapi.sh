#!/usr/bin/env bash
# =============================================================================
# TETHER - OpenAPI Generator
# =============================================================================
# Generates OpenAPI specification from Rust code and optionally creates
# a TypeScript client.
#
# PREREQUISITES:
# - Cargo/Rust installed
# - bun (for TypeScript client generation)
#
# USAGE:
#   ./scripts/generate-openapi.sh [OPTIONS]
#
# OPTIONS:
#   -o, --output PATH       Output file path (default: openapi.json)
#   -f, --format FORMAT     Output format: json or yaml (default: json)
#   --client-dir PATH       TypeScript client output directory
#   --skip-client           Skip TypeScript client generation
#   -h, --help              Show this help message
#
# =============================================================================

set -euo pipefail

# -----------------------------------------------------------------------------
# Configuration
# -----------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# The Rust package that exports OpenAPI
OPENAPI_PACKAGE="tether-server"

# Output configuration
OPENAPI_OUTPUT_FILE="${PROJECT_ROOT}/openapi.json"
OUTPUT_FORMAT="json"
CLIENT_OUTPUT_DIR="${PROJECT_ROOT}/web-ui/src/api"
SKIP_CLIENT=false

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
            --output|-o)
                OPENAPI_OUTPUT_FILE="$2"
                shift 2
                ;;
            --output=*)
                OPENAPI_OUTPUT_FILE="${1#*=}"
                shift
                ;;
            --format|-f)
                OUTPUT_FORMAT="$2"
                shift 2
                ;;
            --format=*)
                OUTPUT_FORMAT="${1#*=}"
                shift
                ;;
            --client-dir)
                CLIENT_OUTPUT_DIR="$2"
                shift 2
                ;;
            --client-dir=*)
                CLIENT_OUTPUT_DIR="${1#*=}"
                shift
                ;;
            --skip-client)
                SKIP_CLIENT=true
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

Generate OpenAPI specification from Rust code.

Options:
    -o, --output PATH       Output file path (default: openapi.json)
    -f, --format FORMAT     Output format: json or yaml (default: json)
    --client-dir PATH       TypeScript client output directory
    --skip-client           Skip TypeScript client generation
    -h, --help              Show this help message

Environment Variables:
    OPENAPI_OUTPUT_FILE  Path to output OpenAPI spec
    CLIENT_OUTPUT_DIR    Path to generated TypeScript client

The OpenAPI spec is extracted from the Rust code using utoipa annotations.
The binary is run with --export-openapi flag to output the spec to stdout.
EOF
}

# -----------------------------------------------------------------------------
# Pre-flight Checks
# -----------------------------------------------------------------------------

check_dependencies() {
    log_info "Checking dependencies..."

    # Check Rust/Cargo
    if ! command -v cargo &>/dev/null; then
        die "cargo not found. Install Rust from https://rustup.rs"
    fi

    # Check project has the required package
    if ! grep -q "name = \"${OPENAPI_PACKAGE}\"" "${PROJECT_ROOT}/Cargo.toml" 2>/dev/null; then
        # Check in workspace members
        if [[ ! -d "${PROJECT_ROOT}/crates/${OPENAPI_PACKAGE}" ]]; then
            die "Package '${OPENAPI_PACKAGE}' not found in project"
        fi
    fi

    log_success "Dependencies check passed"
}

# -----------------------------------------------------------------------------
# Build OpenAPI Exporter
# -----------------------------------------------------------------------------

build_exporter() {
    log_info "Building OpenAPI exporter..."

    (
        cd "${PROJECT_ROOT}"

        # Build the package with release optimizations
        cargo build --release --package "${OPENAPI_PACKAGE}" 2>&1 || \
            cargo build --package "${OPENAPI_PACKAGE}" 2>&1
    )

    log_success "Exporter built successfully"
}

# -----------------------------------------------------------------------------
# Generate OpenAPI Specification
# -----------------------------------------------------------------------------

generate_openapi() {
    log_info "Generating OpenAPI specification..."

    # Create output directory if it doesn't exist
    mkdir -p "$(dirname "${OPENAPI_OUTPUT_FILE}")"

    # Temporary file for output
    local temp_file
    temp_file=$(mktemp)

    # Cleanup on exit
    trap "rm -f ${temp_file}" EXIT

    (
        cd "${PROJECT_ROOT}"

        # Method 1: Run the binary with --openapi flag
        if cargo run --release --package "${OPENAPI_PACKAGE}" -- --openapi > "${temp_file}" 2>/dev/null; then
            log_info "Generated via --openapi flag"
        # Method 2: Use environment variable
        elif TETHER_EXPORT_OPENAPI=1 cargo run --release --package "${OPENAPI_PACKAGE}" > "${temp_file}" 2>/dev/null; then
            log_info "Generated via TETHER_EXPORT_OPENAPI environment variable"
        else
            die "Failed to generate OpenAPI specification.

Ensure your Rust code handles one of:
1. --openapi CLI flag
2. TETHER_EXPORT_OPENAPI env var"
        fi
    )

    # Validate the output is valid JSON
    if ! python3 -c "import json; json.load(open('${temp_file}'))" 2>/dev/null; then
        if ! jq empty "${temp_file}" 2>/dev/null; then
            log_error "Generated file is not valid JSON"
            log_error "Content preview:"
            head -20 "${temp_file}" >&2
            die "OpenAPI generation produced invalid JSON"
        fi
    fi

    # Validate it looks like an OpenAPI spec
    if ! grep -q '"openapi"' "${temp_file}" && ! grep -q '"swagger"' "${temp_file}"; then
        log_error "Generated file does not appear to be an OpenAPI spec"
        log_error "Content preview:"
        head -20 "${temp_file}" >&2
        die "Output does not contain 'openapi' or 'swagger' field"
    fi

    # Convert to YAML if requested
    if [[ "${OUTPUT_FORMAT}" == "yaml" ]]; then
        log_info "Converting to YAML format..."
        if command -v yq &>/dev/null; then
            yq -P "${temp_file}" > "${OPENAPI_OUTPUT_FILE}"
        elif command -v python3 &>/dev/null; then
            python3 -c "
import json
import yaml
with open('${temp_file}') as f:
    data = json.load(f)
with open('${OPENAPI_OUTPUT_FILE}', 'w') as f:
    yaml.dump(data, f, default_flow_style=False, allow_unicode=True)
"
        else
            die "YAML conversion requested but neither yq nor python3+pyyaml available"
        fi
    else
        # Pretty-print JSON
        if command -v jq &>/dev/null; then
            jq '.' "${temp_file}" > "${OPENAPI_OUTPUT_FILE}"
        elif command -v python3 &>/dev/null; then
            python3 -c "
import json
with open('${temp_file}') as f:
    data = json.load(f)
with open('${OPENAPI_OUTPUT_FILE}', 'w') as f:
    json.dump(data, f, indent=2)
"
        else
            cp "${temp_file}" "${OPENAPI_OUTPUT_FILE}"
        fi
    fi

    log_success "OpenAPI spec written to: ${OPENAPI_OUTPUT_FILE}"
}

# -----------------------------------------------------------------------------
# Generate TypeScript Client
# -----------------------------------------------------------------------------

generate_typescript_client() {
    if [[ "${SKIP_CLIENT}" == true ]]; then
        log_info "Skipping TypeScript client generation (--skip-client)"
        return 0
    fi

    log_info "Generating TypeScript client from OpenAPI spec..."

    # Check if web directory exists (might be building MCP only)
    if [[ ! -d "${PROJECT_ROOT}/web-ui" ]]; then
        log_warning "Web directory not found. Skipping TypeScript client generation."
        return 0
    fi

    # Check for bun
    if ! command -v bun &>/dev/null; then
        log_warning "bun not found. Skipping TypeScript client generation."
        log_warning "Install bun and run 'bunx @hey-api/openapi-ts' manually"
        return 0
    fi

    # Create client output directory
    mkdir -p "${CLIENT_OUTPUT_DIR}"

    (
        cd "${PROJECT_ROOT}/web-ui"

        # Generate TypeScript client using @hey-api/openapi-ts
        bunx @hey-api/openapi-ts \
            --input "${OPENAPI_OUTPUT_FILE}" \
            --output "${CLIENT_OUTPUT_DIR}" \
            --client @hey-api/client-fetch
    )

    # Verify generation succeeded
    if [[ -f "${CLIENT_OUTPUT_DIR}/index.ts" ]]; then
        log_success "TypeScript client generated at: ${CLIENT_OUTPUT_DIR}"
    else
        log_warning "TypeScript client generation may have failed. Check ${CLIENT_OUTPUT_DIR}"
    fi
}

# -----------------------------------------------------------------------------
# Print OpenAPI Info
# -----------------------------------------------------------------------------

print_openapi_info() {
    log_info "========================================"
    log_info "OpenAPI Specification Info"
    log_info "========================================"

    local title version paths

    if command -v jq &>/dev/null; then
        title=$(jq -r '.info.title // "Unknown"' "${OPENAPI_OUTPUT_FILE}")
        version=$(jq -r '.info.version // "Unknown"' "${OPENAPI_OUTPUT_FILE}")
        paths=$(jq -r '.paths | keys | length' "${OPENAPI_OUTPUT_FILE}")
    elif command -v python3 &>/dev/null; then
        title=$(python3 -c "import json; d=json.load(open('${OPENAPI_OUTPUT_FILE}')); print(d.get('info',{}).get('title','Unknown'))")
        version=$(python3 -c "import json; d=json.load(open('${OPENAPI_OUTPUT_FILE}')); print(d.get('info',{}).get('version','Unknown'))")
        paths=$(python3 -c "import json; d=json.load(open('${OPENAPI_OUTPUT_FILE}')); print(len(d.get('paths',{})))")
    else
        title="(jq/python3 required to parse)"
        version="(jq/python3 required to parse)"
        paths="(jq/python3 required to parse)"
    fi

    log_info "Title:      ${title}"
    log_info "Version:    ${version}"
    log_info "Endpoints:  ${paths}"
    log_info "Output:     ${OPENAPI_OUTPUT_FILE}"
    if [[ "${SKIP_CLIENT}" != true ]]; then
        log_info "Client:     ${CLIENT_OUTPUT_DIR}"
    fi
    log_info "========================================"
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------

main() {
    parse_args "$@"

    log_info "========================================"
    log_info "TETHER - OpenAPI Generator"
    log_info "========================================"

    # Pre-flight checks
    check_dependencies

    # Build the exporter
    build_exporter

    # Generate OpenAPI spec
    generate_openapi

    # Generate TypeScript client
    generate_typescript_client

    # Print info
    print_openapi_info

    log_success "========================================"
    log_success "OpenAPI generation complete!"
    log_success "========================================"
}

main "$@"
