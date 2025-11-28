#!/bin/bash
# Prepare PXE boot files

set -euo pipefail

FIXTURES_DIR="${1:-tests/fixtures}"

echo "Preparing PXE boot files to: $FIXTURES_DIR"

# Create directories
mkdir -p "$FIXTURES_DIR/pxelinux.cfg"

# Check if syslinux is available
if command -v syslinux >/dev/null 2>&1; then
    # Find syslinux file locations
    SYSLINUX_DIR=""
    for dir in /usr/lib/syslinux /usr/share/syslinux /usr/lib/syslinux/modules/bios; do
        if [ -d "$dir" ]; then
            SYSLINUX_DIR="$dir"
            break
        fi
    done
    
    if [ -n "$SYSLINUX_DIR" ]; then
        # Copy pxelinux.0
        if [ -f "$SYSLINUX_DIR/../pxelinux.0" ]; then
            cp "$SYSLINUX_DIR/../pxelinux.0" "$FIXTURES_DIR/" 2>/dev/null || true
        elif [ -f "$SYSLINUX_DIR/pxelinux.0" ]; then
            cp "$SYSLINUX_DIR/pxelinux.0" "$FIXTURES_DIR/" 2>/dev/null || true
        fi
        
        # Copy EFI boot file
        if [ -f "$SYSLINUX_DIR/../efi64/efi/syslinux.efi" ]; then
            cp "$SYSLINUX_DIR/../efi64/efi/syslinux.efi" "$FIXTURES_DIR/bootx64.efi" 2>/dev/null || true
        elif [ -f "$SYSLINUX_DIR/../efi32/efi/syslinux.efi" ]; then
            cp "$SYSLINUX_DIR/../efi32/efi/syslinux.efi" "$FIXTURES_DIR/bootx64.efi" 2>/dev/null || true
        fi
        
        # Copy ldlinux.c32 (required by pxelinux)
        if [ -f "$SYSLINUX_DIR/ldlinux.c32" ]; then
            cp "$SYSLINUX_DIR/ldlinux.c32" "$FIXTURES_DIR/" 2>/dev/null || true
        fi
    fi
fi

# If files don't exist, provide download instructions
if [ ! -f "$FIXTURES_DIR/pxelinux.0" ]; then
    echo "Warning: pxelinux.0 not found"
    echo "Please copy from syslinux package, or download from:"
    echo "  - https://mirrors.edge.kernel.org/pub/linux/utils/boot/syslinux/"
    echo ""
    echo "Or install syslinux using:"
    echo "  Ubuntu/Debian: sudo apt-get install syslinux-common"
    echo "  CentOS/RHEL: sudo yum install syslinux"
fi

if [ ! -f "$FIXTURES_DIR/bootx64.efi" ]; then
    echo "Warning: bootx64.efi not found"
    echo "UEFI tests may require this file"
    echo "Can be obtained from iPXE project: https://ipxe.org/download"
fi

# Create default boot menu configuration
cat > "$FIXTURES_DIR/pxelinux.cfg/default" <<'EOF'
DEFAULT menu.c32
PROMPT 0
MENU TITLE PXE Boot Menu
TIMEOUT 50

LABEL local
  MENU LABEL Boot from local disk
  LOCALBOOT 0

LABEL test
  MENU LABEL Test PXE Boot
  KERNEL http://192.168.100.1:8080/vmlinuz
  APPEND initrd=http://192.168.100.1:8080/initrd.img console=ttyS0

LABEL shell
  MENU LABEL PXE Shell
  KERNEL http://192.168.100.1:8080/vmlinuz
  APPEND initrd=http://192.168.100.1:8080/initrd.img console=ttyS0 init=/bin/sh
EOF

echo "PXE file preparation completed"
echo "File list:"
ls -lh "$FIXTURES_DIR" || true

