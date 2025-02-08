pub mod bandwidth_queuing_model;
pub mod packet_queue;
pub mod pattern_file_queuing_model;
pub mod queuing_model_chain;
pub mod random_queuing_model;

use crate::nfqueue_wrapper::NfqPacket;
use std::fmt::Display;
use std::time::Duration;

pub trait QueuingModel: Display {
    fn enqueue(&mut self, packet: NfqPacket, time_now: Duration);
    fn dequeue(&mut self, time_now: Duration) -> Vec<NfqPacket>;
}
