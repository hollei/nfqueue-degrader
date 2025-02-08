#[derive(Default, PartialEq, Eq, Hash)]
pub struct ProtocolInfo {
    pub source_ip: [u8; 4],
    pub source_port: u16,
    pub destination_ip: [u8; 4],
    pub destination_port: u16,
    pub protocol: u8,
}

impl ProtocolInfo {
    pub fn from_ipv4_header(payload: &[u8]) -> Self {
        let ip_header = etherparse::PacketHeaders::from_ip_slice(payload);
        if ip_header.is_err() {
            return ProtocolInfo::default();
        }
        let ip_header = ip_header.unwrap();
        let ip_addr = match ip_header.ip.as_ref().unwrap() {
            etherparse::IpHeader::Version4(header) => {
                (header.source, header.destination, header.protocol)
            }
            _ => {
                ([0, 0, 0, 0], [0, 0, 0, 0], 0)
                //panic!("No ipv4 header")
            }
        };
        let ports = ports_from_ipv4_header(&ip_header);

        ProtocolInfo {
            source_ip: ip_addr.0,
            source_port: ports.0,
            destination_ip: ip_addr.1,
            destination_port: ports.1,
            protocol: ip_addr.2,
        }
    }
    pub fn source_ip_to_string(&self) -> String {
        let addr = std::net::Ipv4Addr::new(
            self.source_ip[0],
            self.source_ip[1],
            self.source_ip[2],
            self.source_ip[3],
        );
        format!("{}", addr)
    }

    pub fn destination_ip_to_string(&self) -> String {
        let addr = std::net::Ipv4Addr::new(
            self.destination_ip[0],
            self.destination_ip[1],
            self.destination_ip[2],
            self.destination_ip[3],
        );
        format!("{}", addr)
    }
}

impl std::fmt::Display for ProtocolInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "source ip: {}, source port: {}, target ip: {}, target port: {}, protocol: {}",
            self.source_ip_to_string(),
            self.source_port,
            self.destination_ip_to_string(),
            self.destination_port,
            self.protocol
        )
    }
}

fn ports_from_ipv4_header(header: &etherparse::PacketHeaders) -> (u16, u16) {
    let transport = header.transport.as_ref();
    match transport {
        Some(etherparse::TransportHeader::Udp(udp_header)) => {
            (udp_header.source_port, udp_header.destination_port)
        }
        Some(etherparse::TransportHeader::Tcp(tcp_header)) => {
            (tcp_header.source_port, tcp_header.destination_port)
        }
        None => (0, 0),
    }
}
