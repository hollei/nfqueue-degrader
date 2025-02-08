use crate::degrader_model::DegraderModel;
use std::time::{SystemTime, Duration};
use rand::Rng;

pub struct RandomDegraderModel {
    loss_rate: u32,
    delay: Duration,
    delay_variation_max: Duration,
    packet_counter: u64
}

impl RandomDegraderModel {
    pub fn new(loss_rate: u32, delay: Duration, delay_variation_max: Duration) -> RandomDegraderModel {
        RandomDegraderModel{
            loss_rate: loss_rate,
            delay: delay,
            delay_variation_max: delay_variation_max,
            packet_counter: 0
        }
    }
}

impl DegraderModel for RandomDegraderModel {
    fn drop_packet(&self, packet_no: u32) -> bool {
        let random_no: u32 = rand::thread_rng().gen();
        (random_no % 100) < self.loss_rate
    }

    fn get_send_time(&self, packet_no: u32, receive_time: Duration) -> Duration {
        let random_no: u128 = rand::thread_rng().gen();
        // TODO: variation can also be negative?
        let variation = random_no %self.delay_variation_max.as_micros();
        let var_duration = Duration::from_micros(variation as u64);
        receive_time + self.delay + var_duration
    }
}