use crate::config::ProtocolConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootProtocol {
    Efi,
    Legacy,
    DhcpBoot,
}

pub struct ProtocolHandler;

impl ProtocolHandler {
    pub fn select_protocol(
        config: &ProtocolConfig,
        client_arch: Option<u16>,
    ) -> Option<BootProtocol> {
        // Check client architecture option (option 93)
        if let Some(arch) = client_arch {
            match arch {
                6 => {
                    return if config.efi {
                        Some(BootProtocol::Efi)
                    } else {
                        None
                    }
                }
                0 | 1 => {
                    return if config.legacy {
                        Some(BootProtocol::Legacy)
                    } else {
                        None
                    }
                }
                _ => {}
            }
        }

        // Default selection based on enabled protocols
        if config.efi {
            Some(BootProtocol::Efi)
        } else if config.legacy {
            Some(BootProtocol::Legacy)
        } else if config.dhcp_boot {
            Some(BootProtocol::DhcpBoot)
        } else {
            None
        }
    }

    pub fn get_boot_filename(protocol: BootProtocol) -> &'static str {
        match protocol {
            BootProtocol::Efi => "bootx64.efi",
            BootProtocol::Legacy => "pxelinux.0",
            BootProtocol::DhcpBoot => "pxelinux.0",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProtocolConfig;

    #[test]
    fn test_protocol_selection() {
        let config = ProtocolConfig {
            efi: true,
            legacy: true,
            dhcp_boot: true,
        };

        assert_eq!(
            ProtocolHandler::select_protocol(&config, Some(6)),
            Some(BootProtocol::Efi)
        );
        assert_eq!(
            ProtocolHandler::select_protocol(&config, Some(0)),
            Some(BootProtocol::Legacy)
        );
    }

    #[test]
    fn test_boot_filename() {
        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::Efi),
            "bootx64.efi"
        );
        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::Legacy),
            "pxelinux.0"
        );
    }
}
