use super::packet_queue::PacketQueue;
use super::QueuingModel;
use crate::nfqueue_wrapper::{NfqPacket, Verdict};
use rand::{
    distributions::{Distribution, Uniform},
    Rng, SeedableRng,
};
use std::fmt::Display;
use std::ops::RangeInclusive;
use std::time::Duration;

pub struct RandomQueuingModel {
    loss_rate: u32,
    delay: Delay,
    rand: rand::rngs::SmallRng,
    queue: PacketQueue,
}

impl RandomQueuingModel {
    pub fn new(loss_rate: u32) -> Self {
        Self {
            loss_rate,
            delay: Delay::default(),
            rand: rand::rngs::SmallRng::from_seed([1; 32]),
            queue: PacketQueue::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_delay(self, delay: Duration) -> Self {
        Self {
            delay: Delay::new(delay),
            ..self
        }
    }

    pub fn with_delay_range(self, range: (Duration, Duration)) -> Self {
        Self {
            delay: Delay::new_with_range(range.0, range.1),
            ..self
        }
    }
}

impl RandomQueuingModel {
    fn drop_packet(&mut self) -> bool {
        let random_no: u32 = self.rand.gen();
        (random_no % 100) < self.loss_rate
    }

    fn get_send_time(&mut self, receive_time: Duration) -> Duration {
        receive_time + self.delay.sample(&mut self.rand)
    }
}

impl QueuingModel for RandomQueuingModel {
    fn enqueue(&mut self, packet: NfqPacket, time_now: Duration) {
        if !self.drop_packet() {
            let send_time = self.get_send_time(time_now);
            self.queue.push(packet, send_time);
        } else {
            packet.set_verdict(Verdict::Drop)
        }
    }

    fn dequeue(&mut self, time_now: Duration) -> Vec<NfqPacket> {
        self.queue.pop(time_now)
    }
}

impl Display for RandomQueuingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "random queuing model, loss: {}, delay: {}",
            self.loss_rate, self.delay
        )
    }
}

enum Delay {
    Fixed(Duration),
    Range(RangeInclusive<Duration>, Uniform<Duration>),
}

impl Delay {
    fn new(value: Duration) -> Self {
        Self::Fixed(value)
    }

    fn new_with_range(low: Duration, high: Duration) -> Self {
        Delay::Range(low..=high, Uniform::new_inclusive(low, high))
    }

    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Duration {
        match self {
            Delay::Fixed(value) => *value,
            Delay::Range(_range, dist) => dist.sample(rng),
        }
    }
}

impl Default for Delay {
    fn default() -> Self {
        Self::new(Duration::from_nanos(0))
    }
}

impl Display for Delay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Delay::Fixed(value) => write!(f, "{} ms", value.as_millis()),
            Delay::Range(range, _dist) => write!(
                f,
                "{}-{} ms",
                range.start().as_millis(),
                range.end().as_millis()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ops::Range, time::Instant};

    use super::*;

    fn confidence_interval_for_random_packet_loss(packet_count: u64, loss_rate: f64) -> Range<f64> {
        // Packet are dropped randomly. Given the number of packets and the configured
        // loss rate, we can estimate a confidence interval for the loss rate.

        assert!(packet_count > 0);
        assert!(loss_rate > 0.0);
        assert!(loss_rate < 1.0);

        // This z score corresponds to a confidence of 99.9% - we accept one out of 1000 tests to fail.
        let z = 3.29;
        let count = packet_count as f64;

        // The formula used here is the so-called "Wilson score interval". It is an approximation for
        // the confidence interval of a binominal proportion.
        // See https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval for details.
        let a = loss_rate + (z * z) / (2.0 * count);
        // b = z * Math.sqrt((rate * (1 - rate) + (z * z) / (4 * count)) / count)
        let b = z * ((loss_rate * (1.0 - loss_rate) + (z * z) / (4.0 * count)) / count).sqrt();
        let denom = 1.0 + (z * z) / count;

        Range {
            start: (a - b) / denom,
            end: (a + b) / denom,
        }
    }

    #[test]
    fn run_degrader_model_for_loss_only() {
        let now = Instant::now();
        let loss_percentage: u32 = 10;
        let mut model = RandomQueuingModel::new(loss_percentage);
        let mut drop_counter: u64 = 0;
        let packet_count = 1000;
        for _ in 0..packet_count {
            if model.drop_packet() {
                drop_counter += 1;
            } else {
                let now = now.elapsed();
                assert_eq!(model.get_send_time(now), now);
            }
        }
        let confidence_interval = confidence_interval_for_random_packet_loss(
            packet_count,
            loss_percentage as f64 / 100.0,
        );
        let drop_rate = drop_counter as f64 / packet_count as f64;
        assert!(confidence_interval.contains(&drop_rate));
    }

    #[test]
    fn run_degrader_model_with_delay() {
        let now = Instant::now();
        let delay = Duration::from_millis(23);
        let mut model = RandomQueuingModel::new(0).with_delay(delay);
        let mut drop_counter: u64 = 0;
        for _ in 0..1000 {
            if model.drop_packet() {
                drop_counter += 1;
            } else {
                let now = now.elapsed();
                assert_eq!(model.get_send_time(now), now + delay);
            }
        }
        assert_eq!(drop_counter, 0);
    }

    #[test]
    fn run_degrader_model_with_delay_range() {
        let now = Instant::now();
        let delay_range = (Duration::from_millis(20), Duration::from_millis(100));
        let mut model = RandomQueuingModel::new(0).with_delay_range(delay_range);
        let mut drop_counter: u64 = 0;
        for _ in 0..1000 {
            if model.drop_packet() {
                drop_counter += 1;
            } else {
                let now = now.elapsed();
                let sent = model.get_send_time(now);
                assert!(sent >= now + delay_range.0);
                assert!(sent <= now + delay_range.1);
            }
        }
        assert_eq!(drop_counter, 0);
    }
}
