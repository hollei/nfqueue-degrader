use crate::packet::Packet;
use crate::stats_aggregator::StatsAggregator;
use std::io::{ErrorKind, Write};
use std::sync::mpsc;
use std::time::Duration;

struct SocketPacket {
    pub socket_num: u32,
    pub data: Packet,
}

fn thread_func(rx: mpsc::Receiver<SocketPacket>) {
    let mut files = std::collections::HashMap::new();
    let mut bandwidth_stats = std::collections::HashMap::new();

    loop {
        let received = rx.recv_timeout(Duration::from_millis(1));
        if let Ok(packet) = received {
            let file_name = packet.socket_num.to_string() + "_stats.txt";

            if files.get(&packet.socket_num).is_none() {
                match std::fs::remove_file(&file_name) {
                    Err(e) if e.kind() == ErrorKind::NotFound => (),
                    Err(e) => log::error!("Error removing file {}: {}", file_name, e),
                    Ok(()) => (),
                }
            }

            let bandwidth_stats = bandwidth_stats
                .entry(packet.socket_num)
                .or_insert_with(|| StatsAggregator::new(packet.socket_num, Duration::from_secs(1)));
            bandwidth_stats.data_received(&packet.data);
            let file = files.entry(packet.socket_num).or_insert_with(|| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_name)
                    .unwrap()
            });
            file.write_fmt(format_args!(
                "{}, {}\n",
                packet.data.number,
                packet.data.rtt().as_millis()
            ))
            .unwrap();
        }
    }
}

pub struct PacketStatsCollector {
    sender: mpsc::Sender<SocketPacket>,
}

impl PacketStatsCollector {
    pub fn new() -> PacketStatsCollector {
        let (tx, rx): (mpsc::Sender<_>, mpsc::Receiver<SocketPacket>) = mpsc::channel();

        std::thread::spawn(move || {
            thread_func(rx);
        });

        PacketStatsCollector { sender: tx }
    }

    pub fn collect(&self, socket_num: u32, p: &Packet) {
        self.sender
            .send(SocketPacket {
                socket_num,
                data: p.clone(),
            })
            .unwrap();
    }
}
