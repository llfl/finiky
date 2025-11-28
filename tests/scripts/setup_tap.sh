#!/bin/bash
# Setup TAP network device

set -euo pipefail

TAP_DEVICE="${1:-tap0}"
TAP_IP="${2:-192.168.100.1}"
TAP_NETMASK="${3:-255.255.255.0}"

# Check if already exists
if ip link show "$TAP_DEVICE" >/dev/null 2>&1; then
    echo "TAP device $TAP_DEVICE already exists, deleting first..."
    ip link delete "$TAP_DEVICE" || true
fi

# Create TAP device
echo "Creating TAP device: $TAP_DEVICE"
ip tuntap add mode tap name "$TAP_DEVICE" || {
    echo "Error: Failed to create TAP device, may need to install tunctl or use root privileges"
    exit 1
}

# Configure IP address
echo "Configuring IP address: $TAP_IP/$TAP_NETMASK"
ip addr add "$TAP_IP/24" dev "$TAP_DEVICE" || {
    echo "Error: Failed to configure IP address"
    ip link delete "$TAP_DEVICE" || true
    exit 1
}

# Enable device
ip link set "$TAP_DEVICE" up || {
    echo "Error: Failed to enable TAP device"
    ip link delete "$TAP_DEVICE" || true
    exit 1
}

# Show configuration
echo "TAP device configuration completed:"
ip addr show "$TAP_DEVICE"

