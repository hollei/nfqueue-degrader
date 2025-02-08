use crate::packet::Packet;

use std::time::{Duration, UNIX_EPOCH};

#[derive(Default)]
struct Stats {
    received_packets: Vec<u64>,
    received_rtts: u64,
    received_data: u128,
}

impl Stats {
    pub fn new() -> Self {
        Stats::default()
    }

    pub fn append(&mut self, packet: &Packet) {
        let packet_size = std::mem::size_of::<Packet>() as u64 + packet.payload.len() as u64 + 28;
        self.received_data += packet_size as u128;
        self.received_packets.push(packet.number + 1); // for loss calculation never start with 0
        self.received_rtts += packet.rtt().as_millis() as u64;
    }

    pub fn packet_rate(&self, time_period: &Duration) -> f64 {
        (1000 * self.received_packets.len()) as f64 / time_period.as_millis() as f64
    }

    pub fn bandwidth(&self, time_period: &Duration) -> f64 {
        let bandwidth = (1000_f64 * self.received_data as f64) / time_period.as_millis() as f64;
        bandwidth / 1024_f64
    }

    pub fn rtt(&self) -> u64 {
        self.received_rtts / std::cmp::max(1, self.received_packets.len()) as u64
    }

    pub fn loss_rate(&mut self) -> f64 {
        if self.received_packets.is_empty() {
            return 0.0;
        }
        self.received_packets.sort_unstable();
        let last_packet_no = self.received_packets.first().unwrap();
        let loss_counter = self.received_packets[1..]
            .iter()
            .fold((0_u64, last_packet_no), |state, packet_no| {
                (state.0 + packet_no - state.1 - 1, packet_no)
            })
            .0;

        (100 * loss_counter) as f64 / (self.received_packets.len() as f64 + loss_counter as f64)
    }
}

pub struct StatsAggregator {
    id: u32,
    stats: Stats,
    start_time: Duration,
    period: Duration,
}

impl StatsAggregator {
    pub fn new(id: u32, period: Duration) -> StatsAggregator {
        StatsAggregator {
            id,
            stats: Stats::new(),
            start_time: UNIX_EPOCH.elapsed().unwrap(),
            period,
        }
    }

    pub fn data_received(&mut self, packet: &Packet) {
        let now = UNIX_EPOCH.elapsed().unwrap();
        self.stats.append(packet);

        if (now - self.start_time) >= self.period {
            let time_diff = now - self.start_time;
            println!("Stats available for connection {}:  bandwidth {} KB/s, packet rate {}, rtt {} ms, loss rate {} %",
                self.id, self.stats.bandwidth(&time_diff),
                self.stats.packet_rate(&time_diff),
                self.stats.rtt(),
                self.stats.loss_rate());

            self.start_time = now;
            self.stats = Stats::new();
        }
    }
}
