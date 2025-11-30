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

    pub fn get_boot_filename(protocol: BootProtocol, config: &ProtocolConfig) -> String {
        match protocol {
            BootProtocol::Efi => config
                .boot_filename_efi
                .clone()
                .unwrap_or_else(|| "bootx64.efi".to_string()),
            BootProtocol::Legacy => config
                .boot_filename_legacy
                .clone()
                .unwrap_or_else(|| "pxelinux.0".to_string()),
            BootProtocol::DhcpBoot => config
                .boot_filename_dhcp_boot
                .clone()
                .unwrap_or_else(|| "pxelinux.0".to_string()),
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
            boot_filename_efi: None,
            boot_filename_legacy: None,
            boot_filename_dhcp_boot: None,
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
        let config = ProtocolConfig {
            efi: true,
            legacy: true,
            dhcp_boot: true,
            boot_filename_efi: None,
            boot_filename_legacy: None,
            boot_filename_dhcp_boot: None,
        };

        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::Efi, &config),
            "bootx64.efi"
        );
        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::Legacy, &config),
            "pxelinux.0"
        );
    }

    #[test]
    fn test_boot_filename_custom() {
        let config = ProtocolConfig {
            efi: true,
            legacy: true,
            dhcp_boot: true,
            boot_filename_efi: Some("custom_efi.efi".to_string()),
            boot_filename_legacy: Some("custom_legacy.0".to_string()),
            boot_filename_dhcp_boot: Some("custom_dhcp.0".to_string()),
        };

        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::Efi, &config),
            "custom_efi.efi"
        );
        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::Legacy, &config),
            "custom_legacy.0"
        );
        assert_eq!(
            ProtocolHandler::get_boot_filename(BootProtocol::DhcpBoot, &config),
            "custom_dhcp.0"
        );
    }
}
