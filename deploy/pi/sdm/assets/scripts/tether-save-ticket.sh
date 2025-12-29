#!/bin/bash
# tether-save-ticket.sh
#
# Wrapper script for dumbpipe that captures the generated ticket.
#
# When dumbpipe starts in listen-tcp mode, it prints the ticket to stdout
# on a single line. This script:
# 1. Starts dumbpipe in the background
# 2. Reads the first line of output (the ticket)
# 3. Saves the ticket to /opt/tether/data/dumbpipe-ticket.txt
# 4. Continues to relay dumbpipe's output to stdout (for journald)
# 5. Properly handles SIGTERM for graceful shutdown
#
# This script is executed by tether-dumbpipe.service

set -euo pipefail

# Configuration (can be overridden by environment variables from systemd)
DUMBPIPE_BIN="${DUMBPIPE_BIN:-/opt/tether/bin/dumbpipe}"
LOCAL_SERVER="${TETHER_LOCAL_SERVER:-127.0.0.1:3000}"
TICKET_FILE="${TETHER_TICKET_FILE:-/opt/tether/data/dumbpipe-ticket.txt}"
TICKET_TIMEOUT="${TICKET_TIMEOUT:-30}"

# PID of dumbpipe process (for signal handling)
DUMBPIPE_PID=""

# Logging functions
log_info() {
    echo "[INFO] $(date '+%Y-%m-%d %H:%M:%S') $*"
}

log_error() {
    echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') $*" >&2
}

log_debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo "[DEBUG] $(date '+%Y-%m-%d %H:%M:%S') $*"
    fi
}

# Signal handling
cleanup() {
    local signal="${1:-TERM}"
    log_info "Received SIG${signal}, shutting down..."

    if [[ -n "${DUMBPIPE_PID}" ]] && kill -0 "${DUMBPIPE_PID}" 2>/dev/null; then
        log_info "Sending SIGTERM to dumbpipe (PID: ${DUMBPIPE_PID})"
        kill -TERM "${DUMBPIPE_PID}" 2>/dev/null || true

        # Wait up to 5 seconds for graceful shutdown
        local wait_count=0
        while kill -0 "${DUMBPIPE_PID}" 2>/dev/null && [[ ${wait_count} -lt 50 ]]; do
            sleep 0.1
            ((wait_count++))
        done

        # Force kill if still running
        if kill -0 "${DUMBPIPE_PID}" 2>/dev/null; then
            log_info "Sending SIGKILL to dumbpipe"
            kill -KILL "${DUMBPIPE_PID}" 2>/dev/null || true
        fi
    fi

    log_info "Cleanup complete, exiting"
    exit 0
}

# Register signal handlers
trap 'cleanup TERM' SIGTERM
trap 'cleanup INT' SIGINT
trap 'cleanup HUP' SIGHUP

# Validate environment
validate_environment() {
    log_info "Validating environment..."

    if [[ ! -x "${DUMBPIPE_BIN}" ]]; then
        log_error "dumbpipe binary not found or not executable: ${DUMBPIPE_BIN}"
        exit 1
    fi

    local ticket_dir
    ticket_dir=$(dirname "${TICKET_FILE}")
    if [[ ! -d "${ticket_dir}" ]]; then
        log_error "Ticket directory does not exist: ${ticket_dir}"
        exit 1
    fi
    if [[ ! -w "${ticket_dir}" ]]; then
        log_error "Ticket directory is not writable: ${ticket_dir}"
        exit 1
    fi

    log_info "Environment validation passed"
}

# Ticket validation
is_valid_ticket() {
    local line="$1"

    # Dumbpipe tickets start with specific prefixes and are base32 encoded
    if [[ "${line}" =~ ^(endpoint|blobdownload|docsync)[a-z0-9]+ ]]; then
        if [[ ${#line} -ge 50 ]]; then
            return 0
        fi
    fi

    return 1
}

# Extract ticket from dumbpipe output
extract_ticket() {
    local input_fd="$1"
    local timeout="${2:-30}"
    local ticket=""
    local line=""
    local elapsed=0

    log_info "Waiting for ticket from dumbpipe (timeout: ${timeout}s)..."

    while [[ ${elapsed} -lt ${timeout} ]]; do
        if read -r -t 1 line <&"${input_fd}" 2>/dev/null; then
            log_debug "Read line: ${line:0:80}..."

            if is_valid_ticket "${line}"; then
                ticket="${line}"
                log_info "Found valid ticket (${#ticket} chars)"
                break
            fi

            # Also relay the line to stdout for logging
            echo "${line}"
        fi

        ((elapsed++)) || true
    done

    if [[ -z "${ticket}" ]]; then
        log_error "Failed to extract ticket within ${timeout} seconds"
        return 1
    fi

    echo "${ticket}"
}

# Save ticket to file
save_ticket() {
    local ticket="$1"
    local file="$2"
    local temp_file="${file}.tmp.$$"

    log_info "Saving ticket to ${file}..."

    # Write to temp file first (atomic write pattern)
    if ! echo "${ticket}" > "${temp_file}"; then
        log_error "Failed to write ticket to temp file: ${temp_file}"
        rm -f "${temp_file}" 2>/dev/null || true
        return 1
    fi

    # Set restrictive permissions
    chmod 600 "${temp_file}"

    # Atomic move
    if ! mv "${temp_file}" "${file}"; then
        log_error "Failed to move ticket to final location: ${file}"
        rm -f "${temp_file}" 2>/dev/null || true
        return 1
    fi

    log_info "Ticket saved successfully"
    return 0
}

# Main execution
main() {
    log_info "Starting tether-save-ticket.sh"
    log_info "  DUMBPIPE_BIN: ${DUMBPIPE_BIN}"
    log_info "  LOCAL_SERVER: ${LOCAL_SERVER}"
    log_info "  TICKET_FILE: ${TICKET_FILE}"

    validate_environment

    # Create a pipe for capturing dumbpipe output
    local pipe_dir
    pipe_dir=$(mktemp -d)
    local stdout_pipe="${pipe_dir}/stdout"
    mkfifo "${stdout_pipe}"

    # Cleanup function for pipe
    cleanup_pipe() {
        rm -f "${stdout_pipe}" 2>/dev/null || true
        rmdir "${pipe_dir}" 2>/dev/null || true
    }
    trap 'cleanup_pipe; cleanup TERM' SIGTERM SIGINT SIGHUP

    log_info "Starting dumbpipe listen-tcp..."

    # Start dumbpipe with output to our pipe
    "${DUMBPIPE_BIN}" listen-tcp --host "${LOCAL_SERVER}" 2>&1 > "${stdout_pipe}" &
    DUMBPIPE_PID=$!

    log_info "dumbpipe started with PID: ${DUMBPIPE_PID}"

    # Open pipe for reading
    exec 3< "${stdout_pipe}"

    # Extract ticket from the pipe
    local ticket
    if ! ticket=$(extract_ticket 3 "${TICKET_TIMEOUT}"); then
        log_error "Failed to extract ticket, terminating dumbpipe"
        cleanup TERM
        exit 1
    fi

    # Save the ticket
    if ! save_ticket "${ticket}" "${TICKET_FILE}"; then
        log_error "Failed to save ticket, but continuing (dumbpipe is running)"
    fi

    log_info "Ticket capture complete, relaying dumbpipe output..."

    # Continue relaying dumbpipe output until it exits or we receive a signal
    while kill -0 "${DUMBPIPE_PID}" 2>/dev/null; do
        if read -r -t 1 line <&3 2>/dev/null; then
            echo "${line}"
        fi
    done

    # Dumbpipe exited - get exit code
    wait "${DUMBPIPE_PID}" || true
    local exit_code=$?

    log_info "dumbpipe exited with code: ${exit_code}"

    # Cleanup
    exec 3<&-
    cleanup_pipe

    exit "${exit_code}"
}

main "$@"
