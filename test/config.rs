use crate::packet::Packet;
use clap::{App, Arg};
use std::time::Duration;

#[derive(Copy, Clone)]
pub enum Mode {
    Server,
    Client,
    Both,
}

#[derive(Clone)]
pub struct Config {
    pub client_ip: String,
    pub client_port_range: (u16, u16),
    pub server_ip: String,
    pub server_port_range: (u16, u16),
    pub mode: Mode,
    pub send_interval: Duration,
    pub payload_size: u64,
}

fn parse_port_range(range: &str) -> (u16, u16) {
    let range: Vec<&str> = range.split(':').collect();
    let range_start = range[0].parse::<u16>().unwrap();
    let range_end = range[1].parse::<u16>().unwrap();
    (range_start, range_end)
}

impl Config {
    pub fn from_cli() -> Config {
        let matches = App::new("network analyzer")
            .version("1.0.0")
            .author("Holger Kaden <holger.kaden@logmein.com>")
            .about(
                "tool to analyse network rtt, delay variation and packet loss
                for multiple, simultanuous udp connections",
            )
            .arg(
                Arg::with_name("server_ip")
                    .long("server_ip")
                    .takes_value(true)
                    .default_value("127.0.0.1")
                    .help("listen ip address of server"),
            )
            .arg(
                Arg::with_name("client_ip")
                    .long("client_ip")
                    .takes_value(true)
                    .default_value("127.0.0.1")
                    .help("listen ip address of server"),
            )
            .arg(
                Arg::with_name("client_ports")
                    .long("client_ports")
                    .takes_value(true)
                    .default_value("30000:30010")
                    .help("client receive port range"),
            )
            .arg(
                Arg::with_name("server_ports")
                    .long("server_ports")
                    .takes_value(true)
                    .default_value("40000:40010")
                    .help("server receive port range"),
            )
            .arg(
                Arg::with_name("server")
                    .long("server")
                    .takes_value(true)
                    .default_value("1")
                    .help("start in server mode"),
            )
            .arg(
                Arg::with_name("client")
                    .long("client")
                    .takes_value(true)
                    .default_value("1")
                    .help("start in client mode"),
            )
            .arg(
                Arg::with_name("send_interval_ms")
                    .long("send_interval_ms")
                    .takes_value(true)
                    .default_value("20")
                    .help("distance between two sent packets"),
            )
            .arg(
                Arg::with_name("ip_packet_size")
                    .long("ip_packet_size")
                    .takes_value(true)
                    .default_value("100")
                    .help("size in bytes for an ip packet (includes protocol headers)"),
            )
            .get_matches();

        let ip_packet_size = matches
            .value_of("ip_packet_size")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let packet_size = std::mem::size_of::<Packet>();
        // ip + udp header has 28 bytes
        let payload_size = if packet_size + 28 <= ip_packet_size {
            ip_packet_size - packet_size - 28
        } else {
            println!(
                "Warning: Set ip packet size to minimum: {} bytes",
                packet_size + 28
            );
            0
        };

        let send_interval = Duration::from_millis(match &matches.value_of("send_interval_ms") {
            Some(val) => val.parse::<u64>().unwrap(),
            None => panic!("error: parsing value for send_interval_ms"),
        });

        let client_ip = String::from(matches.value_of("client_ip").unwrap());
        let server_ip = String::from(matches.value_of("server_ip").unwrap());
        let client_ports = parse_port_range(matches.value_of("client_ports").unwrap());
        let server_ports = parse_port_range(matches.value_of("server_ports").unwrap());

        let is_server = matches.is_present("server");
        let is_client = matches.is_present("client");

        let mut mode = Mode::Both;
        if is_server && !is_client {
            mode = Mode::Server;
        }
        if !is_server && is_client {
            mode = Mode::Client;
        }

        Config {
            server_ip,
            client_ip,
            server_port_range: server_ports,
            client_port_range: client_ports,
            mode,
            send_interval,
            payload_size: payload_size as u64,
        }
    }
}
