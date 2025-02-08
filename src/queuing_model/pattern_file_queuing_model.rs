use super::packet_queue::PacketQueue;
use super::QueuingModel;
use crate::nfqueue_wrapper::{NfqPacket, Verdict};
use csv::{Error, ReaderBuilder, Trim};
use std::fmt::Display;
use std::path::Path;
use std::time::Duration;

#[derive(Clone)]
pub struct PacketInfo {
    pub delay: Duration,
    pub drop: bool,
}

pub struct PatternFileQueuingModel {
    packet_info: Vec<PacketInfo>,
    is_first_packet: bool,
    curr_packet_no: usize,
    queue: PacketQueue,
}

impl PatternFileQueuingModel {
    pub fn parse_packet_info<P>(csv_path: P) -> Result<Vec<PacketInfo>, Error>
    where
        P: AsRef<Path>,
    {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .trim(Trim::All)
            .from_path(csv_path)?;

        rdr.deserialize::<(u64, u32)>()
            .map(|result| {
                result.map(|(ms, drop)| PacketInfo {
                    delay: Duration::from_millis(ms),
                    drop: drop != 0,
                })
            })
            .collect()
    }

    pub fn new(packet_info: &[PacketInfo]) -> PatternFileQueuingModel {
        PatternFileQueuingModel {
            packet_info: packet_info.to_vec(),
            is_first_packet: true,
            curr_packet_no: 0,
            queue: PacketQueue::new(),
        }
    }

    fn drop_packet(&mut self) -> bool {
        if self.is_first_packet {
            self.is_first_packet = false;
        } else {
            self.curr_packet_no += 1;
            if self.curr_packet_no == self.packet_info.len() {
                self.curr_packet_no = 0;
            }
        }
        self.packet_info[self.curr_packet_no].drop
    }

    fn get_send_time(&mut self, receive_time: Duration) -> Duration {
        receive_time + self.packet_info[self.curr_packet_no].delay
    }
}

impl QueuingModel for PatternFileQueuingModel {
    fn enqueue(&mut self, packet: NfqPacket, time_now: Duration) {
        if !self.drop_packet() {
            let send_time = self.get_send_time(time_now);
            self.queue.push(packet, send_time);
        } else {
            packet.set_verdict(Verdict::Drop);
        }
    }

    fn dequeue(&mut self, time_now: Duration) -> Vec<NfqPacket> {
        self.queue.pop(time_now)
    }
}

impl Display for PatternFileQueuingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "csv file degrader model with {} packet patterns",
            self.packet_info.len()
        )
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn run_degrader_model() {
        let packet_info =
            PatternFileQueuingModel::parse_packet_info("examples/100-150ms_delay_30%_loss.csv")
                .expect("Error reading csv file");
        let mut model = PatternFileQueuingModel::new(&packet_info);
        for info in packet_info.iter().cycle().take(23) {
            if model.drop_packet() {
                assert!(info.drop)
            } else {
                let delay = model.get_send_time(Duration::from_millis(0));
                assert_eq!(delay, info.delay);
            }
        }
    }

    #[test]
    fn read_csv_file() {
        let packet_info =
            PatternFileQueuingModel::parse_packet_info("examples/100-150ms_delay_30%_loss.csv")
                .expect("Error reading csv file");
        let delay_range = Duration::from_millis(100)..=Duration::from_millis(150);
        let mut drop_counter = 0;
        for info in packet_info {
            if info.drop {
                drop_counter += 1;
            } else {
                assert!(delay_range.contains(&info.delay));
            }
        }
        assert_eq!(drop_counter, 3)
    }
}
