use crate::config::DhcpConfig;
use std::net::Ipv4Addr;

pub struct DhcpOptions;

impl DhcpOptions {
    pub fn build_options(config: &DhcpConfig, _client_ip: Ipv4Addr) -> Vec<u8> {
        let mut options = vec![
            // Message type: DHCP Offer
            53, // DHCP Message Type
            1, 2, // Offer
            1, // Subnet Mask
            4,
        ];

        let mask = parse_ip(&config.subnet_mask).unwrap();
        options.extend_from_slice(&mask.octets());

        // Router (gateway)
        if let Some(ref gateway) = config.gateway {
            if let Ok(gw_ip) = gateway.parse::<Ipv4Addr>() {
                options.push(3); // Router
                options.push(4);
                options.extend_from_slice(&gw_ip.octets());
            }
        }

        // DNS servers
        if !config.dns_servers.is_empty() {
            options.push(6); // DNS Servers
            options.push((config.dns_servers.len() * 4) as u8);
            for dns in &config.dns_servers {
                if let Ok(dns_ip) = dns.parse::<Ipv4Addr>() {
                    options.extend_from_slice(&dns_ip.octets());
                }
            }
        }

        // IP Address Lease Time (1 hour)
        options.push(51); // IP Address Lease Time
        options.push(4);
        let lease_time: u32 = 3600;
        options.extend_from_slice(&lease_time.to_be_bytes());

        // Server Identifier (next-server)
        options.push(54); // Server Identifier
        options.push(4);
        if let Ok(server_ip) = config.next_server.parse::<Ipv4Addr>() {
            options.extend_from_slice(&server_ip.octets());
        }

        // End option
        options.push(255);

        options
    }

    pub fn build_filename_option(filename: &str) -> Vec<u8> {
        let mut options = Vec::new();
        options.push(67); // Bootfile Name
        options.push(filename.len() as u8);
        options.extend_from_slice(filename.as_bytes());
        options.push(255); // End
        options
    }
}

fn parse_ip(ip_str: &str) -> Result<Ipv4Addr, std::net::AddrParseError> {
    ip_str.parse::<Ipv4Addr>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DhcpConfig};

    #[test]
    fn test_build_options() {
        let config = Config::default();
        let client_ip = "192.168.1.100".parse().unwrap();
        let options = DhcpOptions::build_options(&config.dhcp, client_ip);

        assert!(!options.is_empty());
        assert_eq!(options[0], 53); // Message Type
    }
}
