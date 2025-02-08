use crate::nfqueue_wrapper::*;
use crate::degrader_model::*;
use std::collections;

pub struct PacketQueue {
    queue: collections::BTreeMap<std::time::Duration, Vec<NfqPacket>>
}

impl PacketQueue {
    pub fn new() -> PacketQueue {
        PacketQueue{
            queue: collections::BTreeMap::new()
        }
    }
    pub fn enqueue(&mut self, model: &dyn DegraderModel, packet: NfqPacket, time_now: std::time::Duration) {
        if model.drop_packet(packet.id) {
            packet.set_verdict(Verdict::Drop);
        }

        let send_time = model.get_send_time(packet.id, time_now);
        self.queue.entry(send_time).or_insert(Vec::new()).push(packet);
    }

    pub fn dequeue(&mut self, time_now: std::time::Duration) {
        let mut vec: Vec<NfqPacket> = Vec::new();

        let mut times: Vec<std::time::Duration> = Vec::new();
        for entry in &self.queue {
            if entry.0 <= &time_now {

                times.push(*entry.0);
            }
        }

        for time in &times {
            let mut v = self.queue.remove(&time).unwrap();
            vec.append(& mut v);
        }
        
        for packet in &vec {
            packet.set_verdict(Verdict::Accept);
        }
    }
}

