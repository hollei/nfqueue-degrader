use crate::nfqueue_wrapper::*;
use std::collections;
use std::time::Duration;

pub struct PacketQueue {
    queue: collections::BTreeMap<Duration, Vec<NfqPacket>>,
}

impl PacketQueue {
    pub fn new() -> PacketQueue {
        PacketQueue {
            queue: collections::BTreeMap::new(),
        }
    }

    pub fn push(&mut self, packet: NfqPacket, send_time: Duration) {
        self.queue
            .entry(send_time)
            .or_insert_with(Vec::new)
            .push(packet);
    }

    pub fn pop(&mut self, time_now: Duration) -> Vec<NfqPacket> {
        let mut packets: Vec<NfqPacket> = Vec::new();

        let times: Vec<_> = self
            .queue
            .iter()
            .filter_map(|entry| {
                if entry.0 <= &time_now {
                    Some(*entry.0)
                } else {
                    None
                }
            })
            .collect();

        for time in &times {
            let mut v = self.queue.remove(time).unwrap();
            packets.append(&mut v);
        }

        packets
    }
}
