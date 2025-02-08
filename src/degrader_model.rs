use std::time::{Duration};

pub trait DegraderModel {
    fn drop_packet(&self, packetNo: u32) -> bool;
    fn get_send_time(&self, packetNo: u32, receive_time: Duration) -> Duration;
}