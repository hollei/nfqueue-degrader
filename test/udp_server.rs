use crate::config;
use crate::packet::Packet;
use std::net::UdpSocket;

pub struct UdpServer {
    cfg: config::Config,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl UdpServer {
    pub fn new(cfg: config::Config) -> UdpServer {
        UdpServer { cfg, thread: None }
    }

    pub fn start(&mut self) {
        let mut sockets: Vec<UdpSocket> = Vec::new();
        for (_, port) in (self.cfg.server_port_range.0..self.cfg.server_port_range.1).enumerate() {
            let server_socket =
                std::net::UdpSocket::bind(self.cfg.server_ip.to_owned() + ":" + &port.to_string())
                    .unwrap();
            server_socket.set_nonblocking(true).unwrap();
            sockets.push(server_socket);
        }
        let server_thread = std::thread::spawn(move || loop {
            let mut buf = [0; 32000];
            for server_socket in &sockets {
                if let Ok((size, from_addr)) = server_socket.recv_from(&mut buf) {
                    // TODO: why create a Packet ?
                    let _p = Packet::from_bytes(&buf[0..size]).unwrap();

                    if server_socket.send_to(&buf[0..size], from_addr).is_err() {
                        panic!("error in server thread while sending");
                    }
                }
            }
        });

        self.thread = Some(server_thread);
    }

    pub fn wait_for_completion(&mut self) {
        let t = self.thread.take();
        match t {
            Some(thread) => {
                thread.join().unwrap();
            }
            None => panic!("server not started"),
        }
    }
}
