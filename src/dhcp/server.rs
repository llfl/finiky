use crate::config::DhcpConfig;
use crate::dhcp::options::DhcpOptions;
use crate::dhcp::protocols::ProtocolHandler;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing as log;

const DHCP_SERVER_PORT: u16 = 67;

#[derive(Debug, Clone)]
pub struct DhcpMessage {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub secs: u16,
    pub flags: u16,
    pub ciaddr: Ipv4Addr,
    pub yiaddr: Ipv4Addr,
    pub siaddr: Ipv4Addr,
    pub giaddr: Ipv4Addr,
    pub chaddr: [u8; 16],
    pub options: Vec<u8>,
}

impl DhcpMessage {
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 240 {
            return Err("DHCP message too short".to_string());
        }

        let xid = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ciaddr = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
        let yiaddr = Ipv4Addr::new(data[16], data[17], data[18], data[19]);
        let siaddr = Ipv4Addr::new(data[20], data[21], data[22], data[23]);
        let giaddr = Ipv4Addr::new(data[24], data[25], data[26], data[27]);

        let mut chaddr = [0u8; 16];
        chaddr[..16].copy_from_slice(&data[28..44]);

        let options = if data.len() > 240 {
            data[240..].to_vec()
        } else {
            Vec::new()
        };

        Ok(DhcpMessage {
            op: data[0],
            htype: data[1],
            hlen: data[2],
            hops: data[3],
            xid,
            secs: u16::from_be_bytes([data[8], data[9]]),
            flags: u16::from_be_bytes([data[10], data[11]]),
            ciaddr,
            yiaddr,
            giaddr,
            siaddr,
            chaddr,
            options,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = vec![0u8; 240];
        data[0] = self.op;
        data[1] = self.htype;
        data[2] = self.hlen;
        data[3] = self.hops;
        data[4..8].copy_from_slice(&self.xid.to_be_bytes());
        data[8..10].copy_from_slice(&self.secs.to_be_bytes());
        data[10..12].copy_from_slice(&self.flags.to_be_bytes());
        data[12..16].copy_from_slice(&self.ciaddr.octets());
        data[16..20].copy_from_slice(&self.yiaddr.octets());
        data[20..24].copy_from_slice(&self.siaddr.octets());
        data[24..28].copy_from_slice(&self.giaddr.octets());
        data[28..44].copy_from_slice(&self.chaddr);
        data.extend_from_slice(&self.options);
        data
    }

    pub fn get_option(&self, option: u8) -> Option<&[u8]> {
        let mut i = 0;
        while i < self.options.len() {
            if self.options[i] == 255 {
                break; // End option
            }
            if self.options[i] == option && i + 1 < self.options.len() {
                let len = self.options[i + 1] as usize;
                if i + 2 + len <= self.options.len() {
                    return Some(&self.options[i + 2..i + 2 + len]);
                }
            }
            if i + 1 < self.options.len() {
                let len = self.options[i + 1] as usize;
                i += 2 + len;
            } else {
                break;
            }
        }
        None
    }

    pub fn get_message_type(&self) -> Option<u8> {
        self.get_option(53)
            .map(|v| if !v.is_empty() { v[0] } else { 0 })
    }

    pub fn get_client_arch(&self) -> Option<u16> {
        self.get_option(93).map(|v| {
            if v.len() >= 2 {
                u16::from_be_bytes([v[0], v[1]])
            } else {
                0
            }
        })
    }
}

pub struct DhcpServer {
    config: Arc<DhcpConfig>,
    ip_pool: IpPool,
}

struct IpPool {
    start: Ipv4Addr,
    end: Ipv4Addr,
    current: std::sync::Mutex<Ipv4Addr>,
    leases: std::sync::Mutex<std::collections::HashMap<[u8; 6], Ipv4Addr>>,
}

impl IpPool {
    fn new(start: Ipv4Addr, end: Ipv4Addr) -> Self {
        IpPool {
            start,
            end,
            current: std::sync::Mutex::new(start),
            leases: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn allocate(&self, mac: [u8; 6]) -> Option<Ipv4Addr> {
        let mut leases = self.leases.lock().unwrap();

        // Check if MAC already has a lease
        if let Some(&ip) = leases.get(&mac) {
            return Some(ip);
        }

        // Allocate new IP
        let mut current = self.current.lock().unwrap();
        let mut candidate = *current;

        loop {
            if candidate > self.end {
                candidate = self.start;
            }

            // Check if IP is already leased
            let is_leased = leases.values().any(|&ip| ip == candidate);
            if !is_leased {
                *current = {
                    let octets = candidate.octets();
                    let last = octets[3].wrapping_add(1);
                    if last == 0 {
                        Ipv4Addr::new(octets[0], octets[1], octets[2].wrapping_add(1), 0)
                    } else {
                        Ipv4Addr::new(octets[0], octets[1], octets[2], last)
                    }
                };
                leases.insert(mac, candidate);
                return Some(candidate);
            }

            let octets = candidate.octets();
            let last = octets[3].wrapping_add(1);
            candidate = if last == 0 {
                Ipv4Addr::new(octets[0], octets[1], octets[2].wrapping_add(1), 0)
            } else {
                Ipv4Addr::new(octets[0], octets[1], octets[2], last)
            };

            if candidate == *current {
                return None; // Pool exhausted
            }
        }
    }
}

impl DhcpServer {
    pub fn new(config: DhcpConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let start = config.ip_pool_start.parse::<Ipv4Addr>()?;
        let end = config.ip_pool_end.parse::<Ipv4Addr>()?;

        Ok(DhcpServer {
            config: Arc::new(config),
            ip_pool: IpPool::new(start, end),
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create raw socket for DHCP
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

        // Set socket options for broadcast
        socket.set_broadcast(true)?;
        socket.set_reuse_address(true)?;

        // Bind to DHCP server port
        let addr = SocketAddr::from(([0, 0, 0, 0], DHCP_SERVER_PORT));
        socket.bind(&addr.into())?;

        // Convert to tokio UdpSocket
        socket.set_nonblocking(true)?;
        let std_socket = std::net::UdpSocket::from(socket);
        let udp_socket = UdpSocket::from_std(std_socket)?;

        log::info!("DHCP server listening on port {}", DHCP_SERVER_PORT);

        let mut buf = vec![0u8; 1500];
        let config = Arc::clone(&self.config);
        let ip_pool = &self.ip_pool;

        loop {
            match udp_socket.recv_from(&mut buf).await {
                Ok((size, peer)) => {
                    let data = &buf[..size];
                    if let Ok(request) = DhcpMessage::from_bytes(data) {
                        if let Some(response) =
                            self.handle_request(&request, ip_pool, &config).await
                        {
                            let response_bytes = response.to_bytes();
                            if let Err(e) = udp_socket.send_to(&response_bytes, peer).await {
                                log::error!("Failed to send DHCP response: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("DHCP receive error: {}", e);
                }
            }
        }
    }

    async fn handle_request(
        &self,
        request: &DhcpMessage,
        ip_pool: &IpPool,
        config: &Arc<DhcpConfig>,
    ) -> Option<DhcpMessage> {
        let msg_type = request.get_message_type()?;

        // Handle both Discover (1) and Request (3) the same way
        if msg_type != 1 && msg_type != 3 {
            return None;
        }

        if msg_type == 1 {
            log::debug!("Received DHCP Discover from {:?}", request.chaddr);
        } else {
            log::debug!("Received DHCP Request from {:?}", request.chaddr);
        }

        let mac = {
            let mut mac = [0u8; 6];
            mac.copy_from_slice(&request.chaddr[..6]);
            mac
        };

        let client_ip = ip_pool.allocate(mac)?;
        let client_arch = request.get_client_arch();

        let protocol = ProtocolHandler::select_protocol(&config.protocols, client_arch)?;
        let filename = ProtocolHandler::get_boot_filename(protocol);

        let mut response = DhcpMessage {
            op: 2, // BOOTREPLY
            htype: request.htype,
            hlen: request.hlen,
            hops: 0,
            xid: request.xid,
            secs: 0,
            flags: request.flags,
            ciaddr: Ipv4Addr::UNSPECIFIED,
            yiaddr: client_ip,
            siaddr: config.next_server.parse().ok()?,
            giaddr: Ipv4Addr::UNSPECIFIED,
            chaddr: request.chaddr,
            options: Vec::new(),
        };

        let mut options = DhcpOptions::build_options(config, client_ip);
        let filename_options = DhcpOptions::build_filename_option(filename);
        options.pop(); // Remove end marker
        options.extend_from_slice(&filename_options);

        response.options = options;

        Some(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_dhcp_message_parsing() {
        let mut data = vec![0u8; 240];
        data[0] = 1; // BOOTREQUEST
        data[1] = 1; // Ethernet
        data[2] = 6; // MAC length
        data[4..8].copy_from_slice(&0x12345678u32.to_be_bytes());

        let msg = DhcpMessage::from_bytes(&data).unwrap();
        assert_eq!(msg.op, 1);
        assert_eq!(msg.xid, 0x12345678);
    }

    #[test]
    fn test_ip_pool() {
        let start = "192.168.1.100".parse().unwrap();
        let end = "192.168.1.110".parse().unwrap();
        let pool = IpPool::new(start, end);

        let mac1 = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let ip1 = pool.allocate(mac1).unwrap();
        assert_eq!(ip1, start);

        let ip1_again = pool.allocate(mac1).unwrap();
        assert_eq!(ip1_again, ip1); // Same MAC gets same IP
    }
}
