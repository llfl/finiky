use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
mod dhcp;
mod filesystem;
mod http;
mod server;
mod tftp;

const DEFAULT_CONFIG: &str = include_str!("../examples/config.toml");

#[derive(Parser)]
#[command(name = "finiky")]
#[command(about = "PXE Server for rapid OS deployment to bare metal")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// DHCP server port
    #[arg(long)]
    dhcp_port: Option<u16>,

    /// TFTP server port
    #[arg(long)]
    tftp_port: Option<u16>,

    /// HTTP server port
    #[arg(long)]
    http_port: Option<u16>,

    /// TFTP root directory or tar.gz file
    #[arg(long)]
    tftp_root: Option<PathBuf>,

    /// HTTP root directory or tar.gz file
    #[arg(long)]
    http_root: Option<PathBuf>,

    /// Network interface to bind to
    #[arg(long)]
    interface: Option<String>,

    /// Enable EFI protocol
    #[arg(long)]
    enable_efi: Option<bool>,

    /// Enable Legacy protocol
    #[arg(long)]
    enable_legacy: Option<bool>,

    /// Enable DHCP-boot protocol
    #[arg(long)]
    enable_dhcp_boot: Option<bool>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the default embedded configuration file
    PrintConfig,
    /// Start the PXE server
    Start,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::PrintConfig) => {
            println!("{}", DEFAULT_CONFIG);
            return Ok(());
        }
        Some(Commands::Start) | None => {
            let mut config = if let Some(config_path) = cli.config {
                config::Config::from_file(&config_path)?
            } else {
                config::Config::default()
            };

            // Apply command line overrides
            if let Some(port) = cli.dhcp_port {
                config.dhcp.port = port;
            }
            if let Some(port) = cli.tftp_port {
                config.tftp.port = port;
            }
            if let Some(port) = cli.http_port {
                config.http.port = port;
            }
            if let Some(root) = cli.tftp_root {
                config.tftp.root = root.to_string_lossy().to_string();
            }
            if let Some(root) = cli.http_root {
                config.http.root = root.to_string_lossy().to_string();
            }
            if let Some(interface) = cli.interface {
                config.dhcp.interface = Some(interface);
            }
            if let Some(enabled) = cli.enable_efi {
                config.dhcp.protocols.efi = enabled;
            }
            if let Some(enabled) = cli.enable_legacy {
                config.dhcp.protocols.legacy = enabled;
            }
            if let Some(enabled) = cli.enable_dhcp_boot {
                config.dhcp.protocols.dhcp_boot = enabled;
            }

            server::Server::new(config)?.start().await?;
        }
    }

    Ok(())
}
