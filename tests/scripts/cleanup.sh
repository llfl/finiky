#!/bin/bash
# Cleanup TAP network device

set -euo pipefail

TAP_DEVICE="${1:-tap0}"

if ip link show "$TAP_DEVICE" >/dev/null 2>&1; then
    echo "Deleting TAP device: $TAP_DEVICE"
    ip link delete "$TAP_DEVICE" || true
else
    echo "TAP device $TAP_DEVICE does not exist, skipping cleanup"
fi

