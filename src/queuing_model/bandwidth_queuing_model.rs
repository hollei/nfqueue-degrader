use super::QueuingModel;
use crate::nfqueue_wrapper::{NfqPacket, Verdict};
use std::fmt::Display;
use std::time::Duration;

struct TokenBucket {
    token_count: u64, // 1 token is one byte
    max_tokens: u64,
    rate: u64, // bytes per second
    last_token_time: Duration,
}

impl TokenBucket {
    fn new(rate: u64, burst_size_bytes: u64) -> TokenBucket {
        TokenBucket {
            token_count: 0,
            max_tokens: burst_size_bytes,
            rate,
            last_token_time: Duration::default(),
        }
    }

    fn add_token(&mut self, time_now: Duration) {
        let diff_us = (time_now - self.last_token_time).as_micros() as u64;
        let token_count = (self.rate * diff_us) / 1000000;
        if token_count > 0 {
            self.last_token_time = time_now;
        }

        if (self.token_count + token_count) < self.max_tokens {
            self.token_count += token_count;
        } else {
            self.token_count = self.max_tokens;
        }
    }

    fn remove_token(&mut self, packet_size_bytes: u64) -> bool {
        if self.token_count < packet_size_bytes {
            if self.token_count == self.max_tokens {
                log::error!("burst size smaller than packet size");
                self.token_count = 0;
                return true;
            }
            return false;
        }
        self.token_count -= packet_size_bytes;
        true
    }
}

pub struct BandwidthQueuingModel {
    token_bucket: TokenBucket,
    buffer: Vec<NfqPacket>,
    current_buffer_size: u64, // in bytes
    max_buffer_size: u64,     // in bytes
}

impl BandwidthQueuingModel {
    // info: passed parameters are in KB and must be converted to bytes
    pub fn new(rate: u64, burst_size: u64, buffer_size: u64) -> BandwidthQueuingModel {
        BandwidthQueuingModel {
            token_bucket: TokenBucket::new(rate * 1024, burst_size * 1024),
            current_buffer_size: 0,
            max_buffer_size: buffer_size * 1024,
            buffer: Vec::new(),
        }
    }
}

impl QueuingModel for BandwidthQueuingModel {
    fn enqueue(&mut self, packet: NfqPacket, _: Duration) {
        let packet_size = packet.payload.len() as u64;
        if self.max_buffer_size == 0
            || self.max_buffer_size >= (self.current_buffer_size + packet_size)
        {
            self.buffer.push(packet);
            self.current_buffer_size += packet_size;
        } else {
            // TODO: implement another prioritization logic???
            // e.g. randomly drop either a packet in buffer or the new one??
            // otherwise connections with higher packet rates have higher prio
            // or if all connections have same packet rate, the first connection may get higher prio
            packet.set_verdict(Verdict::Drop);
        }
    }

    fn dequeue(&mut self, time_now: Duration) -> Vec<NfqPacket> {
        self.token_bucket.add_token(time_now);

        let mut packets = Vec::<NfqPacket>::new();
        while !self.buffer.is_empty() {
            let packet_size = self.buffer[0].payload.len() as u64;
            if self.token_bucket.remove_token(packet_size) {
                let packet = self.buffer.remove(0);
                packets.push(packet);
            } else {
                break;
            }
        }

        let total_size = packets
            .iter()
            .fold(0, |total_size, p| total_size + p.payload.len() as u64);

        if self.current_buffer_size < total_size {
            panic!("unexpected buffer size");
        }
        self.current_buffer_size -= total_size;
        packets
    }
}

impl Display for BandwidthQueuingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bandwidth queing model: rate {}, burst_size {}, buffer_size {}",
            self.token_bucket.rate / 1024,
            self.token_bucket.max_tokens / 1024,
            self.max_buffer_size / 1024
        )
    }
}
