# Finiky - PXE Server

A fast and portable PXE Server written in Rust for rapid OS deployment to bare metal machines.

## Features

- **Easy CLI**: Start DHCP + TFTP + HTTP servers with simple command-line interface
- **Multi-protocol DHCP**: Supports EFI, Legacy BIOS, and DHCP-boot protocols
- **Flexible File Serving**: Serve files from directories or directly from tar.gz archives (virtual filesystem)
- **Portable**: Built with musl support, avoiding GNU-specific dependencies
- **Configurable**: Support for configuration files with command-line overrides

## Installation

```bash
cargo build --release
```

To build for musl-based static binary (for Linux deployment):

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

You can also set an alias for convenience:

```bash
# Add to your shell config (~/.zshrc or ~/.bashrc)
alias cargo-musl='cargo build --release --target x86_64-unknown-linux-musl'

# Then use it
cargo-musl
```

## Usage

### Generate Default Configuration

```bash
# Generate config.toml (default filename)
finiky gen-config

# Generate with custom filename
finiky gen-config my-config.toml
```

This will generate the default configuration file that you can customize.

### Start Server

```bash
# Using default configuration
finiky start

# With configuration file
finiky start --config /path/to/config.toml

# With command-line overrides
finiky start --dhcp-port 67 --tftp-port 69 --http-port 8080 \
  --tftp-root ./tftp --http-root ./http

# Enable/disable specific protocols
finiky start --enable-efi true --enable-legacy false
```

### Configuration File

The configuration file supports the following options:

```toml
[dhcp]
port = 67
interface = null  # Optional network interface name
ip_pool_start = "192.168.1.100"
ip_pool_end = "192.168.1.200"
subnet_mask = "255.255.255.0"
gateway = "192.168.1.1"
dns_servers = ["8.8.8.8", "8.8.4.4"]
next_server = "192.168.1.1"

[dhcp.protocols]
efi = true
legacy = true
dhcp_boot = true

[tftp]
port = 69
root = "./tftp"  # Directory or tar.gz file

[http]
port = 8080
root = "./http"  # Directory or tar.gz file
```

## Features in Detail

### Virtual Filesystem

The server can serve files from:
- **Directories**: Standard directory-based file serving
- **tar.gz archives**: Files are read directly from compressed archives without extraction

Example:
```bash
# Serve from directory
finiky start --tftp-root ./boot_files

# Serve from tar.gz archive
finiky start --tftp-root ./boot_files.tar.gz
```

### DHCP Protocols

- **EFI**: Returns `bootx64.efi` for UEFI systems
- **Legacy**: Returns `pxelinux.0` for legacy BIOS systems
- **DHCP-boot**: Standard DHCP boot protocol

The server automatically detects the client architecture and responds accordingly.

## Testing

Run all tests:

```bash
cargo test
```

Run specific test suite:

```bash
cargo test --test filesystem_tests
cargo test --test dhcp_tests
cargo test --test tftp_tests
cargo test --test http_tests
cargo test --test integration_tests
```

## Architecture

- **DHCP Server**: Handles PXE boot requests, IP allocation, and protocol selection
- **TFTP Server**: Serves boot files using TFTP protocol (RFC 1350)
- **HTTP Server**: Serves larger files and installation media via HTTP
- **Virtual Filesystem**: Abstract filesystem layer supporting directories and tar.gz archives

## License

MIT

