mod config;
mod packet;
mod stats_aggregator;
mod stats_collector;
mod udp_client;
mod udp_server;

fn main() {
    println!("Start network analyzer");
    let cfg = config::Config::from_cli();

    let mut server = None;
    let mut client = None;

    match cfg.mode {
        config::Mode::Client => {
            client = Some(udp_client::UdpClient::new(cfg));
            client.as_mut().unwrap().start();
        }
        config::Mode::Server => {
            server = Some(udp_server::UdpServer::new(cfg));
            server.as_mut().unwrap().start();
        }

        config::Mode::Both => {
            server = Some(udp_server::UdpServer::new(cfg.clone()));
            server.as_mut().unwrap().start();
            client = Some(udp_client::UdpClient::new(cfg));
            client.as_mut().unwrap().start();
        }
    }

    if let Some(c) = client.as_mut() {
        c.wait_for_completion()
    }
    if let Some(s) = server.as_mut() {
        s.wait_for_completion()
    }
}
