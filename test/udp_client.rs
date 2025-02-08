use crate::config;
use crate::packet::Packet;
use crate::stats_collector::PacketStatsCollector;
use std::net::UdpSocket;
use std::time::{Duration, UNIX_EPOCH};

fn send(send_sockets: &[UdpSocket], packet_no: u64, payload_size: u64) {
    let time = UNIX_EPOCH.elapsed().unwrap();
    for (socket_no, send_socket) in send_sockets.iter().enumerate() {
        let p = Packet {
            number: packet_no,
            client_send_time: time,
            client_receive_time: Duration::default(),
            server_send_time: Duration::default(),
            payload: vec![1; payload_size as usize],
        };

        let sent = send_socket.send(p.as_bytes().as_slice());
        match sent {
            Ok(_) => (),
            Err(_) => {
                println!("Send error: packet {}, socket {}", p.number, socket_no);
            }
        }
    }
}

fn receive(receive_sockets: &[UdpSocket], stats_collector: &PacketStatsCollector) {
    let mut buf = [0; 32000];
    for (idx, receive_socket) in receive_sockets.iter().enumerate() {
        if let Ok(size) = receive_socket.recv(&mut buf) {
            let mut p = Packet::from_bytes(&buf[0..size]).unwrap();
            p.client_receive_time = UNIX_EPOCH.elapsed().unwrap();
            stats_collector.collect(idx as u32, &p);
        }
    }
}

pub struct UdpClient {
    cfg: config::Config,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl UdpClient {
    pub fn new(cfg: config::Config) -> UdpClient {
        UdpClient { cfg, thread: None }
    }

    pub fn start(&mut self) {
        let mut send_sockets: Vec<UdpSocket> = Vec::new();
        let mut receive_sockets: Vec<UdpSocket> = Vec::new();
        let mut current_server_port = self.cfg.server_port_range.0;
        for (_, port) in (self.cfg.client_port_range.0..self.cfg.client_port_range.1).enumerate() {
            let client_socket =
                std::net::UdpSocket::bind(self.cfg.client_ip.to_owned() + ":" + &port.to_string())
                    .unwrap();
            client_socket
                .connect(self.cfg.server_ip.to_owned() + ":" + &current_server_port.to_string())
                .unwrap();
            client_socket.set_nonblocking(true).unwrap();
            let receive_socket = client_socket.try_clone().unwrap();
            send_sockets.push(client_socket);
            receive_socket.set_nonblocking(true).unwrap();
            receive_sockets.push(receive_socket);

            if current_server_port >= self.cfg.server_port_range.1 {
                current_server_port = self.cfg.server_port_range.0;
            } else {
                current_server_port += 1;
            }
        }

        let send_interval = self.cfg.send_interval;
        let payload_size = self.cfg.payload_size;
        let thread = std::thread::spawn(move || {
            let mut next_send_time = UNIX_EPOCH.elapsed().unwrap();
            let mut packet_no: u64 = 0;
            let stats_collector = PacketStatsCollector::new();
            loop {
                let t = UNIX_EPOCH.elapsed().unwrap(); // clock.elapsed().unwrap();
                if t >= next_send_time {
                    send(&send_sockets, packet_no, payload_size);
                    packet_no += 1;
                    next_send_time += send_interval;
                }
                receive(&receive_sockets, &stats_collector);
                std::thread::sleep(Duration::from_millis(1));
            }
        });

        self.thread = Some(thread);
    }

    pub fn wait_for_completion(&mut self) {
        match self.thread.take() {
            Some(thread) => {
                thread.join().unwrap();
            }
            None => panic!("client send thread not started"),
        }
    }
}
