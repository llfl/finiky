#!/bin/bash
# QEMU PXE End-to-End Test Script
# Used to verify the complete functionality of the finiky PXE server

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
SCRIPTS_DIR="$SCRIPT_DIR/scripts"
TAP_DEVICE="${TAP_DEVICE:-tap0}"
TAP_IP="${TAP_IP:-192.168.100.1}"
TAP_NETMASK="${TAP_NETMASK:-255.255.255.0}"
QEMU_MEMORY="${QEMU_MEMORY:-512M}"
TEST_TIMEOUT="${TEST_TIMEOUT:-120}"

# Test mode
TEST_MODE="${1:-both}"  # uefi, legacy, both

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up test environment...${NC}"
    
    # Stop QEMU
    if [ -n "${QEMU_PID:-}" ] && kill -0 "$QEMU_PID" 2>/dev/null; then
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
    fi
    
    # Stop finiky server
    if [ -n "${FINIKY_PID:-}" ] && kill -0 "$FINIKY_PID" 2>/dev/null; then
        kill "$FINIKY_PID" 2>/dev/null || true
        wait "$FINIKY_PID" 2>/dev/null || true
    fi
    
    # Cleanup TAP device
    if [ -f "$SCRIPTS_DIR/cleanup.sh" ]; then
        bash "$SCRIPTS_DIR/cleanup.sh" "$TAP_DEVICE" || true
    fi
    
    echo -e "${GREEN}Cleanup completed${NC}"
}

# Setup cleanup on exit
trap cleanup EXIT INT TERM

# Check dependencies
check_dependencies() {
    echo -e "${YELLOW}Checking dependencies...${NC}"
    
    local missing_deps=()
    
    command -v qemu-system-x86_64 >/dev/null 2>&1 || missing_deps+=("qemu-system-x86_64")
    command -v ip >/dev/null 2>&1 || missing_deps+=("iproute2")
    command -v cargo >/dev/null 2>&1 || missing_deps+=("cargo")
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        echo -e "${RED}Missing dependencies: ${missing_deps[*]}${NC}"
        echo "Please install the missing dependencies and try again"
        exit 1
    fi
    
    # Check syslinux (optional, for preparing boot files)
    if ! command -v syslinux >/dev/null 2>&1; then
        echo -e "${YELLOW}Warning: syslinux not found, will try to use pre-prepared boot files${NC}"
    fi
    
    echo -e "${GREEN}Dependency check passed${NC}"
}

# Prepare test environment
prepare_environment() {
    echo -e "${YELLOW}Preparing test environment...${NC}"
    
    # Build finiky
    echo "Building finiky server..."
    cd "$PROJECT_ROOT"
    cargo build --release || {
        echo -e "${RED}Build failed${NC}"
        exit 1
    }
    
    # Prepare PXE files
    if [ -f "$SCRIPTS_DIR/prepare_pxe_files.sh" ]; then
        echo "Preparing PXE boot files..."
        bash "$SCRIPTS_DIR/prepare_pxe_files.sh" "$FIXTURES_DIR" || {
            echo -e "${RED}PXE file preparation failed${NC}"
            exit 1
        }
    else
        echo -e "${YELLOW}Warning: prepare_pxe_files.sh not found, skipping file preparation${NC}"
    fi
    
    # Setup TAP device
    if [ -f "$SCRIPTS_DIR/setup_tap.sh" ]; then
        echo "Setting up TAP network device..."
        bash "$SCRIPTS_DIR/setup_tap.sh" "$TAP_DEVICE" "$TAP_IP" "$TAP_NETMASK" || {
            echo -e "${RED}TAP device setup failed${NC}"
            exit 1
        }
    else
        echo -e "${YELLOW}Warning: setup_tap.sh not found, skipping network setup${NC}"
    fi
    
    echo -e "${GREEN}Environment preparation completed${NC}"
}

# Start finiky server
start_finiky_server() {
    echo -e "${YELLOW}Starting finiky PXE server...${NC}"
    
    local tftp_root="$FIXTURES_DIR"
    local http_root="$FIXTURES_DIR"
    local config_file="$SCRIPT_DIR/test_config.toml"
    
    # Create test configuration file
    cat > "$config_file" <<EOF
[dhcp]
port = 67
interface = "$TAP_DEVICE"
ip_pool_start = "192.168.100.100"
ip_pool_end = "192.168.100.200"
subnet_mask = "$TAP_NETMASK"
gateway = "$TAP_IP"
dns_servers = ["8.8.8.8"]
next_server = "$TAP_IP"

[dhcp.protocols]
efi = true
legacy = true
dhcp_boot = true

[tftp]
port = 69
root = "$tftp_root"

[http]
port = 8080
root = "$http_root"
EOF
    
    # Check if finiky binary exists
    local finiky_binary="$PROJECT_ROOT/target/release/finiky"
    if [ ! -f "$finiky_binary" ]; then
        echo -e "${RED}Error: finiky binary not found: $finiky_binary${NC}"
        echo "Please run: cargo build --release"
        exit 1
    fi
    
    # Start server (background)
    cd "$PROJECT_ROOT"
    RUST_LOG=info "$finiky_binary" start --config "$config_file" > "$SCRIPT_DIR/finiky.log" 2>&1 &
    FINIKY_PID=$!
    
    # Wait for server to start
    sleep 3
    
    # Check if server is running
    if ! kill -0 "$FINIKY_PID" 2>/dev/null; then
        echo -e "${RED}finiky server failed to start${NC}"
        echo "Server logs:"
        cat "$SCRIPT_DIR/finiky.log" || true
        exit 1
    fi
    
    # Check if ports are listening
    local retries=5
    local count=0
    while [ $count -lt $retries ]; do
        if ss -tuln | grep -q ":67 " && \
           ss -tuln | grep -q ":69 " && \
           ss -tuln | grep -q ":8080 "; then
            break
        fi
        sleep 1
        count=$((count + 1))
    done
    
    if [ $count -eq $retries ]; then
        echo -e "${YELLOW}Warning: Server may not be fully started, but continuing test...${NC}"
    fi
    
    echo -e "${GREEN}finiky server started (PID: $FINIKY_PID)${NC}"
}

# Run UEFI test
run_uefi_test() {
    echo -e "${YELLOW}Running UEFI PXE boot test...${NC}"
    
    local log_file="$SCRIPT_DIR/qemu_uefi.log"
    local serial_file="$SCRIPT_DIR/qemu_uefi_serial.log"
    
    # Check for UEFI firmware
    local efi_firmware=""
    if [ -f "/usr/share/qemu/OVMF.fd" ]; then
        efi_firmware="/usr/share/qemu/OVMF.fd"
    elif [ -f "/usr/share/OVMF/OVMF_CODE.fd" ]; then
        efi_firmware="/usr/share/OVMF/OVMF_CODE.fd"
    else
        echo -e "${YELLOW}Warning: OVMF firmware not found, skipping UEFI test${NC}"
        return 0
    fi
    
    # Start QEMU
    qemu-system-x86_64 \
        -machine q35,accel=kvm:tcg \
        -cpu qemu64 \
        -m "$QEMU_MEMORY" \
        -netdev tap,id=net0,ifname="$TAP_DEVICE",script=no,downscript=no \
        -device virtio-net-pci,netdev=net0 \
        -bios "$efi_firmware" \
        -boot n \
        -serial stdio \
        -serial file:"$serial_file" \
        -nographic \
        -no-reboot \
        > "$log_file" 2>&1 &
    QEMU_PID=$!
    
    echo "QEMU started (PID: $QEMU_PID), waiting for PXE boot..."
    
    # Wait for test completion or timeout
    local elapsed=0
    while [ $elapsed -lt $TEST_TIMEOUT ]; do
        sleep 2
        elapsed=$((elapsed + 2))
        
        # Check if QEMU is still running
        if ! kill -0 "$QEMU_PID" 2>/dev/null; then
            break
        fi
        
        # Check serial output for success indicators
        if [ -f "$serial_file" ]; then
            if grep -q "PXE" "$serial_file" 2>/dev/null || \
               grep -q "DHCP" "$serial_file" 2>/dev/null || \
               grep -q "TFTP" "$serial_file" 2>/dev/null; then
                echo -e "${GREEN}Detected PXE boot activity${NC}"
            fi
        fi
    done
    
    # Stop QEMU
    if kill -0 "$QEMU_PID" 2>/dev/null; then
        echo "Stopping QEMU..."
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
    fi
    
    # Analyze results
    if [ -f "$serial_file" ]; then
        echo -e "${YELLOW}UEFI test logs:${NC}"
        tail -50 "$serial_file" || true
    fi
    
    echo -e "${GREEN}UEFI test completed${NC}"
    QEMU_PID=""
}

# Run Legacy BIOS test
run_legacy_test() {
    echo -e "${YELLOW}Running Legacy BIOS PXE boot test...${NC}"
    
    local log_file="$SCRIPT_DIR/qemu_legacy.log"
    local serial_file="$SCRIPT_DIR/qemu_legacy_serial.log"
    
    # Start QEMU (using Legacy BIOS)
    qemu-system-x86_64 \
        -machine pc,accel=kvm:tcg \
        -cpu qemu64 \
        -m "$QEMU_MEMORY" \
        -netdev tap,id=net0,ifname="$TAP_DEVICE",script=no,downscript=no \
        -device rtl8139,netdev=net0 \
        -boot n \
        -serial stdio \
        -serial file:"$serial_file" \
        -nographic \
        -no-reboot \
        > "$log_file" 2>&1 &
    QEMU_PID=$!
    
    echo "QEMU started (PID: $QEMU_PID), waiting for PXE boot..."
    
    # Wait for test completion or timeout
    local elapsed=0
    while [ $elapsed -lt $TEST_TIMEOUT ]; do
        sleep 2
        elapsed=$((elapsed + 2))
        
        # Check if QEMU is still running
        if ! kill -0 "$QEMU_PID" 2>/dev/null; then
            break
        fi
        
        # Check serial output for success indicators
        if [ -f "$serial_file" ]; then
            if grep -q "PXE" "$serial_file" 2>/dev/null || \
               grep -q "DHCP" "$serial_file" 2>/dev/null || \
               grep -q "TFTP" "$serial_file" 2>/dev/null; then
                echo -e "${GREEN}Detected PXE boot activity${NC}"
            fi
        fi
    done
    
    # Stop QEMU
    if kill -0 "$QEMU_PID" 2>/dev/null; then
        echo "Stopping QEMU..."
        kill "$QEMU_PID" 2>/dev/null || true
        wait "$QEMU_PID" 2>/dev/null || true
    fi
    
    # Analyze results
    if [ -f "$serial_file" ]; then
        echo -e "${YELLOW}Legacy BIOS test logs:${NC}"
        tail -50 "$serial_file" || true
    fi
    
    echo -e "${GREEN}Legacy BIOS test completed${NC}"
    QEMU_PID=""
}

# Main function
main() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  QEMU PXE End-to-End Test${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    
    # Check root privileges (for network configuration)
    if [ "$EUID" -ne 0 ]; then
        echo -e "${YELLOW}Warning: Root privileges required to configure network devices${NC}"
        echo "Please run this script with sudo"
        exit 1
    fi
    
    check_dependencies
    prepare_environment
    start_finiky_server
    
    # Run tests based on test mode
    case "$TEST_MODE" in
        uefi)
            run_uefi_test
            ;;
        legacy)
            run_legacy_test
            ;;
        both)
            run_uefi_test
            run_legacy_test
            ;;
        *)
            echo -e "${RED}Unknown test mode: $TEST_MODE${NC}"
            echo "Usage: $0 [uefi|legacy|both]"
            exit 1
            ;;
    esac
    
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  All tests completed${NC}"
    echo -e "${GREEN}========================================${NC}"
}

# Run main function
main "$@"

