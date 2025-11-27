use finiky::tftp::server::{TftpOpcode, TftpPacket};

#[test]
fn test_tftp_packet_parsing() {
    // Test Read Request packet
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

#[test]
fn test_tftp_ack_packet() {
    let ack = TftpPacket::build_ack(1);
    assert_eq!(ack.len(), 4);
    assert_eq!(u16::from_be_bytes([ack[0], ack[1]]), TftpOpcode::Ack as u16);
    assert_eq!(u16::from_be_bytes([ack[2], ack[3]]), 1);
}

#[test]
fn test_tftp_data_packet() {
    let data = b"Hello, World!";
    let packet = TftpPacket::build_data(1, data);

    assert_eq!(packet.len(), 2 + 2 + data.len()); // opcode + block + data
    assert_eq!(
        u16::from_be_bytes([packet[0], packet[1]]),
        TftpOpcode::Data as u16
    );
    assert_eq!(u16::from_be_bytes([packet[2], packet[3]]), 1);
    assert_eq!(&packet[4..], data);
}

#[test]
fn test_tftp_error_packet() {
    let error = TftpPacket::build_error(1, "File not found");
    assert_eq!(
        u16::from_be_bytes([error[0], error[1]]),
        TftpOpcode::Error as u16
    );
    assert_eq!(u16::from_be_bytes([error[2], error[3]]), 1);
    assert!(error.ends_with(&[0])); // Null terminator
}

#[test]
fn test_tftp_invalid_packet() {
    let data = vec![0u8; 1]; // Too short
    assert!(TftpPacket::parse(&data).is_err());
}
