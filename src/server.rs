use crate::config::Config;
use crate::dhcp::DhcpServer;
use crate::filesystem;
use crate::http::HttpServer;
use crate::tftp::TftpServer;
use tokio::signal;
use tracing as log;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Server { config })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting PXE Server...");

        // Create filesystems
        let tftp_fs = filesystem::create_filesystem(&self.config.tftp.root)?;
        let http_fs = filesystem::create_filesystem(&self.config.http.root)?;

        // Create servers
        let dhcp_server = DhcpServer::new(self.config.dhcp.clone())?;
        let tftp_server = TftpServer::new(self.config.tftp.port, tftp_fs);
        let http_server = HttpServer::new(self.config.http.port, http_fs);

        log::info!("All servers initialized");

        // Start all servers concurrently
        let dhcp_handle = tokio::spawn(async move {
            if let Err(e) = dhcp_server.start().await {
                log::error!("DHCP server error: {}", e);
            }
        });

        let tftp_handle = tokio::spawn(async move {
            if let Err(e) = tftp_server.start().await {
                log::error!("TFTP server error: {}", e);
            }
        });

        let http_handle = tokio::spawn(async move {
            if let Err(e) = http_server.start().await {
                log::error!("HTTP server error: {}", e);
            }
        });

        // Wait for shutdown signal
        tokio::select! {
            _ = signal::ctrl_c() => {
                log::info!("Shutdown signal received");
            }
            _ = dhcp_handle => {
                log::warn!("DHCP server stopped");
            }
            _ = tftp_handle => {
                log::warn!("TFTP server stopped");
            }
            _ = http_handle => {
                log::warn!("HTTP server stopped");
            }
        }

        Ok(())
    }
}
