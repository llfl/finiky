use crate::filesystem::FileSystem;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing as log;

const BLOCK_SIZE: usize = 512;
const MAX_PACKET_SIZE: usize = 516; // 4 bytes header + 512 bytes data

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TftpOpcode {
    ReadRequest = 1,
    WriteRequest = 2,
    Data = 3,
    Ack = 4,
    Error = 5,
}

#[derive(Debug)]
pub struct TftpPacket {
    opcode: TftpOpcode,
    data: Vec<u8>,
}

impl TftpPacket {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 2 {
            return Err("Packet too short".to_string());
        }

        let opcode = u16::from_be_bytes([data[0], data[1]]);
        let opcode = match opcode {
            1 => TftpOpcode::ReadRequest,
            2 => TftpOpcode::WriteRequest,
            3 => TftpOpcode::Data,
            4 => TftpOpcode::Ack,
            5 => TftpOpcode::Error,
            _ => return Err(format!("Unknown opcode: {}", opcode)),
        };

        Ok(TftpPacket {
            opcode,
            data: data[2..].to_vec(),
        })
    }

    #[allow(dead_code)]
    pub fn build_ack(block_num: u16) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&(TftpOpcode::Ack as u16).to_be_bytes());
        packet.extend_from_slice(&block_num.to_be_bytes());
        packet
    }

    pub fn build_data(block_num: u16, data: &[u8]) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&(TftpOpcode::Data as u16).to_be_bytes());
        packet.extend_from_slice(&block_num.to_be_bytes());
        packet.extend_from_slice(data);
        packet
    }

    pub fn build_error(code: u16, message: &str) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&(TftpOpcode::Error as u16).to_be_bytes());
        packet.extend_from_slice(&code.to_be_bytes());
        packet.extend_from_slice(message.as_bytes());
        packet.push(0); // Null terminator
        packet
    }

    pub fn extract_filename(&self) -> Option<String> {
        if matches!(
            self.opcode,
            TftpOpcode::ReadRequest | TftpOpcode::WriteRequest
        ) {
            let null_pos = self.data.iter().position(|&b| b == 0)?;
            String::from_utf8(self.data[..null_pos].to_vec()).ok()
        } else {
            None
        }
    }
}

pub struct TftpServer {
    port: u16,
    filesystem: Arc<dyn FileSystem>,
}

impl TftpServer {
    pub fn new(port: u16, filesystem: Box<dyn FileSystem>) -> Self {
        TftpServer {
            port,
            filesystem: Arc::from(filesystem),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = Arc::new(UdpSocket::bind(format!("0.0.0.0:{}", self.port)).await?);
        log::info!("TFTP server listening on port {}", self.port);

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        let filesystem = Arc::clone(&self.filesystem);

        loop {
            match socket.recv_from(&mut buf).await {
                Ok((size, peer)) => {
                    let data = &buf[..size];
                    if let Ok(packet) = TftpPacket::parse(data) {
                        let socket_clone = Arc::clone(&socket);
                        let filesystem_clone = Arc::clone(&filesystem);
                        tokio::spawn(Self::handle_request(
                            socket_clone,
                            peer,
                            packet,
                            filesystem_clone,
                        ));
                    }
                }
                Err(e) => {
                    log::error!("TFTP receive error: {}", e);
                }
            }
        }
    }

    async fn handle_request(
        socket: Arc<UdpSocket>,
        peer: SocketAddr,
        packet: TftpPacket,
        filesystem: Arc<dyn FileSystem>,
    ) {
        match packet.opcode {
            TftpOpcode::ReadRequest => {
                if let Some(filename) = packet.extract_filename() {
                    log::debug!("TFTP read request for: {}", filename);
                    Self::handle_read(socket, peer, filename, filesystem).await;
                }
            }
            TftpOpcode::WriteRequest => {
                // TFTP write not supported for PXE boot
                let error = TftpPacket::build_error(2, "Write not supported");
                let _ = socket.send_to(&error, peer).await;
            }
            _ => {
                log::warn!("Unexpected TFTP packet type from {}", peer);
            }
        }
    }

    async fn handle_read(
        socket: Arc<UdpSocket>,
        peer: SocketAddr,
        filename: String,
        filesystem: Arc<dyn FileSystem>,
    ) {
        // Normalize filename (remove leading slash if present)
        let filename = filename.trim_start_matches('/');

        if !filesystem.exists(filename).await {
            log::warn!("TFTP file not found: {}", filename);
            let error = TftpPacket::build_error(1, "File not found");
            let _ = socket.send_to(&error, peer).await;
            return;
        }

        let file_data = match filesystem.read_file(filename).await {
            Ok(data) => data,
            Err(e) => {
                log::error!("Error reading file {}: {}", filename, e);
                let error = TftpPacket::build_error(0, "Error reading file");
                let _ = socket.send_to(&error, peer).await;
                return;
            }
        };

        // Send file in blocks
        let mut block_num = 1u16;
        let mut offset = 0;

        loop {
            let remaining = file_data.len() - offset;
            let chunk_size = remaining.min(BLOCK_SIZE);
            let chunk = &file_data[offset..offset + chunk_size];

            let data_packet = TftpPacket::build_data(block_num, chunk);

            // Send data packet
            if let Err(e) = socket.send_to(&data_packet, peer).await {
                log::error!("Error sending TFTP data: {}", e);
                return;
            }

            // Wait for ACK
            let mut ack_buf = vec![0u8; 4];
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                socket.recv_from(&mut ack_buf),
            )
            .await
            {
                Ok(Ok((size, ack_peer))) if ack_peer == peer => {
                    if size >= 4 {
                        let ack_opcode = u16::from_be_bytes([ack_buf[0], ack_buf[1]]);
                        let ack_block = u16::from_be_bytes([ack_buf[2], ack_buf[3]]);

                        if ack_opcode == TftpOpcode::Ack as u16 && ack_block == block_num {
                            offset += chunk_size;

                            // If this was the last block (less than BLOCK_SIZE), we're done
                            if chunk_size < BLOCK_SIZE {
                                log::debug!("TFTP transfer complete: {}", filename);
                                return;
                            }

                            block_num = block_num.wrapping_add(1);
                            if block_num == 0 {
                                block_num = 1; // Wrap around (though unlikely)
                            }
                        } else {
                            log::warn!("Invalid ACK from {}", peer);
                            return;
                        }
                    }
                }
                Ok(Ok((_, ack_peer))) => {
                    log::warn!("ACK from wrong peer: {}", ack_peer);
                }
                Ok(Err(e)) => {
                    log::error!("Error receiving ACK: {}", e);
                    return;
                }
                Err(_) => {
                    log::warn!("Timeout waiting for ACK");
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::directory::DirectoryFileSystem;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tftp_packet_parsing() {
        let mut data = Vec::new();
        data.extend_from_slice(&(TftpOpcode::ReadRequest as u16).to_be_bytes());
        data.extend_from_slice(b"test.txt");
        data.push(0);
        data.extend_from_slice(b"octet");
        data.push(0);

        let packet = TftpPacket::parse(&data).unwrap();
        assert!(matches!(packet.opcode, TftpOpcode::ReadRequest));
        assert_eq!(packet.extract_filename(), Some("test.txt".to_string()));
    }

    #[tokio::test]
    async fn test_tftp_ack() {
        let ack = TftpPacket::build_ack(1);
        assert_eq!(ack.len(), 4);
        assert_eq!(u16::from_be_bytes([ack[0], ack[1]]), TftpOpcode::Ack as u16);
        assert_eq!(u16::from_be_bytes([ack[2], ack[3]]), 1);
    }
}
