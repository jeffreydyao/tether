#!/usr/bin/env bash
#
# tether-network-watchdog.sh
#
# Network watchdog for Tether - manages WiFi connectivity and AP fallback.
# Monitors internet connectivity and falls back to AP mode when all networks fail.
#
# Exit codes:
#   0 - Clean shutdown
#   1 - General error
#   2 - NetworkManager not running
#   3 - WiFi hardware not available
#   4 - Configuration file error
#
# Signals:
#   SIGHUP  - Reload configuration and force connectivity check
#   SIGTERM - Graceful shutdown
#   SIGINT  - Graceful shutdown
#

set -euo pipefail

################################################################################
# CONSTANTS
################################################################################

readonly SCRIPT_NAME="tether-network-watchdog"
readonly SCRIPT_VERSION="1.0.0"

# Paths
readonly CONFIG_DIR="/opt/tether/config"
readonly CONFIG_FILE="${CONFIG_DIR}/tether.toml"
readonly STATE_DIR="/opt/tether/data"
readonly STATE_FILE="${STATE_DIR}/watchdog.state"
readonly LOG_DIR="/opt/tether/logs"
readonly LOG_FILE="${LOG_DIR}/network-watchdog.log"
readonly PID_FILE="/run/tether-network-watchdog.pid"

# NetworkManager connection names
readonly AP_CONNECTION_NAME="TetherSetup"
readonly AP_SSID="TetherSetup"
readonly AP_INTERFACE="wlan0"

# AP Mode configuration
readonly AP_IP_ADDRESS="192.168.4.1"
readonly AP_IP_NETMASK="24"
readonly AP_CHANNEL="6"
readonly AP_BAND="bg"  # 2.4GHz

# Connectivity check configuration
readonly CONNECTIVITY_CHECK_URL="${CONNECTIVITY_CHECK_URL:-http://connectivitycheck.gstatic.com/generate_204}"
readonly CONNECTIVITY_CHECK_TIMEOUT=10
readonly CONNECTIVITY_EXPECTED_CODE=204

# Timing (in seconds)
readonly CHECK_INTERVAL="${CHECK_INTERVAL:-30}"
readonly NETWORK_SWITCH_DELAY=5
readonly MAX_CONNECTION_ATTEMPTS=3
readonly CONNECTION_TIMEOUT=30

# nmcli exit codes
readonly NMCLI_SUCCESS=0
readonly NMCLI_UNKNOWN_ERROR=1
readonly NMCLI_INVALID_INPUT=2
readonly NMCLI_TIMEOUT=3
readonly NMCLI_ACTIVATION_FAILED=4
readonly NMCLI_DEACTIVATION_FAILED=5
readonly NMCLI_DISCONNECT_FAILED=6
readonly NMCLI_DELETE_FAILED=7
readonly NMCLI_NM_NOT_RUNNING=8
readonly NMCLI_NOT_FOUND=10

################################################################################
# GLOBAL STATE
################################################################################

# Current state
declare -g CURRENT_MODE="unknown"         # "wifi", "ap", "disconnected"
declare -g CURRENT_SSID=""
declare -g LAST_CONNECTIVITY_CHECK=0
declare -g SHUTDOWN_REQUESTED=false
declare -g RELOAD_REQUESTED=false

# Configuration (loaded from TOML)
declare -ga CONFIGURED_SSIDS=()
declare -ga CONFIGURED_PASSWORDS=()
declare -g IS_ONBOARDED="false"
declare -g PRIMARY_SSID=""

################################################################################
# LOGGING
################################################################################

# Log levels
readonly LOG_LEVEL_DEBUG=0
readonly LOG_LEVEL_INFO=1
readonly LOG_LEVEL_WARN=2
readonly LOG_LEVEL_ERROR=3

# Current log level (can be overridden by environment)
LOG_LEVEL="${LOG_LEVEL:-$LOG_LEVEL_INFO}"

_log() {
    local level="$1"
    local level_name="$2"
    shift 2
    local message="$*"
    local timestamp
    timestamp=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

    if [[ "$level" -ge "$LOG_LEVEL" ]]; then
        # Log to file
        echo "${timestamp} [${level_name}] ${message}" >> "$LOG_FILE"

        # Also log to stderr if running interactively
        if [[ -t 2 ]]; then
            echo "${timestamp} [${level_name}] ${message}" >&2
        fi

        # Log to systemd journal if available
        if command -v logger &>/dev/null; then
            logger -t "$SCRIPT_NAME" -p "daemon.${level_name,,}" "$message"
        fi
    fi
}

log_debug() { _log "$LOG_LEVEL_DEBUG" "DEBUG" "$@"; }
log_info()  { _log "$LOG_LEVEL_INFO"  "INFO"  "$@"; }
log_warn()  { _log "$LOG_LEVEL_WARN"  "WARN"  "$@"; }
log_error() { _log "$LOG_LEVEL_ERROR" "ERROR" "$@"; }

################################################################################
# INITIALIZATION AND CLEANUP
################################################################################

init_directories() {
    # Create required directories with proper permissions
    local dirs=("$CONFIG_DIR" "$STATE_DIR" "$LOG_DIR")

    for dir in "${dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            mkdir -p "$dir"
            chmod 755 "$dir"
            log_info "Created directory: $dir"
        fi
    done

    # Ensure log file exists and is writable
    if [[ ! -f "$LOG_FILE" ]]; then
        touch "$LOG_FILE"
        chmod 644 "$LOG_FILE"
    fi
}

write_pid_file() {
    echo $$ > "$PID_FILE"
    log_debug "Wrote PID $$ to $PID_FILE"
}

remove_pid_file() {
    if [[ -f "$PID_FILE" ]]; then
        rm -f "$PID_FILE"
        log_debug "Removed PID file"
    fi
}

cleanup() {
    log_info "Cleaning up..."
    remove_pid_file
    log_info "Watchdog shutdown complete"
}

################################################################################
# SIGNAL HANDLERS
################################################################################

handle_shutdown() {
    log_info "Received shutdown signal"
    SHUTDOWN_REQUESTED=true
}

handle_reload() {
    log_info "Received reload signal (SIGHUP)"
    RELOAD_REQUESTED=true
}

setup_signal_handlers() {
    trap handle_shutdown SIGTERM SIGINT
    trap handle_reload SIGHUP
    trap cleanup EXIT
}

################################################################################
# PREREQUISITES CHECK
################################################################################

check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

check_networkmanager_running() {
    # Check if NetworkManager service is active
    if ! systemctl is-active --quiet NetworkManager; then
        log_error "NetworkManager is not running"

        # Attempt to start it
        log_info "Attempting to start NetworkManager..."
        if systemctl start NetworkManager; then
            log_info "NetworkManager started successfully"
            sleep 2  # Give it time to initialize
        else
            log_error "Failed to start NetworkManager"
            return 1
        fi
    fi

    # Verify nmcli can communicate with NetworkManager
    if ! LC_ALL=C nmcli general status &>/dev/null; then
        log_error "Cannot communicate with NetworkManager via nmcli"
        return 1
    fi

    log_debug "NetworkManager is running and responsive"
    return 0
}

check_wifi_hardware() {
    # Check if WiFi device exists
    if ! nmcli device status | grep -q "^${AP_INTERFACE}"; then
        log_error "WiFi interface ${AP_INTERFACE} not found"
        return 1
    fi

    # Check device type
    local device_type
    device_type=$(LC_ALL=C nmcli -t -f DEVICE,TYPE device status | grep "^${AP_INTERFACE}:" | cut -d: -f2)

    if [[ "$device_type" != "wifi" ]]; then
        log_error "Device ${AP_INTERFACE} is not a WiFi device (type: $device_type)"
        return 1
    fi

    # Check if WiFi radio is enabled
    local wifi_enabled
    wifi_enabled=$(LC_ALL=C nmcli radio wifi)

    if [[ "$wifi_enabled" != "enabled" ]]; then
        log_warn "WiFi radio is disabled, attempting to enable..."
        if LC_ALL=C nmcli radio wifi on; then
            log_info "WiFi radio enabled"
            sleep 2
        else
            log_error "Failed to enable WiFi radio"
            return 1
        fi
    fi

    log_debug "WiFi hardware check passed"
    return 0
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    check_root

    if ! check_networkmanager_running; then
        exit 2
    fi

    if ! check_wifi_hardware; then
        exit 3
    fi

    log_info "Prerequisites check passed"
}

################################################################################
# CONFIGURATION PARSING
################################################################################

# Simple TOML parser for our specific config format
# Handles:
#   onboarded = true/false
#   primary_ssid = "NetworkName"
#   [[wifi_networks]]
#   ssid = "NetworkName"
#   password = "password123"  # Optional

parse_config() {
    log_info "Parsing configuration from $CONFIG_FILE"

    # Reset configuration
    CONFIGURED_SSIDS=()
    CONFIGURED_PASSWORDS=()
    IS_ONBOARDED="false"
    PRIMARY_SSID=""

    if [[ ! -f "$CONFIG_FILE" ]]; then
        log_warn "Configuration file not found: $CONFIG_FILE"
        log_info "Assuming first boot - device not onboarded"
        return 0
    fi

    if [[ ! -r "$CONFIG_FILE" ]]; then
        log_error "Cannot read configuration file: $CONFIG_FILE"
        return 1
    fi

    local in_wifi_section=false
    local current_ssid=""
    local current_password=""
    local line_number=0

    while IFS= read -r line || [[ -n "$line" ]]; do
        ((line_number++))

        # Remove leading/trailing whitespace
        line=$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

        # Skip empty lines and comments
        [[ -z "$line" ]] && continue
        [[ "$line" =~ ^# ]] && continue

        # Check for [[wifi_networks]] or [[wifi.networks]] section
        if [[ "$line" == "[[wifi_networks]]" ]] || [[ "$line" == "[[wifi.networks]]" ]]; then
            # Save previous network if exists
            if [[ -n "$current_ssid" ]]; then
                CONFIGURED_SSIDS+=("$current_ssid")
                CONFIGURED_PASSWORDS+=("$current_password")
            fi
            in_wifi_section=true
            current_ssid=""
            current_password=""
            continue
        fi

        # Check for other section headers (exit wifi section)
        if [[ "$line" =~ ^\[.*\]$ && "$line" != "[[wifi_networks]]" && "$line" != "[[wifi.networks]]" ]]; then
            # Save previous network if exists
            if [[ -n "$current_ssid" ]]; then
                CONFIGURED_SSIDS+=("$current_ssid")
                CONFIGURED_PASSWORDS+=("$current_password")
                current_ssid=""
                current_password=""
            fi
            in_wifi_section=false
            continue
        fi

        # Parse key = value
        if [[ "$line" =~ ^([a-zA-Z_][a-zA-Z0-9_]*)\ *=\ *(.+)$ ]]; then
            local key="${BASH_REMATCH[1]}"
            local value="${BASH_REMATCH[2]}"

            # Remove quotes from value
            value=$(echo "$value" | sed 's/^"//;s/"$//' | sed "s/^'//;s/'$//")

            if $in_wifi_section; then
                case "$key" in
                    ssid)
                        current_ssid="$value"
                        ;;
                    password|psk)
                        current_password="$value"
                        ;;
                esac
            else
                # Top-level configuration
                case "$key" in
                    onboarded)
                        IS_ONBOARDED="$value"
                        ;;
                    primary_ssid|primary_network)
                        PRIMARY_SSID="$value"
                        ;;
                esac
            fi
        fi
    done < "$CONFIG_FILE"

    # Save last network if exists
    if [[ -n "$current_ssid" ]]; then
        CONFIGURED_SSIDS+=("$current_ssid")
        CONFIGURED_PASSWORDS+=("$current_password")
    fi

    # Log parsed configuration
    log_info "Configuration loaded:"
    log_info "  onboarded: $IS_ONBOARDED"
    log_info "  primary_ssid: ${PRIMARY_SSID:-<none>}"
    log_info "  configured networks: ${#CONFIGURED_SSIDS[@]}"

    for i in "${!CONFIGURED_SSIDS[@]}"; do
        local has_password="no"
        [[ -n "${CONFIGURED_PASSWORDS[$i]}" ]] && has_password="yes"
        log_debug "    [$i] ${CONFIGURED_SSIDS[$i]} (password: $has_password)"
    done

    return 0
}

################################################################################
# STATE MANAGEMENT
################################################################################

save_state() {
    local state_content
    state_content=$(cat <<EOF
# Tether Network Watchdog State
# Auto-generated - do not edit
timestamp=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
mode=${CURRENT_MODE}
ssid=${CURRENT_SSID}
last_check=${LAST_CONNECTIVITY_CHECK}
EOF
)
    echo "$state_content" > "$STATE_FILE"
    chmod 644 "$STATE_FILE"
}

load_state() {
    if [[ -f "$STATE_FILE" ]]; then
        log_debug "Loading previous state from $STATE_FILE"
        # Simple key=value parsing
        while IFS='=' read -r key value; do
            [[ "$key" =~ ^# ]] && continue
            [[ -z "$key" ]] && continue
            case "$key" in
                mode) CURRENT_MODE="$value" ;;
                ssid) CURRENT_SSID="$value" ;;
            esac
        done < "$STATE_FILE"
        log_debug "Loaded state: mode=$CURRENT_MODE, ssid=$CURRENT_SSID"
    fi
}

################################################################################
# CONNECTIVITY CHECKING
################################################################################

check_internet_connectivity() {
    log_debug "Checking internet connectivity..."

    local http_code
    local curl_exit_code

    # Use curl to check connectivity
    # -s: silent, -o /dev/null: discard output, -w: write HTTP code
    # -m: max time, --connect-timeout: connection timeout
    http_code=$(curl -s -o /dev/null -w "%{http_code}" \
        -m "$CONNECTIVITY_CHECK_TIMEOUT" \
        --connect-timeout 5 \
        "$CONNECTIVITY_CHECK_URL" 2>/dev/null) || curl_exit_code=$?

    LAST_CONNECTIVITY_CHECK=$(date +%s)

    if [[ -n "${curl_exit_code:-}" ]]; then
        log_debug "curl failed with exit code: $curl_exit_code"
        case "$curl_exit_code" in
            6)  log_debug "Could not resolve host" ;;
            7)  log_debug "Failed to connect to host" ;;
            28) log_debug "Connection timed out" ;;
            *)  log_debug "curl error: $curl_exit_code" ;;
        esac
        return 1
    fi

    if [[ "$http_code" -eq "$CONNECTIVITY_EXPECTED_CODE" ]]; then
        log_debug "Internet connectivity confirmed (HTTP $http_code)"
        return 0
    else
        log_debug "Unexpected HTTP response: $http_code (expected $CONNECTIVITY_EXPECTED_CODE)"
        return 1
    fi
}

# Alternative connectivity check using NetworkManager's built-in check
check_nm_connectivity() {
    local connectivity
    connectivity=$(LC_ALL=C nmcli networking connectivity check 2>/dev/null)

    case "$connectivity" in
        full)
            log_debug "NetworkManager reports full connectivity"
            return 0
            ;;
        limited)
            log_debug "NetworkManager reports limited connectivity"
            return 1
            ;;
        portal)
            log_debug "NetworkManager reports captive portal"
            return 1
            ;;
        none)
            log_debug "NetworkManager reports no connectivity"
            return 1
            ;;
        *)
            log_debug "NetworkManager connectivity unknown: $connectivity"
            return 1
            ;;
    esac
}

################################################################################
# NETWORK SCANNING
################################################################################

scan_available_networks() {
    log_debug "Scanning for available WiFi networks..."

    # Force a rescan
    LC_ALL=C nmcli device wifi rescan ifname "$AP_INTERFACE" 2>/dev/null || true
    sleep 2

    # Get list of available networks
    LC_ALL=C nmcli -t -f SSID,SIGNAL,SECURITY device wifi list ifname "$AP_INTERFACE" 2>/dev/null
}

is_network_available() {
    local target_ssid="$1"

    # Scan and check if network is visible
    local available
    available=$(scan_available_networks)

    if echo "$available" | grep -q "^${target_ssid}:"; then
        log_debug "Network '$target_ssid' is available"
        return 0
    else
        log_debug "Network '$target_ssid' is not available"
        return 1
    fi
}

################################################################################
# CONNECTION MANAGEMENT
################################################################################

get_active_connection() {
    # Get the active WiFi connection name
    LC_ALL=C nmcli -t -f NAME,TYPE,DEVICE connection show --active 2>/dev/null | \
        grep ":802-11-wireless:${AP_INTERFACE}$" | \
        cut -d: -f1 | \
        head -n1
}

get_active_ssid() {
    # Get the SSID of the currently connected network
    LC_ALL=C nmcli -t -f ACTIVE,SSID device wifi list ifname "$AP_INTERFACE" 2>/dev/null | \
        grep "^yes:" | \
        cut -d: -f2 | \
        head -n1
}

connection_exists() {
    local conn_name="$1"
    LC_ALL=C nmcli -t -f NAME connection show 2>/dev/null | grep -q "^${conn_name}$"
}

get_connection_for_ssid() {
    local ssid="$1"
    # Find connection profile for this SSID
    LC_ALL=C nmcli -t -f NAME,802-11-wireless.ssid connection show 2>/dev/null | \
        grep ":${ssid}$" | \
        cut -d: -f1 | \
        head -n1
}

################################################################################
# WIFI STATION MODE
################################################################################

connect_to_wifi() {
    local ssid="$1"
    local password="${2:-}"
    local attempt=1

    log_info "Attempting to connect to WiFi: $ssid"

    # First, ensure AP mode is disabled
    disable_ap_mode_if_active

    while [[ $attempt -le $MAX_CONNECTION_ATTEMPTS ]]; do
        log_debug "Connection attempt $attempt of $MAX_CONNECTION_ATTEMPTS"

        # Check if connection profile exists
        local conn_name
        conn_name=$(get_connection_for_ssid "$ssid")

        local nmcli_result
        local exit_code

        if [[ -n "$conn_name" ]]; then
            # Use existing connection profile
            log_debug "Using existing connection profile: $conn_name"
            nmcli_result=$(LC_ALL=C nmcli --wait "$CONNECTION_TIMEOUT" connection up "$conn_name" 2>&1) && exit_code=0 || exit_code=$?
        else
            # Create new connection on the fly
            log_debug "Creating new connection for SSID: $ssid"
            if [[ -n "$password" ]]; then
                nmcli_result=$(LC_ALL=C nmcli --wait "$CONNECTION_TIMEOUT" device wifi connect "$ssid" password "$password" ifname "$AP_INTERFACE" 2>&1) && exit_code=0 || exit_code=$?
            else
                nmcli_result=$(LC_ALL=C nmcli --wait "$CONNECTION_TIMEOUT" device wifi connect "$ssid" ifname "$AP_INTERFACE" 2>&1) && exit_code=0 || exit_code=$?
            fi
        fi

        case $exit_code in
            $NMCLI_SUCCESS)
                log_info "Successfully connected to: $ssid"
                CURRENT_MODE="wifi"
                CURRENT_SSID="$ssid"
                save_state

                # Wait a moment and verify connectivity
                sleep 3
                if check_internet_connectivity; then
                    log_info "Internet connectivity verified on $ssid"
                    return 0
                else
                    log_warn "Connected to $ssid but no internet access"
                    # Don't return failure yet - might still be usable
                    return 0
                fi
                ;;
            $NMCLI_TIMEOUT)
                log_warn "Connection attempt timed out"
                ;;
            $NMCLI_ACTIVATION_FAILED)
                log_warn "Connection activation failed: $nmcli_result"
                ;;
            $NMCLI_NM_NOT_RUNNING)
                log_error "NetworkManager is not running"
                return 1
                ;;
            $NMCLI_NOT_FOUND)
                log_warn "Network or connection not found"
                return 1  # No point retrying
                ;;
            *)
                log_warn "Connection failed with code $exit_code: $nmcli_result"
                ;;
        esac

        ((attempt++))
        [[ $attempt -le $MAX_CONNECTION_ATTEMPTS ]] && sleep 2
    done

    log_error "Failed to connect to $ssid after $MAX_CONNECTION_ATTEMPTS attempts"
    return 1
}

disconnect_wifi() {
    log_debug "Disconnecting WiFi..."

    local active_conn
    active_conn=$(get_active_connection)

    if [[ -n "$active_conn" && "$active_conn" != "$AP_CONNECTION_NAME" ]]; then
        log_debug "Deactivating connection: $active_conn"
        LC_ALL=C nmcli connection down "$active_conn" 2>/dev/null || true
    fi

    CURRENT_MODE="disconnected"
    CURRENT_SSID=""
    save_state
}

################################################################################
# ACCESS POINT MODE
################################################################################

create_ap_connection_profile() {
    log_info "Creating AP connection profile: $AP_CONNECTION_NAME"

    # Delete existing if present (to ensure clean state)
    if connection_exists "$AP_CONNECTION_NAME"; then
        log_debug "Removing existing AP connection profile"
        LC_ALL=C nmcli connection delete "$AP_CONNECTION_NAME" 2>/dev/null || true
        sleep 1
    fi

    # Create AP connection profile
    # Step 1: Create basic connection
    if ! LC_ALL=C nmcli connection add \
        type wifi \
        ifname "$AP_INTERFACE" \
        con-name "$AP_CONNECTION_NAME" \
        autoconnect no \
        ssid "$AP_SSID" \
        mode ap \
        2>&1; then
        log_error "Failed to create AP connection profile"
        return 1
    fi

    # Step 2: Configure WiFi settings
    if ! LC_ALL=C nmcli connection modify "$AP_CONNECTION_NAME" \
        802-11-wireless.band "$AP_BAND" \
        802-11-wireless.channel "$AP_CHANNEL" \
        2>&1; then
        log_error "Failed to configure AP WiFi settings"
        return 1
    fi

    # Step 3: Configure IP settings (shared mode enables DHCP via dnsmasq)
    if ! LC_ALL=C nmcli connection modify "$AP_CONNECTION_NAME" \
        ipv4.method shared \
        ipv4.addresses "${AP_IP_ADDRESS}/${AP_IP_NETMASK}" \
        2>&1; then
        log_error "Failed to configure AP IP settings"
        return 1
    fi

    # Step 4: Disable IPv6 for simplicity
    if ! LC_ALL=C nmcli connection modify "$AP_CONNECTION_NAME" \
        ipv6.method disabled \
        2>&1; then
        log_warn "Failed to disable IPv6 on AP (non-fatal)"
    fi

    # NOTE: We intentionally do NOT set wifi-sec for open network

    log_info "AP connection profile created successfully"
    log_info "  SSID: $AP_SSID"
    log_info "  IP: ${AP_IP_ADDRESS}/${AP_IP_NETMASK}"
    log_info "  Security: Open (no password)"

    return 0
}

enable_ap_mode() {
    log_info "Enabling AP mode..."

    # Ensure AP connection profile exists
    if ! connection_exists "$AP_CONNECTION_NAME"; then
        if ! create_ap_connection_profile; then
            log_error "Failed to create AP connection profile"
            return 1
        fi
    fi

    # Disconnect any active WiFi connection first
    local active_conn
    active_conn=$(get_active_connection)
    if [[ -n "$active_conn" && "$active_conn" != "$AP_CONNECTION_NAME" ]]; then
        log_debug "Disconnecting from: $active_conn"
        LC_ALL=C nmcli connection down "$active_conn" 2>/dev/null || true
        sleep 2
    fi

    # Activate AP
    local nmcli_result
    local exit_code
    nmcli_result=$(LC_ALL=C nmcli --wait 30 connection up "$AP_CONNECTION_NAME" 2>&1) && exit_code=0 || exit_code=$?

    case $exit_code in
        $NMCLI_SUCCESS)
            log_info "AP mode enabled successfully"
            log_info "  SSID: $AP_SSID"
            log_info "  IP: $AP_IP_ADDRESS"
            log_info "  Connect to configure the device"
            CURRENT_MODE="ap"
            CURRENT_SSID="$AP_SSID"
            save_state
            return 0
            ;;
        $NMCLI_ACTIVATION_FAILED)
            log_error "Failed to activate AP mode: $nmcli_result"

            # Check for common issues
            if echo "$nmcli_result" | grep -qi "device.*busy"; then
                log_error "WiFi device is busy - may need to wait or restart NetworkManager"
            fi
            if echo "$nmcli_result" | grep -qi "not supported"; then
                log_error "AP mode may not be supported by this hardware"
            fi
            return 1
            ;;
        *)
            log_error "AP activation failed with code $exit_code: $nmcli_result"
            return 1
            ;;
    esac
}

disable_ap_mode() {
    log_info "Disabling AP mode..."

    if ! connection_exists "$AP_CONNECTION_NAME"; then
        log_debug "AP connection profile does not exist"
        return 0
    fi

    # Check if AP is currently active
    local active_conn
    active_conn=$(get_active_connection)

    if [[ "$active_conn" == "$AP_CONNECTION_NAME" ]]; then
        log_debug "Deactivating AP connection"
        local result
        result=$(LC_ALL=C nmcli connection down "$AP_CONNECTION_NAME" 2>&1) || true
        sleep 2
    fi

    if [[ "$CURRENT_MODE" == "ap" ]]; then
        CURRENT_MODE="disconnected"
        CURRENT_SSID=""
        save_state
    fi

    log_info "AP mode disabled"
    return 0
}

disable_ap_mode_if_active() {
    local active_conn
    active_conn=$(get_active_connection)

    if [[ "$active_conn" == "$AP_CONNECTION_NAME" ]]; then
        disable_ap_mode
    fi
}

is_ap_mode_active() {
    local active_conn
    active_conn=$(get_active_connection)
    [[ "$active_conn" == "$AP_CONNECTION_NAME" ]]
}

################################################################################
# NETWORK SELECTION AND FAILOVER
################################################################################

get_ordered_networks() {
    # Return networks in priority order:
    # 1. Primary SSID (if set)
    # 2. Other configured networks in order

    local -a ordered=()

    # Add primary first if set and exists in config
    if [[ -n "$PRIMARY_SSID" ]]; then
        for i in "${!CONFIGURED_SSIDS[@]}"; do
            if [[ "${CONFIGURED_SSIDS[$i]}" == "$PRIMARY_SSID" ]]; then
                ordered+=("$i")
                break
            fi
        done
    fi

    # Add remaining networks
    for i in "${!CONFIGURED_SSIDS[@]}"; do
        local already_added=false
        for j in "${ordered[@]:-}"; do
            if [[ "$j" == "$i" ]]; then
                already_added=true
                break
            fi
        done
        if ! $already_added; then
            ordered+=("$i")
        fi
    done

    echo "${ordered[*]:-}"
}

try_connect_to_configured_networks() {
    log_info "Attempting to connect to configured networks..."

    if [[ ${#CONFIGURED_SSIDS[@]} -eq 0 ]]; then
        log_warn "No WiFi networks configured"
        return 1
    fi

    local ordered
    ordered=$(get_ordered_networks)

    for idx in $ordered; do
        local ssid="${CONFIGURED_SSIDS[$idx]}"
        local password="${CONFIGURED_PASSWORDS[$idx]:-}"

        log_info "Trying network: $ssid"

        # Check if network is available (visible in scan)
        if ! is_network_available "$ssid"; then
            log_info "Network '$ssid' is not visible, skipping"
            continue
        fi

        # Attempt connection
        if connect_to_wifi "$ssid" "$password"; then
            # Verify internet connectivity
            sleep "$NETWORK_SWITCH_DELAY"
            if check_internet_connectivity; then
                log_info "Successfully connected to '$ssid' with internet access"
                return 0
            else
                log_warn "Connected to '$ssid' but no internet, trying next..."
                disconnect_wifi
            fi
        fi
    done

    log_error "Failed to connect to any configured network"
    return 1
}

################################################################################
# MAIN WATCHDOG LOGIC
################################################################################

handle_not_onboarded() {
    log_info "Device not onboarded - enabling AP mode for setup"

    if ! is_ap_mode_active; then
        if ! enable_ap_mode; then
            log_error "Failed to enable AP mode for onboarding"
            return 1
        fi
    fi

    log_info "Waiting for onboarding via web UI at http://${AP_IP_ADDRESS}"
    return 0
}

handle_onboarded() {
    log_debug "Checking network status..."

    # Get current connection state
    local active_ssid
    active_ssid=$(get_active_ssid)

    # If in AP mode, check if we should try to connect to WiFi
    if is_ap_mode_active; then
        log_info "Currently in AP mode, attempting to connect to configured WiFi..."

        if try_connect_to_configured_networks; then
            log_info "Successfully connected to WiFi, disabling AP mode"
            return 0
        else
            log_warn "Could not connect to any WiFi network, staying in AP mode"
            return 0
        fi
    fi

    # If connected to a WiFi network, verify connectivity
    if [[ -n "$active_ssid" ]]; then
        log_debug "Currently connected to: $active_ssid"

        if check_internet_connectivity; then
            log_debug "Internet connectivity OK on $active_ssid"
            CURRENT_MODE="wifi"
            CURRENT_SSID="$active_ssid"
            return 0
        else
            log_warn "No internet on current network: $active_ssid"
            log_info "Attempting to find a working network..."

            # Disconnect and try other networks
            disconnect_wifi

            if try_connect_to_configured_networks; then
                return 0
            fi
        fi
    else
        log_info "Not connected to any WiFi network"

        # Try to connect
        if try_connect_to_configured_networks; then
            return 0
        fi
    fi

    # All connection attempts failed - fall back to AP mode
    log_warn "All WiFi networks failed, falling back to AP mode"
    if enable_ap_mode; then
        log_info "AP mode enabled for reconfiguration"
    else
        log_error "Failed to enable AP mode"
    fi

    return 0
}

run_watchdog_cycle() {
    log_debug "Running watchdog cycle..."

    # Check if reload was requested
    if $RELOAD_REQUESTED; then
        log_info "Reloading configuration..."
        RELOAD_REQUESTED=false
        parse_config
    fi

    # Run appropriate handler based on onboarding status
    if [[ "$IS_ONBOARDED" != "true" ]]; then
        handle_not_onboarded
    else
        handle_onboarded
    fi
}

################################################################################
# MAIN ENTRY POINT
################################################################################

main() {
    log_info "=========================================="
    log_info "Tether Network Watchdog v${SCRIPT_VERSION}"
    log_info "=========================================="

    # Initialize
    init_directories
    setup_signal_handlers
    write_pid_file

    # Check prerequisites
    check_prerequisites

    # Load configuration
    if ! parse_config; then
        log_error "Failed to parse configuration"
        exit 4
    fi

    # Load previous state
    load_state

    # Ensure AP profile exists (will be needed eventually)
    if ! connection_exists "$AP_CONNECTION_NAME"; then
        log_info "Creating initial AP connection profile..."
        create_ap_connection_profile || log_warn "Failed to create AP profile (will retry)"
    fi

    # Initial run
    run_watchdog_cycle

    # Main loop
    log_info "Entering main watchdog loop (interval: ${CHECK_INTERVAL}s)"

    while ! $SHUTDOWN_REQUESTED; do
        sleep "$CHECK_INTERVAL" &
        wait $! || true  # Allow signals to interrupt sleep

        if $SHUTDOWN_REQUESTED; then
            break
        fi

        run_watchdog_cycle
    done

    log_info "Watchdog shutting down..."
}

# Run main if executed directly (not sourced)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
